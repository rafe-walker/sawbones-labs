//! Table tests: one table per rule. Each row is a script of ticks and what the
//! outputs must be after the last one.
//!
//! The harness plays the firmware around the interlock: it keeps the fault
//! record as non-volatile memory would, and it plays a supervisor that
//! answers each heartbeat challenge once, on the tick after it appears,
//! unless the script says otherwise.

use sawbones_interlock::{
    step, ActionDigest, ClearSource, Event, FaultReason, FaultRecord, HazardClass,
    HeartbeatChallenge, HeartbeatReply, HeartbeatWatch, Inputs, LeaseTarget, Mode, Outputs,
    PanelState, PanelToken, Port, Rail, Refusal, State, HEARTBEAT_PERIOD_TICKS, LEASE_TTL_TICKS,
    OUTPUT_SETTLE_TICKS, RAIL_B_PORTS, REQUEST_TTL_TICKS,
};

/// What the simulated supervisor sends on a tick.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Beat {
    /// Answers the outstanding challenge if it has not answered it yet.
    Answer,
    /// Sends nothing.
    Silent,
    /// Sends a reply that answers nothing.
    Wrong,
    /// Sends the last reply it sent again.
    Repeat,
    /// Sends the reply to the challenge before the outstanding one.
    Stale,
}

/// How a tick restarts the controller, if it does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Boot {
    /// No restart.
    No,
    /// A reset that keeps RAM: the state is passed in, with the record.
    Reset,
    /// A power loss: RAM is gone, so the state passed in is `POWER_ON`, and
    /// the record is what non-volatile memory holds.
    PowerLoss,
    /// A power loss with this record read back instead (`None`: unreadable).
    PowerLossWith(Option<FaultRecord>),
}

/// One tick of a script.
#[derive(Clone, Copy)]
struct Tick {
    inputs: Inputs,
    event: Option<Event>,
    /// How far the clock moves before this tick.
    advance: u64,
    /// The rail feedback for this tick. `None` is healthy hardware: each rail
    /// is sensed exactly as the previous tick drove it, and off after a
    /// restart.
    sensed: Option<(bool, [bool; RAIL_B_PORTS])>,
    beat: Beat,
    boot: Boot,
}

const NO_RAIL_B: [bool; RAIL_B_PORTS] = [false; RAIL_B_PORTS];

/// Rail B sensed live on one port only.
const fn b_live(port: u8) -> [bool; RAIL_B_PORTS] {
    let mut b = NO_RAIL_B;
    b[port as usize] = true;
    b
}

/// A healthy, idle sample: nothing pressed, every switch off.
const IDLE: Tick = Tick {
    inputs: Inputs {
        estop_pressed: false,
        arm_switch_on: false,
        key_switch_on: false,
        heartbeat: None,
        panel_button_pressed: false,
        rail_a_sensed: false,
        rail_b_sensed: NO_RAIL_B,
        firmware_jumper_fitted: false,
        now: 0,
    },
    event: None,
    advance: 1,
    sensed: None,
    beat: Beat::Answer,
    boot: Boot::No,
};

const PANEL_CLEAR: Event = Event::ClearFault {
    source: ClearSource::Panel(PanelToken::from_panel_button()),
};
const SUPERVISOR_CLEAR: Event = Event::ClearFault {
    source: ClearSource::Supervisor,
};

impl Tick {
    fn key(mut self) -> Self {
        self.inputs.key_switch_on = true;
        self
    }
    fn button(mut self) -> Self {
        self.inputs.panel_button_pressed = true;
        self
    }
    fn estop(mut self) -> Self {
        self.inputs.estop_pressed = true;
        self
    }
    /// The supervisor sends a wrong reply on this tick.
    fn no_heartbeat(self) -> Self {
        self.beat(Beat::Wrong)
    }
    fn beat(mut self, beat: Beat) -> Self {
        self.beat = beat;
        self
    }
    fn event(mut self, event: Event) -> Self {
        self.event = Some(event);
        self
    }
    /// A press of the panel button that the panel driver turns into a clear.
    fn panel_clear(self) -> Self {
        self.button().event(PANEL_CLEAR)
    }
    /// A reset that keeps RAM, with the record read back from memory.
    fn restart(mut self) -> Self {
        self.boot = Boot::Reset;
        self
    }
    fn power_loss(mut self) -> Self {
        self.boot = Boot::PowerLoss;
        self
    }
    fn power_loss_with(mut self, record: Option<FaultRecord>) -> Self {
        self.boot = Boot::PowerLossWith(record);
        self
    }
    fn after(mut self, ticks: u64) -> Self {
        self.advance = ticks;
        self
    }
    fn jumper(mut self) -> Self {
        self.inputs.firmware_jumper_fitted = true;
        self
    }
    /// Overrides the rail feedback on this tick.
    fn sensed(mut self, rail_a: bool, rail_b: [bool; RAIL_B_PORTS]) -> Self {
        self.sensed = Some((rail_a, rail_b));
        self
    }
}

/// What the last tick of a script must produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Want {
    rail_a: bool,
    rail_b: Option<Port>,
    firmware_update_allowed: bool,
    panel: PanelState,
    fault: Option<FaultReason>,
    refusal: Option<Refusal>,
}

/// `rail_b` true means rail B on for the default target's port.
const fn want(rail_a: bool, rail_b: bool, panel: PanelState) -> Want {
    Want {
        rail_a,
        rail_b: if rail_b { Some(PORT0) } else { None },
        firmware_update_allowed: false,
        panel,
        fault: None,
        refusal: None,
    }
}

const fn fault(reason: FaultReason) -> Want {
    Want {
        rail_a: false,
        rail_b: None,
        firmware_update_allowed: false,
        panel: PanelState::Fault,
        fault: Some(reason),
        refusal: None,
    }
}

const UNKNOWN: Want = fault(FaultReason::FaultStateUnknown);

const FIRMWARE: Want = Want {
    rail_a: false,
    rail_b: None,
    firmware_update_allowed: true,
    panel: PanelState::FirmwareUpdate,
    fault: None,
    refusal: None,
};

const fn mismatch(rail: Rail, sensed_live: bool) -> Want {
    fault(FaultReason::OutputMismatch { rail, sensed_live })
}

impl Want {
    const fn refused(mut self, why: Refusal) -> Self {
        self.refusal = Some(why);
        self
    }
    const fn on_port(mut self, port: Port) -> Self {
        self.rail_b = Some(port);
        self
    }
}

const ALL_OFF_AWAIT: Want = want(false, false, PanelState::AwaitingArmOff);
const ALL_OFF_DISARMED: Want = want(false, false, PanelState::Disarmed);
const RAIL_A_ONLY: Want = want(true, false, PanelState::Armed);

const PORT0: Port = Port(0);
const DIGEST1: ActionDigest = ActionDigest([1; 32]);
const DIGEST2: ActionDigest = ActionDigest([2; 32]);
/// The default target: one action on port 0.
const T0: LeaseTarget = LeaseTarget {
    port: PORT0,
    action_digest: DIGEST1,
};

const fn lease3(target: LeaseTarget) -> Event {
    Event::RequestLease {
        class: HazardClass::Class3,
        target,
    }
}
const fn used(target: LeaseTarget) -> Event {
    Event::LeaseUsed { target }
}
const LEASE3: Event = lease3(T0);

/// The firmware and supervisor around the interlock.
struct Bench {
    state: State,
    now: u64,
    last: Outputs,
    /// The fault record in non-volatile memory.
    record: FaultRecord,
    /// The next boot nonce.
    nonce: u64,
    /// The challenge the supervisor last answered, the reply it last sent,
    /// and the challenge before the outstanding one.
    answered: Option<HeartbeatChallenge>,
    sent: Option<HeartbeatReply>,
    previous: Option<HeartbeatChallenge>,
    /// The inputs of the last tick, for the quiet ticks of a gap.
    inputs: Inputs,
}

impl Bench {
    /// A bench that has just booted cleanly at tick 0 with the arm switch on,
    /// so nothing is armed and the switch must be seen off. No reply yet.
    fn booted() -> Self {
        let boot = Inputs {
            arm_switch_on: true,
            ..IDLE.inputs
        };
        let restart = Event::Restart {
            record: Some(FaultRecord::Clear),
            nonce: 1,
        };
        let (state, last) = step(State::POWER_ON, boot, Some(restart));
        assert_eq!(state.mode, Mode::AwaitArmOff);
        Self::from(state, last, 0)
    }

    /// A bench at power-on with no restart event fed at all.
    fn unbooted() -> Self {
        Self::from(State::POWER_ON, State::POWER_ON.outputs(), 0)
    }

    fn from(state: State, last: Outputs, now: u64) -> Self {
        Bench {
            state,
            now,
            last,
            record: state.fault_record(),
            nonce: 2,
            answered: None,
            sent: None,
            previous: None,
            inputs: IDLE.inputs,
        }
    }

    /// Runs one script tick. A tick that moves the clock by more than one
    /// is preceded by quiet ticks, one per tick of the gap, with the previous
    /// inputs and no event, as the firmware loop samples every tick. The
    /// supervisor keeps answering through them unless this tick is silent.
    fn tick(&mut self, tick: &Tick) -> Outputs {
        for _ in 1..tick.advance {
            let quiet = Tick {
                inputs: self.inputs,
                event: None,
                advance: 1,
                sensed: None,
                beat: if tick.beat == Beat::Silent {
                    Beat::Silent
                } else {
                    Beat::Answer
                },
                boot: Boot::No,
            };
            self.one(&quiet);
        }
        self.one(&Tick {
            advance: tick.advance.min(1),
            ..*tick
        })
    }

    fn one(&mut self, tick: &Tick) -> Outputs {
        self.now += tick.advance;
        let now = self.now;
        let restarting = tick.boot != Boot::No;
        let (rail_a_sensed, rail_b_sensed) = match tick.sensed {
            Some(sensed) => sensed,
            None if restarting => (false, NO_RAIL_B),
            None => {
                let mut b = NO_RAIL_B;
                if let Some(port) = self.last.rail_b {
                    b[usize::from(port.0)] = true;
                }
                (self.last.rail_a, b)
            }
        };

        // The supervisor answers what the previous tick showed it. Across a
        // restart there is nothing in flight.
        let outstanding = self.last.challenge;
        let heartbeat = match tick.beat {
            _ if restarting => None,
            Beat::Silent => None,
            // Nothing shown (the heartbeat has not started): nothing to answer.
            Beat::Answer if outstanding.is_none() || self.answered == outstanding => None,
            Beat::Answer => {
                self.answered = outstanding;
                outstanding.map(HeartbeatChallenge::reply)
            }
            Beat::Wrong => Some(HeartbeatReply(outstanding.map_or(1, |c| c.reply().0 ^ 1))),
            Beat::Repeat => Some(self.sent.expect("nothing sent yet to repeat")),
            Beat::Stale => Some(self.previous.expect("no earlier challenge").reply()),
        };
        if heartbeat.is_some() {
            self.sent = heartbeat;
        }

        let mut state = self.state;
        let event = match tick.boot {
            Boot::No => tick.event,
            Boot::Reset | Boot::PowerLoss | Boot::PowerLossWith(_) => {
                let record = match tick.boot {
                    Boot::PowerLossWith(record) => record,
                    _ => Some(self.record),
                };
                if tick.boot != Boot::Reset {
                    state = State::POWER_ON;
                }
                self.nonce += 1;
                Some(Event::Restart {
                    record,
                    nonce: self.nonce,
                })
            }
        };
        let inputs = Inputs {
            now,
            rail_a_sensed,
            rail_b_sensed,
            heartbeat,
            ..tick.inputs
        };
        let (next, outputs) = step(state, inputs, event);
        if outputs.challenge != outstanding {
            self.previous = outstanding;
        }
        self.inputs = Inputs {
            heartbeat: None,
            ..tick.inputs
        };
        self.state = next;
        self.last = outputs;
        self.record = next.fault_record();
        outputs
    }
}

/// Runs a script from a clean boot and returns the last tick's outputs.
fn run(script: &[Tick]) -> Outputs {
    let mut bench = Bench::booted();
    for tick in script {
        bench.tick(tick);
    }
    bench.last
}

fn trim(got: Outputs) -> Want {
    Want {
        rail_a: got.rail_a,
        rail_b: got.rail_b,
        firmware_update_allowed: got.firmware_update_allowed,
        panel: got.panel,
        fault: got.fault,
        refusal: got.refusal,
    }
}

fn check(table: &[(&str, &[Tick], Want)]) {
    for (name, script, want) in table {
        assert_eq!(&trim(run(script)), want, "row: {name}");
    }
}

/// The shortest way to rail A: switch seen off, then on.
const ARMED: [Tick; 2] = [IDLE, ARM];
const ARM: Tick = Tick {
    inputs: Inputs {
        arm_switch_on: true,
        ..IDLE.inputs
    },
    ..IDLE
};

fn script(prefix: &[Tick], rest: &[Tick]) -> Vec<Tick> {
    [prefix, rest].concat()
}

#[test]
fn power_on_drives_nothing() {
    let outputs = State::POWER_ON.outputs();
    assert!(!outputs.rail_a && outputs.rail_b.is_none());
    assert!(!outputs.firmware_update_allowed);
    assert_eq!(outputs.panel, PanelState::Fault);
    assert_eq!(outputs.fault, Some(FaultReason::FaultStateUnknown));
    assert_eq!(State::default(), State::POWER_ON);
}

#[test]
fn arm_sequence() {
    check(&[
        (
            "switch left on from power-on does not arm",
            &[ARM, ARM, ARM],
            ALL_OFF_AWAIT,
        ),
        ("switch off alone does not arm", &[IDLE], ALL_OFF_DISARMED),
        ("off then on arms rail A", &ARMED, RAIL_A_ONLY),
        (
            "left on, then off, then on arms",
            &[ARM, IDLE, ARM],
            RAIL_A_ONLY,
        ),
        (
            "switch off drops rail A",
            &[IDLE, ARM, IDLE],
            ALL_OFF_DISARMED,
        ),
        (
            "off and on again re-arms in the same session",
            &[IDLE, ARM, IDLE, ARM],
            RAIL_A_ONLY,
        ),
        (
            "a request cannot arm with the switch off",
            &[IDLE, IDLE.event(Event::RequestArm)],
            ALL_OFF_DISARMED.refused(Refusal::NotArmed),
        ),
        (
            "a request cannot stand in for seeing the switch off",
            &[ARM.event(Event::RequestArm), ARM.event(Event::RequestArm)],
            ALL_OFF_AWAIT.refused(Refusal::NotArmed),
        ),
        (
            "a request while armed is not refused",
            &[IDLE, ARM, ARM.event(Event::RequestArm)],
            RAIL_A_ONLY,
        ),
        (
            "the key and the button alone give nothing",
            &[IDLE.key().button(), ARM.key().button()],
            RAIL_A_ONLY,
        ),
    ]);
}

#[test]
fn restart_turns_everything_off() {
    check(&[
        (
            "a clean restart with the switch left on",
            &[IDLE, ARM, IDLE, ARM.restart()],
            ALL_OFF_AWAIT,
        ),
        (
            "still off on later ticks with the switch left on",
            &[IDLE, ARM, IDLE, ARM.restart(), ARM, ARM],
            ALL_OFF_AWAIT,
        ),
        (
            "a clean restart with the switch off still needs off then on",
            &[IDLE, IDLE.restart()],
            ALL_OFF_DISARMED,
        ),
        (
            "fresh sequence after a clean restart re-arms",
            &[IDLE, ARM.restart(), IDLE, ARM],
            RAIL_A_ONLY,
        ),
        (
            "a restart while armed comes up with the fault state unknown",
            &[IDLE, ARM, ARM.restart()],
            UNKNOWN,
        ),
        (
            "and the arm switch alone does not bring the rail back",
            &[IDLE, ARM, ARM.restart(), IDLE, ARM],
            UNKNOWN,
        ),
        (
            "a panel clear, then off and on, does",
            &[IDLE, ARM, ARM.restart(), IDLE.panel_clear(), ARM],
            RAIL_A_ONLY,
        ),
        (
            "restart does not clear a latched fault",
            &[IDLE.estop(), IDLE.restart()],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "nor does a power loss",
            &[IDLE.estop(), IDLE.power_loss(), IDLE, ARM],
            fault(FaultReason::EmergencyStop),
        ),
    ]);
}

#[test]
fn emergency_stop_latches() {
    let stopped = fault(FaultReason::EmergencyStop);
    check(&[
        ("pressed while armed", &[IDLE, ARM, ARM.estop()], stopped),
        (
            "stays latched after release",
            &[IDLE, ARM, ARM.estop(), ARM, IDLE, ARM],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "pressed before any arming",
            &[ARM.estop()],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "outranks a clean restart on the same tick",
            &[IDLE, ARM, IDLE, ARM.estop().restart()],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "a restart while armed is unknown first, and the stop keeps it latched",
            &[IDLE, ARM, ARM.estop().restart(), IDLE.estop().panel_clear()],
            UNKNOWN.refused(Refusal::EmergencyStopPressed),
        ),
        (
            "outranks a lost heartbeat on the same tick",
            &[IDLE, ARM, ARM.estop().no_heartbeat()],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "a request on the tick it is pressed is refused",
            &[IDLE, ARM, ARM.estop().event(Event::RequestArm)],
            fault(FaultReason::EmergencyStop).refused(Refusal::Faulted),
        ),
    ]);
}

#[test]
fn lost_heartbeat_latches() {
    check(&[
        (
            "lost while armed",
            &[IDLE, ARM, ARM.no_heartbeat()],
            fault(FaultReason::HeartbeatLost),
        ),
        (
            "stays latched after it comes back",
            &[IDLE, ARM, ARM.no_heartbeat(), ARM, IDLE, ARM],
            fault(FaultReason::HeartbeatLost),
        ),
        (
            "the first reason is kept",
            &[IDLE, ARM, ARM.no_heartbeat(), ARM.estop()],
            fault(FaultReason::HeartbeatLost),
        ),
        (
            "missing from power-on",
            &[IDLE.no_heartbeat()],
            fault(FaultReason::HeartbeatLost),
        ),
    ]);
}

#[test]
fn clearing_a_fault() {
    let stopped = fault(FaultReason::EmergencyStop);
    check(&[
        (
            "clears with the stop released and the switch off",
            &[IDLE, ARM, ARM.estop(), IDLE.panel_clear()],
            ALL_OFF_DISARMED,
        ),
        (
            "refused while the stop is pressed",
            &[IDLE, ARM, ARM.estop(), IDLE.estop().panel_clear()],
            fault(FaultReason::EmergencyStop).refused(Refusal::EmergencyStopPressed),
        ),
        (
            "refused while the arm switch is on",
            &[IDLE, ARM, ARM.estop(), ARM.panel_clear()],
            fault(FaultReason::EmergencyStop).refused(Refusal::ArmSwitchOn),
        ),
        (
            "refused while the heartbeat is still missing",
            &[
                IDLE,
                ARM,
                ARM.no_heartbeat(),
                IDLE.no_heartbeat().panel_clear(),
            ],
            fault(FaultReason::HeartbeatLost).refused(Refusal::HeartbeatUnhealthy),
        ),
        (
            "no input clears it without the event",
            &[IDLE, ARM, ARM.estop(), IDLE, IDLE, IDLE],
            stopped,
        ),
        (
            "after a clear, the switch going on arms again",
            &[IDLE, ARM, ARM.estop(), IDLE.panel_clear(), ARM],
            RAIL_A_ONLY,
        ),
        (
            "other requests are refused while latched",
            &[IDLE, ARM, ARM.estop(), ARM.event(Event::RequestArm)],
            fault(FaultReason::EmergencyStop).refused(Refusal::Faulted),
        ),
        (
            "a clear with nothing latched is a no-op",
            &[IDLE, ARM, ARM.panel_clear()],
            RAIL_A_ONLY,
        ),
        (
            "a clear from the supervisor is refused",
            &[
                IDLE,
                ARM,
                ARM.estop(),
                IDLE.button().event(SUPERVISOR_CLEAR),
            ],
            stopped.refused(Refusal::ClearNotFromPanel),
        ),
        (
            "a panel clear with the button not sensed is refused",
            &[IDLE, ARM, ARM.estop(), IDLE.event(PANEL_CLEAR)],
            stopped.refused(Refusal::PanelButtonNotPressed),
        ),
        (
            "after a bad reply, a clear waits for the next challenge's answer",
            &[IDLE, ARM, ARM.no_heartbeat(), IDLE.panel_clear()],
            fault(FaultReason::HeartbeatLost).refused(Refusal::HeartbeatUnhealthy),
        ),
        (
            "and succeeds once it is answered",
            &[
                IDLE,
                ARM,
                ARM.no_heartbeat(),
                IDLE.after(HEARTBEAT_PERIOD_TICKS),
                IDLE.panel_clear(),
            ],
            ALL_OFF_DISARMED,
        ),
    ]);
}

/// Armed with the key in: the starting point for every lease row.
const KEYED: Tick = Tick {
    inputs: Inputs {
        key_switch_on: true,
        ..ARM.inputs
    },
    ..ARM
};
const PENDING: Want = want(true, false, PanelState::ApprovalRequested);
const RAIL_B_ON: Want = want(true, true, PanelState::LeaseLive);

/// Armed, a class 3 request made, then granted by a press.
fn granted() -> Vec<Tick> {
    script(&ARMED, &[KEYED.event(LEASE3), KEYED.button()])
}

#[test]
fn lease_is_granted_only_by_a_fresh_button_press() {
    check(&[
        (
            "a request alone gives no rail B",
            &script(&ARMED, &[KEYED.event(LEASE3)]),
            PENDING,
        ),
        ("request then press grants", &granted(), RAIL_B_ON),
        (
            "a press with no request gives nothing",
            &script(&ARMED, &[KEYED.button(), KEYED, KEYED.button()]),
            RAIL_A_ONLY,
        ),
        (
            "a press before the request is not an approval of it",
            &script(&ARMED, &[KEYED.button(), KEYED.event(LEASE3)]),
            PENDING,
        ),
        (
            "a press on the tick the request arrives does not grant",
            &script(&ARMED, &[KEYED.button().event(LEASE3)]),
            PENDING,
        ),
        (
            "a button held down since before the request does not grant",
            &script(
                &ARMED,
                &[KEYED.button().event(LEASE3), KEYED.button(), KEYED.button()],
            ),
            PENDING,
        ),
        (
            "released and pressed again, it grants",
            &script(
                &ARMED,
                &[KEYED.button().event(LEASE3), KEYED, KEYED.button()],
            ),
            RAIL_B_ON,
        ),
        (
            "rail B stays on after the button is released",
            &script(&granted(), &[KEYED, KEYED]),
            RAIL_B_ON,
        ),
        (
            "a second request while one is pending is refused",
            &script(&ARMED, &[KEYED.event(LEASE3), KEYED.event(LEASE3)]),
            PENDING.refused(Refusal::LeaseBusy),
        ),
        (
            "a request while a lease is live is refused",
            &script(&granted(), &[KEYED.event(LEASE3)]),
            RAIL_B_ON.refused(Refusal::LeaseBusy),
        ),
        (
            "a request before arming is refused",
            &[IDLE.key().event(LEASE3)],
            ALL_OFF_DISARMED.refused(Refusal::NotArmed),
        ),
        (
            "a request on the tick of arming is refused and leaves nothing pending",
            &[IDLE, KEYED.event(LEASE3)],
            RAIL_A_ONLY.refused(Refusal::NotArmed),
        ),
    ]);
}

#[test]
fn lease_needs_the_key() {
    check(&[
        (
            "a request with the key off is refused",
            &script(&ARMED, &[ARM.event(LEASE3)]),
            RAIL_A_ONLY.refused(Refusal::KeyOff),
        ),
        (
            "the key going off withdraws a pending request",
            &script(&ARMED, &[KEYED.event(LEASE3), ARM, KEYED.button()]),
            RAIL_A_ONLY,
        ),
        (
            "a press with the key off grants nothing",
            &script(&ARMED, &[KEYED.event(LEASE3), ARM.button()]),
            RAIL_A_ONLY,
        ),
        (
            "the key going off drops rail B",
            &script(&granted(), &[ARM]),
            RAIL_A_ONLY,
        ),
        (
            "the key coming back does not bring rail B back",
            &script(&granted(), &[ARM, KEYED, KEYED.button()]),
            RAIL_A_ONLY,
        ),
    ]);
}

#[test]
fn lease_expires_after_a_fixed_number_of_ticks() {
    check(&[
        (
            "live on the last tick of the window",
            &script(&granted(), &[KEYED.after(LEASE_TTL_TICKS - 1)]),
            RAIL_B_ON,
        ),
        (
            "off on the tick the window ends",
            &script(&granted(), &[KEYED.after(LEASE_TTL_TICKS)]),
            RAIL_A_ONLY,
        ),
        (
            "holding the button does not extend it",
            &script(&granted(), &[KEYED.button().after(LEASE_TTL_TICKS)]),
            RAIL_A_ONLY,
        ),
        (
            "a press after expiry is not a new approval",
            &script(&granted(), &[KEYED.after(LEASE_TTL_TICKS), KEYED.button()]),
            RAIL_A_ONLY,
        ),
        (
            "a pending request can be granted on the last tick of its window",
            &script(
                &ARMED,
                &[
                    KEYED.event(LEASE3),
                    KEYED.button().after(REQUEST_TTL_TICKS - 1),
                ],
            ),
            RAIL_B_ON,
        ),
        (
            "a stale request is dropped, so a late press grants nothing",
            &script(
                &ARMED,
                &[KEYED.event(LEASE3), KEYED.button().after(REQUEST_TTL_TICKS)],
            ),
            RAIL_A_ONLY,
        ),
    ]);
}

#[test]
fn lease_is_spent_after_one_use() {
    let used = used(T0);
    check(&[
        (
            "use drops rail B",
            &script(&granted(), &[KEYED.event(used)]),
            RAIL_A_ONLY,
        ),
        (
            "a second use is refused",
            &script(&granted(), &[KEYED.event(used), KEYED.event(used)]),
            RAIL_A_ONLY.refused(Refusal::NoLiveLease),
        ),
        (
            "a replayed press after use grants nothing",
            &script(&granted(), &[KEYED.event(used), KEYED, KEYED.button()]),
            RAIL_A_ONLY,
        ),
        (
            "a use with nothing granted is refused",
            &script(&ARMED, &[KEYED.event(LEASE3), KEYED.event(used)]),
            PENDING.refused(Refusal::NoLiveLease),
        ),
        (
            "a new request and a new press grant a new lease",
            &script(
                &granted(),
                &[KEYED.event(used), KEYED.event(LEASE3), KEYED.button()],
            ),
            RAIL_B_ON,
        ),
    ]);
}

#[test]
fn lease_does_not_survive_disarm_restart_or_fault() {
    check(&[
        (
            "arm switch off drops both rails",
            &script(&granted(), &[IDLE.key()]),
            ALL_OFF_DISARMED,
        ),
        (
            "re-arming does not bring rail B back",
            &script(&granted(), &[IDLE.key(), KEYED, KEYED.button()]),
            RAIL_A_ONLY,
        ),
        (
            "restart drops both rails",
            &script(&granted(), &[KEYED.restart()]),
            UNKNOWN,
        ),
        (
            "emergency stop drops both rails",
            &script(&granted(), &[KEYED.estop()]),
            fault(FaultReason::EmergencyStop),
        ),
        (
            "lost heartbeat drops both rails",
            &script(&granted(), &[KEYED.no_heartbeat()]),
            fault(FaultReason::HeartbeatLost),
        ),
        (
            "after a fault is cleared and the rail re-armed, the lease is gone",
            &script(
                &granted(),
                &[
                    KEYED.estop(),
                    IDLE.key().panel_clear(),
                    KEYED,
                    KEYED.button(),
                ],
            ),
            RAIL_A_ONLY,
        ),
        (
            "a pending request does not survive a restart",
            &script(
                &ARMED,
                &[
                    KEYED.event(LEASE3),
                    KEYED.restart(),
                    IDLE.key().panel_clear(),
                    KEYED,
                    KEYED.button(),
                ],
            ),
            RAIL_A_ONLY,
        ),
    ]);
}

#[test]
fn a_clock_that_goes_backwards_drops_the_lease() {
    let at = |now| Inputs {
        now,
        ..KEYED.inputs
    };
    let restart = Event::Restart {
        record: Some(FaultRecord::Clear),
        nonce: 7,
    };
    let (state, _) = step(State::POWER_ON, at(9).with_arm(false), Some(restart));
    let reply = state.heartbeat.challenge().expect("started").reply();
    let (state, _) = step(state, at(10).with_arm(false).with_reply(reply), None);
    let (state, _) = step(state, at(11), None);
    let (state, _) = step(state, at(12).with_rail_a(), Some(LEASE3));
    let (state, outputs) = step(state, at(13).with_rail_a().with_button(), None);
    assert_eq!(outputs.rail_b, Some(PORT0));
    let healthy = Inputs {
        rail_b_sensed: b_live(0),
        ..at(12).with_rail_a()
    };
    let (_, outputs) = step(state, healthy, None);
    assert!(outputs.rail_a && outputs.rail_b.is_none());
    assert_eq!(outputs.fault, None);
    // Back before the outstanding challenge was issued: the heartbeat's
    // timing cannot be judged, so it counts as lost.
    let before_issue = Inputs {
        rail_b_sensed: b_live(0),
        ..at(state.heartbeat.issued_at - 1).with_rail_a()
    };
    let (_, outputs) = step(state, before_issue, None);
    assert!(!outputs.rail_a && outputs.rail_b.is_none());
    assert_eq!(outputs.fault, Some(FaultReason::HeartbeatLost));
}

#[test]
fn class_4_is_refused_in_every_state() {
    let class4 = Event::RequestLease {
        class: HazardClass::Class4,
        target: T0,
    };
    let never = Refusal::ClassNeverPermitted;
    check(&[
        (
            "awaiting the arm switch",
            &[ARM.event(class4)],
            ALL_OFF_AWAIT.refused(never),
        ),
        (
            "disarmed",
            &[IDLE.event(class4)],
            ALL_OFF_DISARMED.refused(never),
        ),
        (
            "armed",
            &script(&ARMED, &[KEYED.event(class4)]),
            RAIL_A_ONLY.refused(never),
        ),
        (
            "armed, and a press afterwards grants nothing",
            &script(&ARMED, &[KEYED.event(class4), KEYED.button()]),
            RAIL_A_ONLY,
        ),
        (
            "with a class 3 request pending",
            &script(&ARMED, &[KEYED.event(LEASE3), KEYED.event(class4)]),
            PENDING.refused(never),
        ),
        (
            "with a class 3 lease live",
            &script(&granted(), &[KEYED.event(class4)]),
            RAIL_B_ON.refused(never),
        ),
        (
            "with a fault latched",
            &[IDLE.estop(), IDLE.event(class4)],
            fault(FaultReason::EmergencyStop).refused(never),
        ),
    ]);
}

#[test]
fn classes_without_a_lease_are_told_so() {
    for class in [
        HazardClass::Class0,
        HazardClass::Class1,
        HazardClass::Class2,
    ] {
        let request = Event::RequestLease { class, target: T0 };
        let last = run(&script(&ARMED, &[KEYED.event(request), KEYED.button()]));
        assert!(last.rail_a && last.rail_b.is_none(), "{class:?}");
        let last = run(&script(&ARMED, &[KEYED.event(request)]));
        assert_eq!(last.refusal, Some(Refusal::NoLeaseForClass), "{class:?}");
    }
}

/// Small setters for the one test that drives `step` directly.
trait InputsExt {
    fn with_arm(self, on: bool) -> Self;
    fn with_button(self) -> Self;
    fn with_rail_a(self) -> Self;
    fn with_reply(self, reply: HeartbeatReply) -> Self;
}

impl InputsExt for Inputs {
    fn with_reply(mut self, reply: HeartbeatReply) -> Self {
        self.heartbeat = Some(reply);
        self
    }
    fn with_arm(mut self, on: bool) -> Self {
        self.arm_switch_on = on;
        self
    }
    fn with_button(mut self) -> Self {
        self.panel_button_pressed = true;
        self
    }
    fn with_rail_a(mut self) -> Self {
        self.rail_a_sensed = true;
        self
    }
}

/// Armed, a class 3 request for `target` made, then granted by a press.
fn granted_for(target: LeaseTarget) -> Vec<Tick> {
    script(&ARMED, &[KEYED.event(lease3(target)), KEYED.button()])
}

const T1: LeaseTarget = LeaseTarget {
    port: Port(1),
    action_digest: DIGEST1,
};
const T0_OTHER_ACTION: LeaseTarget = LeaseTarget {
    port: PORT0,
    action_digest: DIGEST2,
};
#[allow(clippy::cast_possible_truncation)]
const NO_SUCH_PORT: Port = Port(RAIL_B_PORTS as u8);

#[test]
fn lease_is_bound_to_one_port_and_one_action() {
    let on_1 = RAIL_B_ON.on_port(Port(1));
    check(&[
        (
            "a lease enables rail B on its own port only",
            &granted_for(T1),
            on_1,
        ),
        (
            "a use naming the same port and action spends it",
            &script(&granted_for(T1), &[KEYED.event(used(T1))]),
            RAIL_A_ONLY,
        ),
        (
            "a use naming another action is refused and ends the lease",
            &script(&granted(), &[KEYED.event(used(T0_OTHER_ACTION))]),
            RAIL_A_ONLY.refused(Refusal::LeaseMismatch),
        ),
        (
            "a use naming another port is refused and ends the lease",
            &script(&granted(), &[KEYED.event(used(T1))]),
            RAIL_A_ONLY.refused(Refusal::LeaseMismatch),
        ),
        (
            "after a mismatched use the right use finds nothing",
            &script(&granted(), &[KEYED.event(used(T1)), KEYED.event(used(T0))]),
            RAIL_A_ONLY.refused(Refusal::NoLiveLease),
        ),
        (
            "a request for another port while a lease is live is refused",
            &script(&granted(), &[KEYED.event(lease3(T1)), KEYED.button()]),
            RAIL_B_ON,
        ),
        (
            "and the refusal says busy",
            &script(&granted(), &[KEYED.event(lease3(T1))]),
            RAIL_B_ON.refused(Refusal::LeaseBusy),
        ),
        (
            "a press grants the target that was requested, not another",
            &script(
                &ARMED,
                &[
                    KEYED.event(lease3(T1)),
                    KEYED.event(lease3(T0)),
                    KEYED.button(),
                ],
            ),
            on_1,
        ),
        (
            "a request for a port with no rail B is refused",
            &script(
                &ARMED,
                &[KEYED.event(lease3(LeaseTarget {
                    port: NO_SUCH_PORT,
                    action_digest: DIGEST1,
                }))],
            ),
            RAIL_A_ONLY.refused(Refusal::UnknownPort),
        ),
        (
            "and leaves nothing for a press to grant",
            &script(
                &ARMED,
                &[
                    KEYED.event(lease3(LeaseTarget {
                        port: NO_SUCH_PORT,
                        action_digest: DIGEST1,
                    })),
                    KEYED.button(),
                ],
            ),
            RAIL_A_ONLY,
        ),
        (
            "an unknown port is refused before arming too",
            &[IDLE.key().event(lease3(LeaseTarget {
                port: Port(u8::MAX),
                action_digest: DIGEST1,
            }))],
            ALL_OFF_DISARMED.refused(Refusal::UnknownPort),
        ),
        (
            "a new lease after a spent one may be for another port",
            &script(
                &granted(),
                &[
                    KEYED.event(used(T0)),
                    KEYED.event(lease3(T1)),
                    KEYED.button(),
                ],
            ),
            on_1,
        ),
    ]);
}

#[test]
fn a_stuck_on_rail_drops_everything_and_latches() {
    let stuck_a = mismatch(Rail::A, true);
    check(&[
        (
            "rail A live while disarmed",
            &[IDLE, IDLE.sensed(true, NO_RAIL_B)],
            stuck_a,
        ),
        (
            "rail A live at power-on",
            &[IDLE.sensed(true, NO_RAIL_B)],
            stuck_a,
        ),
        (
            "rail A live on the tick of a clean restart",
            &[IDLE, ARM, IDLE, ARM.restart().sensed(true, NO_RAIL_B)],
            stuck_a,
        ),
        (
            "rail A live on the tick of a restart from armed stays faulted",
            &[IDLE, ARM, ARM.restart().sensed(true, NO_RAIL_B)],
            UNKNOWN,
        ),
        (
            "rail B live on a port with no lease, while armed",
            &script(&ARMED, &[KEYED.sensed(true, b_live(2))]),
            mismatch(Rail::B(Port(2)), true),
        ),
        (
            "rail B live on another port while a lease is live drops the lease",
            &script(
                &granted(),
                &[KEYED.sensed(true, [true, false, false, true])],
            ),
            mismatch(Rail::B(Port(3)), true),
        ),
        (
            "a rail has the tick it is commanded off to drop",
            &script(&granted(), &[KEYED.event(used(T0))]),
            RAIL_A_ONLY,
        ),
        (
            "but live on the tick after is stuck on",
            &script(
                &granted(),
                &[KEYED.event(used(T0)), KEYED.sensed(true, b_live(0))],
            ),
            mismatch(Rail::B(PORT0), true),
        ),
        (
            "rail A still live the tick after disarming is stuck on",
            &[IDLE, ARM, IDLE, IDLE.sensed(true, NO_RAIL_B)],
            stuck_a,
        ),
        (
            "the emergency stop outranks it",
            &[IDLE.estop().sensed(true, NO_RAIL_B)],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "rail A is judged before rail B",
            &[IDLE.sensed(true, b_live(1))],
            stuck_a,
        ),
        (
            "a clear is refused while a rail is still live",
            &[
                IDLE.sensed(true, NO_RAIL_B),
                IDLE.panel_clear().sensed(true, NO_RAIL_B),
            ],
            stuck_a.refused(Refusal::RailSensedLive),
        ),
        (
            "a clear is refused while rail B is live on any port",
            &[
                IDLE.sensed(false, b_live(3)),
                IDLE.panel_clear().sensed(false, b_live(3)),
            ],
            mismatch(Rail::B(Port(3)), true).refused(Refusal::RailSensedLive),
        ),
        (
            "a clear succeeds once every rail is dead",
            &[IDLE.sensed(true, NO_RAIL_B), IDLE.panel_clear()],
            ALL_OFF_DISARMED,
        ),
    ]);
}

#[test]
fn a_rail_dead_while_commanded_on_latches_after_the_settle_window() {
    let settling = OUTPUT_SETTLE_TICKS - 1;
    check(&[
        (
            "rail A dead inside its settle window is tolerated",
            &[IDLE, ARM, ARM.sensed(false, NO_RAIL_B).after(settling)],
            RAIL_A_ONLY,
        ),
        (
            "rail A dead at the end of the window latches",
            &[
                IDLE,
                ARM,
                ARM.sensed(false, NO_RAIL_B).after(OUTPUT_SETTLE_TICKS),
            ],
            mismatch(Rail::A, false),
        ),
        (
            "rail A dropping out mid-session latches",
            &[IDLE, ARM, ARM.after(50), ARM.sensed(false, NO_RAIL_B)],
            mismatch(Rail::A, false),
        ),
        (
            "rail B dead inside its window is tolerated",
            &script(&granted(), &[KEYED.sensed(true, NO_RAIL_B).after(settling)]),
            RAIL_B_ON,
        ),
        (
            "rail B dead at the end of its window latches and drops rail A",
            &script(
                &granted(),
                &[KEYED.sensed(true, NO_RAIL_B).after(OUTPUT_SETTLE_TICKS)],
            ),
            mismatch(Rail::B(PORT0), false),
        ),
        (
            "rail B live on the wrong port instead of the leased one",
            &script(
                &granted(),
                &[KEYED.sensed(true, b_live(1)).after(OUTPUT_SETTLE_TICKS)],
            ),
            mismatch(Rail::B(PORT0), false),
        ),
        (
            "a dead rail cannot be cleared into service: it is re-armed afresh",
            &[
                IDLE,
                ARM,
                ARM.sensed(false, NO_RAIL_B).after(OUTPUT_SETTLE_TICKS),
                IDLE.panel_clear(),
                ARM,
            ],
            RAIL_A_ONLY,
        ),
    ]);
}

#[test]
fn firmware_jumper() {
    let armed_jumper = fault(FaultReason::FirmwareJumperWhileArmed);
    check(&[
        (
            "fitted while disarmed allows an update",
            &[IDLE, IDLE.jumper()],
            FIRMWARE,
        ),
        (
            "fitted at power-on with the arm switch on allows an update",
            &[ARM.jumper()],
            FIRMWARE,
        ),
        (
            "fitted while armed latches a fault",
            &[IDLE, ARM, ARM.jumper()],
            armed_jumper,
        ),
        (
            "fitted while a lease is live latches a fault",
            &script(&granted(), &[KEYED.jumper()]),
            armed_jumper,
        ),
        (
            "fitted on the tick the arm switch goes off still counts as armed",
            &[IDLE, ARM, IDLE.jumper()],
            armed_jumper,
        ),
        (
            "fitted on the tick the arm switch goes on does not arm",
            &[IDLE, ARM.jumper()],
            FIRMWARE,
        ),
        (
            "the arm switch cannot arm during an update",
            &[IDLE, IDLE.jumper(), ARM.jumper(), ARM.jumper()],
            FIRMWARE,
        ),
        (
            "a request to arm is refused during an update",
            &[IDLE, IDLE.jumper(), ARM.jumper().event(Event::RequestArm)],
            FIRMWARE.refused(Refusal::FirmwareJumperFitted),
        ),
        (
            "a lease request is refused during an update",
            &[IDLE, IDLE.jumper().key().event(LEASE3)],
            FIRMWARE.refused(Refusal::FirmwareJumperFitted),
        ),
        (
            "removing it with the arm switch on does not arm",
            &[IDLE, IDLE.jumper(), ARM.jumper(), ARM],
            ALL_OFF_AWAIT,
        ),
        (
            "removing it needs a fresh arm sequence",
            &[IDLE, IDLE.jumper(), ARM.jumper(), ARM, IDLE, ARM],
            RAIL_A_ONLY,
        ),
        (
            "removing it with the arm switch off disarms",
            &[IDLE, IDLE.jumper(), IDLE],
            ALL_OFF_DISARMED,
        ),
        (
            "no update with a fault latched",
            &[IDLE.estop(), IDLE.jumper()],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "the emergency stop during an update latches",
            &[IDLE, IDLE.jumper(), IDLE.jumper().estop()],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "a stuck-on rail during an update latches",
            &[IDLE, IDLE.jumper(), IDLE.jumper().sensed(true, NO_RAIL_B)],
            mismatch(Rail::A, true),
        ),
        (
            "after a jumper fault is cleared with the jumper still fitted, an update",
            &[
                IDLE,
                ARM,
                ARM.jumper(),
                IDLE.jumper().panel_clear(),
                IDLE.jumper(),
            ],
            FIRMWARE,
        ),
        (
            "a restart during an update keeps it once the jumper is seen",
            &[IDLE, IDLE.jumper(), ARM.jumper().restart()],
            FIRMWARE,
        ),
    ]);
}

#[test]
fn rules_id_is_pinned() {
    use sawbones_interlock::{rules_id, RULES_VERSION};
    // If this fails, a placeholder constant or RULES_VERSION changed. Update
    // it on purpose: the supervisor's expected id changes with it. The value
    // was checked against an independent FNV-1a in Python over the words
    // [3, 100, 300, 5, 4, 10] as little-endian u64s.
    assert_eq!(RULES_VERSION, 3);
    assert_eq!(rules_id(), 0x1233_e61c_0e2e_b342);
}

// ---------------------------------------------------------------------------
// #197: a restart must not forget a fault, and only the panel clears one.
// ---------------------------------------------------------------------------

/// The gap from hw-008: a fault latched while armed, then the power goes and
/// RAM with it. The record brings the fault back; the arm switch alone does
/// nothing.
#[test]
fn regression_197_a_power_loss_keeps_a_recorded_fault() {
    check(&[
        (
            "e-stop while armed, power lost, stop released, switch cycled",
            &[
                IDLE,
                ARM,
                ARM.estop(),
                ARM.power_loss(),
                IDLE,
                ARM,
                IDLE,
                ARM,
            ],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "lost heartbeat while armed, power lost",
            &[IDLE, ARM, ARM.no_heartbeat(), IDLE.power_loss(), IDLE, ARM],
            fault(FaultReason::HeartbeatLost),
        ),
        (
            "jumper fault, power lost",
            &[IDLE, ARM, ARM.jumper(), IDLE.power_loss(), IDLE, ARM],
            fault(FaultReason::FirmwareJumperWhileArmed),
        ),
        (
            "only a panel clear with the switch off brings it back",
            &[
                IDLE,
                ARM,
                ARM.estop(),
                ARM.power_loss(),
                IDLE.panel_clear(),
                ARM,
            ],
            RAIL_A_ONLY,
        ),
    ]);
}

/// The record was last written while armed: a fault may have latched after,
/// with no time to write it. The restart cannot prove there was none.
#[test]
fn regression_197_a_power_loss_while_armed_is_unknown() {
    check(&[
        (
            "power lost while armed",
            &[IDLE, ARM, ARM.power_loss()],
            UNKNOWN,
        ),
        (
            "power lost with a lease live",
            &script(&granted(), &[KEYED.power_loss()]),
            UNKNOWN,
        ),
        (
            "the switch going off then on does not arm",
            &[IDLE, ARM, ARM.power_loss(), IDLE, ARM, IDLE, ARM],
            UNKNOWN,
        ),
        (
            "a supervisor clear does not clear it",
            &[
                IDLE,
                ARM,
                ARM.power_loss(),
                IDLE.button().event(SUPERVISOR_CLEAR),
                ARM,
            ],
            UNKNOWN,
        ),
        (
            "a panel clear then off and on arms",
            &[IDLE, ARM, ARM.power_loss(), IDLE.panel_clear(), ARM],
            RAIL_A_ONLY,
        ),
    ]);
}

#[test]
fn regression_197_no_readable_record_is_unknown() {
    let torn = IDLE.power_loss_with(None);
    check(&[
        ("from disarmed", &[IDLE, torn], UNKNOWN),
        ("and stays so", &[IDLE, torn, ARM, IDLE, ARM], UNKNOWN),
        (
            "until the panel clears it",
            &[IDLE, torn, IDLE.panel_clear()],
            ALL_OFF_DISARMED,
        ),
        (
            "and the switch then arms",
            &[IDLE, torn, IDLE.panel_clear(), ARM],
            RAIL_A_ONLY,
        ),
        (
            "a clean record comes up unfaulted",
            &[IDLE, IDLE.power_loss_with(Some(FaultRecord::Clear))],
            ALL_OFF_DISARMED,
        ),
        (
            "a recorded fault comes up latched",
            &[
                IDLE,
                IDLE.power_loss_with(Some(FaultRecord::Latched(FaultReason::EmergencyStop))),
            ],
            fault(FaultReason::EmergencyStop),
        ),
    ]);
}

/// A firmware that never feeds the restart stays at power-on: everything
/// off, the fault state unknown, whatever the switches do.
#[test]
fn regression_197_power_on_without_a_restart_stays_unknown() {
    let mut bench = Bench::unbooted();
    for tick in [IDLE, ARM, IDLE, ARM, KEYED.button(), IDLE, ARM] {
        let out = bench.tick(&tick);
        assert_eq!(trim(out), UNKNOWN);
    }
}

/// The state kept in RAM and the record are combined: the worse one wins.
#[test]
fn regression_197_a_restart_is_never_cleaner_than_ram_or_record() {
    let boot = |mode: Mode, record: Option<FaultRecord>| {
        let state = State {
            mode,
            ..State::POWER_ON
        };
        let inputs = Inputs {
            now: 5,
            ..IDLE.inputs
        };
        let (next, _) = step(state, inputs, Some(Event::Restart { record, nonce: 9 }));
        next.mode
    };
    let stopped = Mode::Faulted(FaultReason::EmergencyStop);
    let unknown = Mode::Faulted(FaultReason::FaultStateUnknown);
    let armed = Mode::Armed {
        since: 1,
        approval: sawbones_interlock::Approval::Idle,
    };
    let latched_hb = Some(FaultRecord::Latched(FaultReason::HeartbeatLost));
    for (kept, record, want) in [
        (stopped, Some(FaultRecord::Clear), stopped),
        (stopped, None, stopped),
        (stopped, latched_hb, stopped),
        (armed, Some(FaultRecord::Clear), unknown),
        (armed, latched_hb, Mode::Faulted(FaultReason::HeartbeatLost)),
        (Mode::Disarmed, Some(FaultRecord::Armed), unknown),
        (Mode::Disarmed, None, unknown),
        (
            Mode::Disarmed,
            latched_hb,
            Mode::Faulted(FaultReason::HeartbeatLost),
        ),
        // Disarmed and clear: up, but waiting for the switch (it is off here).
        (Mode::Disarmed, Some(FaultRecord::Clear), Mode::Disarmed),
        // POWER_ON says nothing: the record decides.
        (unknown, Some(FaultRecord::Clear), Mode::Disarmed),
        (unknown, None, unknown),
        (unknown, Some(FaultRecord::Armed), unknown),
    ] {
        assert_eq!(boot(kept, record), want, "{kept:?} + {record:?}");
    }
}

#[test]
fn regression_197_the_record_says_what_to_persist() {
    let armed = Mode::Armed {
        since: 1,
        approval: sawbones_interlock::Approval::Idle,
    };
    assert_eq!(armed.fault_record(), FaultRecord::Armed);
    for mode in [Mode::AwaitArmOff, Mode::Disarmed, Mode::FirmwareUpdate] {
        assert_eq!(mode.fault_record(), FaultRecord::Clear, "{mode:?}");
    }
    for reason in [
        FaultReason::EmergencyStop,
        FaultReason::HeartbeatLost,
        FaultReason::FaultStateUnknown,
    ] {
        assert_eq!(
            Mode::Faulted(reason).fault_record(),
            FaultRecord::Latched(reason)
        );
    }
    assert_eq!(
        State::POWER_ON.fault_record(),
        FaultRecord::Latched(FaultReason::FaultStateUnknown)
    );
}

/// hw-009: a clear from the supervisor is refused in every state, with the
/// button held or not, and changes nothing.
#[test]
fn regression_197_a_supervisor_clear_is_refused_in_every_state() {
    let prefixes: [(&str, Vec<Tick>); 7] = [
        ("awaiting arm off", vec![ARM]),
        ("disarmed", vec![IDLE]),
        ("armed", ARMED.to_vec()),
        ("lease live", granted()),
        ("firmware update", vec![IDLE, IDLE.jumper()]),
        ("e-stop latched", vec![IDLE, ARM, ARM.estop()]),
        ("unknown after restart", vec![IDLE, ARM, ARM.restart()]),
    ];
    for (name, prefix) in prefixes {
        let last = *prefix.last().unwrap();
        let before = trim(run(&prefix));
        for held in [false, true] {
            let mut tick = last.event(SUPERVISOR_CLEAR);
            tick.boot = Boot::No;
            tick.inputs.arm_switch_on = false;
            tick.inputs.panel_button_pressed = held;
            // The same tick as a panel clear would need, so only the source
            // differs.
            let got = trim(run(&script(&prefix, &[tick])));
            assert_eq!(got.refusal, Some(Refusal::ClearNotFromPanel), "{name}");
            assert_eq!(got.fault, before.fault, "{name}: the fault changed");
        }
    }
}

#[test]
fn regression_197_a_panel_clear_needs_the_button_sensed() {
    let stopped = fault(FaultReason::EmergencyStop);
    check(&[
        (
            "token without the pin",
            &[IDLE, ARM, ARM.estop(), IDLE.event(PANEL_CLEAR)],
            stopped.refused(Refusal::PanelButtonNotPressed),
        ),
        (
            "token with the pin",
            &[IDLE, ARM, ARM.estop(), IDLE.panel_clear()],
            ALL_OFF_DISARMED,
        ),
        (
            "the pin alone clears nothing",
            &[IDLE, ARM, ARM.estop(), IDLE.button(), IDLE.button()],
            stopped,
        ),
    ]);
}

// ---------------------------------------------------------------------------
// #198: the heartbeat is a challenge the interlock issues and the supervisor
// answers.
// ---------------------------------------------------------------------------

/// The bench armed, with the outstanding challenge answered.
fn armed_bench() -> Bench {
    let mut bench = Bench::booted();
    for tick in ARMED {
        bench.tick(&tick);
    }
    assert!(bench.last.rail_a);
    bench
}

#[test]
fn regression_198_correct_replies_keep_the_rail_on() {
    let mut bench = armed_bench();
    let first = bench.last.challenge;
    for _ in 0..(20 * HEARTBEAT_PERIOD_TICKS) {
        let out = bench.tick(&ARM);
        assert!(out.rail_a && out.fault.is_none());
    }
    assert_ne!(bench.last.challenge, first, "the challenge never changed");
}

#[test]
fn regression_198_every_bad_reply_latches() {
    let lost = fault(FaultReason::HeartbeatLost);
    check(&[
        ("a wrong reply", &[IDLE, ARM, ARM.no_heartbeat()], lost),
        (
            "a repeated reply",
            &[IDLE, ARM, ARM.beat(Beat::Repeat)],
            lost,
        ),
        (
            "a stale reply, to the challenge before",
            &script(
                &ARMED,
                &[
                    ARM.after(HEARTBEAT_PERIOD_TICKS),
                    ARM,
                    ARM.beat(Beat::Stale),
                ],
            ),
            lost,
        ),
        (
            "a reply sent twice",
            &script(
                &ARMED,
                &[ARM.after(HEARTBEAT_PERIOD_TICKS), ARM.beat(Beat::Repeat)],
            ),
            lost,
        ),
    ]);
}

#[test]
fn regression_198_a_missing_reply_latches_at_the_deadline() {
    let mut bench = armed_bench();
    // Run to a fresh challenge, answered on the tick after it appears.
    while bench.state.heartbeat.answered || bench.state.heartbeat.issued_at == 0 {
        bench.tick(&ARM);
    }
    let issued = bench.state.heartbeat.issued_at;
    // Silent from here: the rail holds until the period ends unanswered.
    loop {
        let out = bench.tick(&ARM.beat(Beat::Silent));
        if out.fault.is_some() {
            assert_eq!(out.fault, Some(FaultReason::HeartbeatLost));
            assert!(!out.rail_a && out.rail_b.is_none());
            assert_eq!(bench.now, issued + HEARTBEAT_PERIOD_TICKS);
            break;
        }
        assert!(out.rail_a);
    }
}

/// The stated stopping-time budget: from the last good reply to every rail
/// off is at most two periods.
#[test]
fn regression_198_stopping_time_is_two_periods_at_most() {
    for answer_at in 1..HEARTBEAT_PERIOD_TICKS {
        let mut bench = armed_bench();
        // Wait for a new challenge, then let the supervisor answer it late
        // by `answer_at` ticks and fall silent.
        let start = bench.state.heartbeat.seq;
        while bench.state.heartbeat.seq == start {
            bench.tick(&ARM);
        }
        for _ in 1..answer_at {
            bench.tick(&ARM.beat(Beat::Silent));
        }
        bench.tick(&ARM);
        assert!(bench.last.rail_a, "answered at {answer_at}");
        let last_good = bench.now;
        while bench.last.fault.is_none() {
            bench.tick(&ARM.beat(Beat::Silent));
        }
        assert!(
            bench.now - last_good < 2 * HEARTBEAT_PERIOD_TICKS,
            "dropped {} ticks after the last good reply",
            bench.now - last_good
        );
    }
}

/// A reply on the tick its challenge is issued cannot have seen it.
#[test]
fn regression_198_a_reply_before_the_challenge_is_seen_is_lost() {
    let bench = armed_bench();
    let watch = bench.state.heartbeat;
    let next = HeartbeatWatch {
        seq: watch.seq + 1,
        ..watch
    };
    // Jump to the tick the next challenge is issued, and answer it there.
    let inputs = Inputs {
        now: watch.issued_at + HEARTBEAT_PERIOD_TICKS,
        rail_a_sensed: true,
        heartbeat: next.challenge().map(HeartbeatChallenge::reply),
        ..ARM.inputs
    };
    let (state, out) = step(bench.state, inputs, None);
    assert_eq!(state.heartbeat.seq, next.seq);
    assert_eq!(out.fault, Some(FaultReason::HeartbeatLost));
}

/// Nothing arms until a correct reply has been seen, and a reply that comes
/// later does not arm a switch that was already on.
#[test]
fn regression_198_no_arm_without_a_reply() {
    check(&[
        (
            "switch off then on with no reply at all",
            &[
                IDLE.beat(Beat::Silent),
                ARM.beat(Beat::Silent).event(Event::RequestArm),
            ],
            ALL_OFF_AWAIT.refused(Refusal::HeartbeatUnhealthy),
        ),
        (
            "the first reply comes with the switch still on",
            &[IDLE.beat(Beat::Silent), ARM.beat(Beat::Silent), ARM, ARM],
            ALL_OFF_AWAIT,
        ),
        (
            "then off and on arms",
            &[
                IDLE.beat(Beat::Silent),
                ARM.beat(Beat::Silent),
                ARM,
                IDLE,
                ARM,
            ],
            RAIL_A_ONLY,
        ),
        (
            "no reply for a whole period after boot latches",
            &[
                IDLE.beat(Beat::Silent),
                IDLE.beat(Beat::Silent).after(HEARTBEAT_PERIOD_TICKS),
            ],
            fault(FaultReason::HeartbeatLost),
        ),
    ]);
}

/// Replies recorded in one boot are worthless in the next: the challenges
/// come from the boot nonce.
#[test]
fn regression_198_replies_from_another_boot_do_not_count() {
    let boot = |nonce| {
        let inputs = Inputs {
            now: 0,
            ..IDLE.inputs
        };
        let restart = Event::Restart {
            record: Some(FaultRecord::Clear),
            nonce,
        };
        step(State::POWER_ON, inputs, Some(restart)).0
    };
    let old = boot(1);
    let new = boot(2);
    assert_ne!(old.heartbeat.challenge(), new.heartbeat.challenge());
    let replay = Inputs {
        now: 1,
        heartbeat: old.heartbeat.challenge().map(HeartbeatChallenge::reply),
        ..IDLE.inputs
    };
    let (_, out) = step(new, replay, None);
    assert_eq!(out.fault, Some(FaultReason::HeartbeatLost));
}

/// The reply is not an echo of the challenge, and challenges do not repeat
/// within a boot.
#[test]
fn regression_198_the_reply_is_not_an_echo() {
    let mut seen = std::collections::HashSet::new();
    for seq in 0..10_000 {
        let challenge = HeartbeatWatch {
            seq,
            ..HeartbeatWatch::start(3, 0)
        }
        .challenge()
        .expect("started");
        assert_ne!(challenge.reply().0, challenge.0);
        assert!(seen.insert(challenge), "challenge repeated at {seq}");
    }
}

/// A stalled step loop cannot stretch the budget: a sample two periods after
/// the challenge was issued is a loss, even with that challenge answered.
#[test]
fn regression_198_a_stalled_loop_is_a_loss() {
    let bench = armed_bench();
    let watch = bench.state.heartbeat;
    assert!(watch.answered && bench.last.rail_a);
    let sample = |now| Inputs {
        now,
        rail_a_sensed: true,
        ..ARM.inputs
    };
    // One period on, answered: a new challenge, rail still on.
    let (_, out) = step(bench.state, sample(watch.issued_at + P), None);
    assert!(out.rail_a, "a period on, answered, the rail stays on");
    // Two periods on, with no sample between: lost.
    let (_, out) = step(bench.state, sample(watch.issued_at + 2 * P), None);
    assert_eq!(out.fault, Some(FaultReason::HeartbeatLost));
    assert!(!out.rail_a);
}

const P: u64 = HEARTBEAT_PERIOD_TICKS;

/// What the challenge of a heartbeat "started" with nonce 0 at tick 0 would
/// have been, computed here independently: FNV-1a over the challenge tag,
/// the nonce and the sequence number. A reply to it looks correct.
fn zero_nonce_reply() -> HeartbeatReply {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for word in [0x6567_6e65_6c6c_6168_u64, 0, 0] {
        for b in word.to_le_bytes() {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    HeartbeatChallenge(hash).reply()
}

/// Until a restart delivers a non-zero hardware nonce, the heartbeat has not
/// started: no challenge, every reply fails, no clear succeeds and nothing
/// arms, even if the firmware never sends the restart at all.
#[test]
fn regression_198_no_heartbeat_before_restart() {
    let reply = zero_nonce_reply();
    let tick = |state: State, now, arm, button, event| {
        let inputs = Inputs {
            now,
            arm_switch_on: arm,
            panel_button_pressed: button,
            heartbeat: Some(reply),
            ..IDLE.inputs
        };
        step(state, inputs, event)
    };
    assert_eq!(State::POWER_ON.outputs().challenge, None);
    assert!(!State::POWER_ON.heartbeat.started());

    // From POWER_ON with no restart: a correct-looking reply, a panel clear
    // with the switch off, then the switch on. Rails stay off throughout.
    let mut state = State::POWER_ON;
    for (now, arm, button, event) in [
        (1, false, false, None),
        (2, false, true, Some(PANEL_CLEAR)),
        (3, false, false, None),
        (4, true, false, Some(Event::RequestArm)),
        (5, false, true, Some(PANEL_CLEAR)),
        (6, true, false, None),
    ] {
        let (next, out) = tick(state, now, arm, button, event);
        assert!(!out.rail_a && out.rail_b.is_none(), "tick {now}: a rail on");
        assert!(out.fault.is_some(), "tick {now}: the fault cleared");
        assert_eq!(out.challenge, None, "tick {now}: a challenge with no nonce");
        if event == Some(PANEL_CLEAR) {
            assert_eq!(out.refusal, Some(Refusal::HeartbeatUnhealthy));
        }
        assert!(!next.heartbeat.alive);
        state = next;
    }

    // A restart with nonce 0 is no start: a clean record comes up unfaulted,
    // but there is still no challenge, any reply is a loss, and with no reply
    // the arm switch cannot arm.
    let restart = Event::Restart {
        record: Some(FaultRecord::Clear),
        nonce: 0,
    };
    let boot = Inputs {
        now: 1,
        ..IDLE.inputs
    };
    let (booted, out) = step(State::POWER_ON, boot, Some(restart));
    assert_eq!(booted.mode, Mode::Disarmed);
    assert_eq!(out.challenge, None);
    let (_, out) = tick(booted, 2, true, false, None);
    assert_eq!(out.fault, Some(FaultReason::HeartbeatLost));
    assert!(!out.rail_a);
    let silent = Inputs {
        now: 2,
        arm_switch_on: true,
        ..IDLE.inputs
    };
    let (next, out) = step(booted, silent, Some(Event::RequestArm));
    assert!(!out.rail_a);
    assert_eq!(next.mode, Mode::AwaitArmOff);
    assert_eq!(out.refusal, Some(Refusal::HeartbeatUnhealthy));
    let (_, out) = step(next, Inputs { now: 3, ..silent }, None);
    assert!(!out.rail_a);

    // A real nonce starts it.
    let restart = Event::Restart {
        record: Some(FaultRecord::Clear),
        nonce: 41,
    };
    let (started, out) = step(State::POWER_ON, boot, Some(restart));
    assert!(started.heartbeat.started() && out.challenge.is_some());
}
