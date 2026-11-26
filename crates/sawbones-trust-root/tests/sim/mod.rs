//! A simulated board for host tests (#132): ideal rail feedback, a record
//! store that can fail or be corrupted, a scripted random number generator
//! and a supervisor that can answer the heartbeat. Every call the loop makes
//! is logged in order, so tests can check what happened before what.

#![allow(dead_code)]

use std::num::NonZeroU64;

use sawbones_interlock::{FaultRecord, HeartbeatReply, Port, Tick, RAIL_B_PORTS};
use sawbones_trust_root::{
    record, Board, LinkRequest, Pins, RecordWords, RecordWriteFailed, TickReport,
};

/// One call the loop made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Drive { rail_a: bool, rail_b: Option<Port> },
    TogglePump,
    ReadRecord,
    WriteRecord(RecordWords),
    Nonce,
    Sample,
    LinkRequest,
    Report,
    FeedWatchdog,
}

#[derive(Debug)]
pub struct SimBoard {
    pub log: Vec<Op>,
    /// What the non-volatile store holds.
    pub stored: Option<RecordWords>,
    /// Writes fail while this is set.
    pub fail_writes: bool,
    /// Writes "succeed" but store this instead (a store that lies).
    pub corrupt_writes: bool,
    /// What the random number generator returns.
    pub nonce: Option<NonZeroU64>,
    pub now: Tick,
    /// The switch and button positions. Rail feedback is computed from what
    /// is driven, unless a stuck rail is set.
    pub estop_pressed: bool,
    pub arm_switch_on: bool,
    pub key_switch_on: bool,
    pub panel_button_pressed: bool,
    pub firmware_jumper_fitted: bool,
    pub stuck_rail_a: Option<bool>,
    /// The supervisor answers each challenge on the next tick.
    pub auto_reply: bool,
    pub replies: Vec<HeartbeatReply>,
    pub requests: Vec<LinkRequest>,
    pub rail_a: bool,
    pub rail_b: Option<Port>,
    pub pump_level: bool,
    pub last_report: Option<TickReport>,
    /// Broken rules seen by the board itself, in order.
    pub violations: Vec<&'static str>,
    answered: Option<u64>,
    wrote_since_drive: bool,
    toggles_this_tick: u32,
    sampled_this_tick: bool,
}

impl SimBoard {
    pub fn new() -> Self {
        SimBoard {
            log: Vec::new(),
            stored: None,
            fail_writes: false,
            corrupt_writes: false,
            nonce: NonZeroU64::new(0x5eed_1234_abcd_0001),
            now: 0,
            estop_pressed: false,
            arm_switch_on: false,
            key_switch_on: false,
            panel_button_pressed: false,
            firmware_jumper_fitted: false,
            stuck_rail_a: None,
            auto_reply: true,
            replies: Vec::new(),
            requests: Vec::new(),
            rail_a: false,
            rail_b: None,
            pump_level: false,
            last_report: None,
            violations: Vec::new(),
            answered: None,
            wrote_since_drive: false,
            toggles_this_tick: 0,
            sampled_this_tick: false,
        }
    }

    pub fn with_record(record: Option<RecordWords>) -> Self {
        SimBoard {
            stored: record,
            ..SimBoard::new()
        }
    }

    pub fn pump_toggles(&self) -> usize {
        self.log.iter().filter(|op| **op == Op::TogglePump).count()
    }
}

impl Board for SimBoard {
    fn drive_rails(&mut self, rail_a: bool, rail_b: Option<Port>) {
        self.log.push(Op::Drive { rail_a, rail_b });
        let turns_on = (rail_a && !self.rail_a) || (rail_b.is_some() && rail_b != self.rail_b);
        if turns_on {
            if !self.wrote_since_drive {
                self.violations
                    .push("an output turned on with no record write before it");
            }
            if self.stored.and_then(|w| record::decode(&w)) != Some(FaultRecord::Armed) {
                self.violations
                    .push("an output turned on without a durable Armed record");
            }
        }
        self.wrote_since_drive = false;
        self.rail_a = rail_a;
        self.rail_b = rail_b;
    }

    fn toggle_charge_pump(&mut self) {
        self.log.push(Op::TogglePump);
        self.toggles_this_tick += 1;
        if !self.sampled_this_tick {
            self.violations
                .push("the charge pump was toggled outside a loop pass");
        }
        if !self.rail_a && self.rail_b.is_none() {
            self.violations
                .push("the charge pump was toggled with every rail off");
        }
        self.pump_level = !self.pump_level;
    }

    fn read_record(&mut self) -> Option<RecordWords> {
        self.log.push(Op::ReadRecord);
        self.stored
    }

    fn write_record(&mut self, words: &RecordWords) -> Result<(), RecordWriteFailed> {
        self.log.push(Op::WriteRecord(*words));
        self.wrote_since_drive = true;
        if self.fail_writes {
            return Err(RecordWriteFailed);
        }
        if self.corrupt_writes {
            let mut torn = *words;
            torn[3] ^= 1;
            self.stored = Some(torn);
        } else {
            self.stored = Some(*words);
        }
        Ok(())
    }

    fn hardware_nonce(&mut self) -> Option<NonZeroU64> {
        self.log.push(Op::Nonce);
        self.nonce
    }

    fn now(&mut self) -> Tick {
        self.now
    }

    fn sample(&mut self) -> Pins {
        self.log.push(Op::Sample);
        self.sampled_this_tick = true;
        self.toggles_this_tick = 0;
        let mut rail_b_sensed = [false; RAIL_B_PORTS];
        if let Some(Port(p)) = self.rail_b {
            rail_b_sensed[usize::from(p)] = true;
        }
        Pins {
            estop_pressed: self.estop_pressed,
            arm_switch_on: self.arm_switch_on,
            key_switch_on: self.key_switch_on,
            panel_button_pressed: self.panel_button_pressed,
            rail_a_sensed: self.stuck_rail_a.unwrap_or(self.rail_a),
            rail_b_sensed,
            firmware_jumper_fitted: self.firmware_jumper_fitted,
        }
    }

    fn heartbeat_reply(&mut self) -> Option<HeartbeatReply> {
        if !self.replies.is_empty() {
            return Some(self.replies.remove(0));
        }
        if !self.auto_reply {
            return None;
        }
        let challenge = self.last_report?.outputs.challenge?;
        if self.answered == Some(challenge.0) {
            return None;
        }
        self.answered = Some(challenge.0);
        Some(challenge.reply())
    }

    fn link_request(&mut self) -> Option<LinkRequest> {
        self.log.push(Op::LinkRequest);
        if self.requests.is_empty() {
            None
        } else {
            Some(self.requests.remove(0))
        }
    }

    fn report(&mut self, report: &TickReport) {
        self.log.push(Op::Report);
        let on = self.rail_a || self.rail_b.is_some();
        if self.toggles_this_tick > 1 {
            self.violations
                .push("the charge pump was toggled more than once in a pass");
        }
        if on != (self.toggles_this_tick == 1) {
            self.violations
                .push("the charge pump was not fed exactly when a rail was driven on");
        }
        if report.rail_a_driven != self.rail_a || report.rail_b_driven != self.rail_b {
            self.violations
                .push("the report disagrees with the driven rails");
        }
        if (self.rail_a && !report.outputs.rail_a)
            || (self.rail_b.is_some() && self.rail_b != report.outputs.rail_b)
        {
            self.violations
                .push("a rail is driven on that step did not return on");
        }
        self.last_report = Some(*report);
    }

    fn feed_watchdog(&mut self) {
        self.log.push(Op::FeedWatchdog);
        self.sampled_this_tick = false;
    }
}
