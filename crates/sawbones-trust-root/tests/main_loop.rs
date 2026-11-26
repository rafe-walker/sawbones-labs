//! The trust root's main loop against a simulated board (#114, #132, #144):
//! the boot sequence, the record written before any output turns on, and the
//! charge pump fed only from loop passes with a rail on.

mod sim;

use std::num::NonZeroU64;

use sawbones_interlock::{
    ActionDigest, FaultReason, FaultRecord, HazardClass, HeartbeatReply, LeaseTarget, Mode, Port,
    Refusal, HEARTBEAT_PERIOD_TICKS,
};
use sawbones_trust_root::{record, EventSource, LinkRequest, RecordWrite, TrustRoot};
use sim::{Op, SimBoard};

fn tick(root: &mut TrustRoot<SimBoard>) -> sawbones_trust_root::TickReport {
    let report = root.tick();
    root.board_mut().now += 1;
    report
}

fn assert_clean(root: &TrustRoot<SimBoard>) {
    assert!(
        root.board().violations.is_empty(),
        "{:?}",
        root.board().violations
    );
}

/// Boots from a clear record and arms rail A: returns the root, armed.
fn armed() -> TrustRoot<SimBoard> {
    let mut root = TrustRoot::new(SimBoard::with_record(Some(record::encode(
        FaultRecord::Clear,
    ))));
    tick(&mut root); // restart
    tick(&mut root); // the first challenge is answered
    root.board_mut().arm_switch_on = true;
    let report = tick(&mut root);
    assert!(report.outputs.rail_a, "{report:?}");
    assert!(report.rail_a_driven);
    root
}

const TARGET: LeaseTarget = LeaseTarget {
    port: Port(2),
    action_digest: ActionDigest([7; 32]),
};

#[test]
fn outputs_are_driven_off_before_the_first_tick() {
    let root = TrustRoot::new(SimBoard::new());
    assert_eq!(
        root.board().log,
        vec![Op::Drive {
            rail_a: false,
            rail_b: None
        }]
    );
}

#[test]
fn boot_reads_the_record_and_restarts_with_the_hardware_nonce() {
    let mut root = TrustRoot::new(SimBoard::with_record(Some(record::encode(
        FaultRecord::Clear,
    ))));
    let report = tick(&mut root);
    assert_eq!(report.event, EventSource::Restart);

    let log = &root.board().log;
    let read = log.iter().position(|op| *op == Op::ReadRecord).unwrap();
    let nonce = log.iter().position(|op| *op == Op::Nonce).unwrap();
    let first_write = log
        .iter()
        .position(|op| matches!(op, Op::WriteRecord(_)))
        .unwrap();
    assert!(read < first_write && nonce < first_write, "{log:?}");
    // The supervisor's link is not read on the restart tick: the restart is
    // the tick's one event.
    assert!(!log[..first_write].contains(&Op::LinkRequest));

    let state = root.state();
    assert_eq!(state.heartbeat.nonce, root.board().nonce);
    assert!(report.outputs.challenge.is_some());
    assert_eq!(state.mode, Mode::Disarmed);
    assert_clean(&root);
}

#[test]
fn boot_with_a_missing_or_corrupt_record_comes_up_faulted() {
    let mut torn = record::encode(FaultRecord::Clear);
    torn[1] ^= 0x100;
    for stored in [None, Some([0; 4]), Some([u32::MAX; 4]), Some(torn)] {
        let mut root = TrustRoot::new(SimBoard::with_record(stored));
        let report = tick(&mut root);
        assert_eq!(
            report.outputs.fault,
            Some(FaultReason::FaultStateUnknown),
            "{stored:?}"
        );
        // The unknown fault is written back at once, after the cut.
        assert_eq!(
            report.record,
            RecordWrite::Written(FaultRecord::Latched(FaultReason::FaultStateUnknown))
        );
        assert_clean(&root);
    }
}

#[test]
fn boot_with_an_armed_record_comes_up_faulted() {
    let mut root = TrustRoot::new(SimBoard::with_record(Some(record::encode(
        FaultRecord::Armed,
    ))));
    root.board_mut().arm_switch_on = true;
    for _ in 0..20 {
        let report = tick(&mut root);
        assert_eq!(report.outputs.fault, Some(FaultReason::FaultStateUnknown));
        assert!(!report.rail_a_driven);
    }
    assert_eq!(root.board().pump_toggles(), 0);
    assert_clean(&root);
}

#[test]
fn a_failed_random_number_generator_leaves_the_bench_off() {
    let mut board = SimBoard::with_record(Some(record::encode(FaultRecord::Clear)));
    board.nonce = None;
    let mut root = TrustRoot::new(board);
    tick(&mut root);
    assert!(!root.state().heartbeat.started());
    // Whatever the supervisor sends and however the switch moves, nothing
    // arms.
    for i in 0..(4 * HEARTBEAT_PERIOD_TICKS) {
        root.board_mut().arm_switch_on = i % 3 == 0;
        root.board_mut().replies.push(HeartbeatReply(i));
        let report = tick(&mut root);
        assert!(!report.rail_a_driven);
    }
    assert_eq!(root.board().pump_toggles(), 0);
    assert_clean(&root);
}

#[test]
fn the_record_is_written_and_read_back_before_rail_a_turns_on() {
    let root = armed();
    let log = &root.board().log;
    let on = log
        .iter()
        .position(|op| {
            matches!(
                op,
                Op::Drive {
                    rail_a: true,
                    rail_b: None
                }
            )
        })
        .unwrap();
    let armed_words = record::encode(FaultRecord::Armed);
    let write = log[..on]
        .iter()
        .rposition(|op| *op == Op::WriteRecord(armed_words))
        .expect("the Armed record is written before rail A turns on");
    assert!(
        log[write..on].contains(&Op::ReadRecord),
        "and read back: {log:?}"
    );
    // And the pump is fed only after the rail is on.
    assert!(!log[..on].contains(&Op::TogglePump));
    assert_eq!(log[on + 1], Op::TogglePump);
    assert_clean(&root);
}

#[test]
fn the_record_is_written_before_rail_b_turns_on() {
    let mut root = armed();
    root.board_mut().key_switch_on = true;
    root.board_mut().requests.push(LinkRequest::RequestLease {
        class: HazardClass::Class3,
        target: TARGET,
    });
    tick(&mut root);
    root.board_mut().panel_button_pressed = true;
    let before = root.board().log.len();
    let report = tick(&mut root);
    assert_eq!(report.rail_b_driven, Some(Port(2)));
    let log = &root.board().log[before..];
    let on = log
        .iter()
        .position(|op| {
            matches!(
                op,
                Op::Drive {
                    rail_b: Some(_),
                    ..
                }
            )
        })
        .unwrap();
    assert!(log[..on].contains(&Op::WriteRecord(record::encode(FaultRecord::Armed))));
    assert_clean(&root);
}

#[test]
fn a_failed_record_write_keeps_the_output_off_and_the_pump_still() {
    let mut root = TrustRoot::new(SimBoard::with_record(Some(record::encode(
        FaultRecord::Clear,
    ))));
    tick(&mut root);
    tick(&mut root);
    root.board_mut().fail_writes = true;
    root.board_mut().arm_switch_on = true;
    let report = tick(&mut root);
    assert!(report.outputs.rail_a, "step armed");
    assert!(!report.rail_a_driven, "but the rail stayed off");
    assert_eq!(report.record, RecordWrite::Failed(FaultRecord::Armed));
    assert!(!report.pump_toggled);
    // The interlock's feedback check latches a fault within the settle
    // window, because rail A is commanded on and never sensed live.
    let mut faulted = false;
    for _ in 0..10 {
        let report = tick(&mut root);
        assert!(!report.rail_a_driven);
        if report.outputs.fault.is_some() {
            faulted = true;
            break;
        }
    }
    assert!(faulted);
    assert_eq!(root.board().pump_toggles(), 0);
    assert_clean(&root);
}

#[test]
fn a_store_that_does_not_read_back_counts_as_a_failed_write() {
    let mut root = TrustRoot::new(SimBoard::with_record(Some(record::encode(
        FaultRecord::Clear,
    ))));
    tick(&mut root);
    tick(&mut root);
    root.board_mut().corrupt_writes = true;
    root.board_mut().arm_switch_on = true;
    let report = tick(&mut root);
    assert!(!report.rail_a_driven);
    assert_eq!(report.record, RecordWrite::Failed(FaultRecord::Armed));
    assert_clean(&root);
}

#[test]
fn an_emergency_stop_cuts_before_the_record_is_written() {
    let mut root = armed();
    root.board_mut().estop_pressed = true;
    let before = root.board().log.len();
    let report = tick(&mut root);
    assert!(!report.rail_a_driven);
    assert!(!report.pump_toggled);
    let log = &root.board().log[before..];
    let off = log
        .iter()
        .position(|op| {
            *op == Op::Drive {
                rail_a: false,
                rail_b: None,
            }
        })
        .unwrap();
    let write = log
        .iter()
        .position(|op| {
            *op == Op::WriteRecord(record::encode(FaultRecord::Latched(
                FaultReason::EmergencyStop,
            )))
        })
        .unwrap();
    assert!(off < write, "{log:?}");
    assert_clean(&root);
}

#[test]
fn the_pump_is_fed_once_per_pass_while_a_rail_is_on_and_never_otherwise() {
    let mut root = armed();
    let toggles_when_armed = root.board().pump_toggles();
    assert_eq!(toggles_when_armed, 1);
    for _ in 0..50 {
        let report = tick(&mut root);
        assert!(report.rail_a_driven && report.pump_toggled);
    }
    assert_eq!(root.board().pump_toggles(), 51);
    // The supervisor goes quiet: the heartbeat is lost, the rail drops and
    // the pump stops on that same pass.
    root.board_mut().auto_reply = false;
    let mut dropped = false;
    for _ in 0..(3 * HEARTBEAT_PERIOD_TICKS) {
        let report = tick(&mut root);
        if !report.rail_a_driven {
            dropped = true;
            assert!(!report.pump_toggled);
        }
        if dropped {
            assert!(!report.rail_a_driven);
        }
    }
    assert!(dropped);
    assert_eq!(root.state().mode, Mode::Faulted(FaultReason::HeartbeatLost));
    let after_drop = root.board().pump_toggles();
    for _ in 0..20 {
        tick(&mut root);
    }
    assert_eq!(root.board().pump_toggles(), after_drop);
    assert_clean(&root);
}

#[test]
fn a_stalled_loop_stops_the_pump() {
    // The pump is fed only by passes of the loop: no pass, no toggle. A board
    // that is never ticked again sees no further toggles at all.
    let root = armed();
    let toggles = root.board().pump_toggles();
    let last = root
        .board()
        .log
        .iter()
        .rposition(|op| *op == Op::TogglePump);
    assert_eq!(toggles, 1);
    // The toggle is inside the pass: after the sample, before the watchdog.
    let log = &root.board().log;
    let last = last.unwrap();
    let sample = log[..last]
        .iter()
        .rposition(|op| *op == Op::Sample)
        .unwrap();
    let feed = log[last..]
        .iter()
        .position(|op| *op == Op::FeedWatchdog)
        .unwrap();
    assert!(sample < last && feed > 0);
}

#[test]
fn a_clear_from_the_link_is_refused_and_only_the_panel_clears() {
    let mut root = TrustRoot::new(SimBoard::new()); // no record: faulted
    tick(&mut root);
    tick(&mut root); // heartbeat answered
    root.board_mut().requests.push(LinkRequest::ClearFault);
    let report = tick(&mut root);
    assert_eq!(report.event, EventSource::Link);
    assert_eq!(report.outputs.refusal, Some(Refusal::ClearNotFromPanel));
    assert!(report.outputs.fault.is_some());

    root.board_mut().panel_button_pressed = true;
    let report = tick(&mut root);
    assert_eq!(report.event, EventSource::Panel);
    assert_eq!(report.outputs.fault, None);
    assert_eq!(report.record, RecordWrite::Written(FaultRecord::Clear));
    // Holding the button is not a second press.
    let report = tick(&mut root);
    assert_eq!(report.event, EventSource::None);
    assert_clean(&root);
}

#[test]
fn a_link_request_waits_while_the_panel_takes_the_tick() {
    let mut root = TrustRoot::new(SimBoard::new());
    tick(&mut root);
    tick(&mut root);
    root.board_mut().requests.push(LinkRequest::RequestArm);
    root.board_mut().panel_button_pressed = true;
    let report = tick(&mut root);
    assert_eq!(report.event, EventSource::Panel);
    assert_eq!(
        root.board().requests.len(),
        1,
        "the request is still queued"
    );
    let report = tick(&mut root);
    assert_eq!(report.event, EventSource::Link);
    assert_clean(&root);
}

/// A small deterministic generator, so the random walk needs no crate.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn chance(&mut self, one_in: u64) -> bool {
        self.next().is_multiple_of(one_in)
    }
}

#[test]
fn random_walks_never_break_the_firmware_rules() {
    for seed in 1..=200u64 {
        let mut rng = XorShift(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
        let stored = match rng.next() % 4 {
            0 => None,
            1 => Some(record::encode(FaultRecord::Armed)),
            _ => Some(record::encode(FaultRecord::Clear)),
        };
        let mut board = SimBoard::with_record(stored);
        board.nonce = NonZeroU64::new(rng.next() % 8);
        let mut root = TrustRoot::new(board);
        for _ in 0..400 {
            let b = root.board_mut();
            if rng.chance(8) {
                b.arm_switch_on = !b.arm_switch_on;
            }
            if rng.chance(10) {
                b.key_switch_on = !b.key_switch_on;
            }
            if rng.chance(6) {
                b.panel_button_pressed = !b.panel_button_pressed;
            }
            b.estop_pressed = rng.chance(150);
            b.firmware_jumper_fitted = rng.chance(200);
            b.fail_writes = rng.chance(20);
            b.corrupt_writes = rng.chance(40);
            b.stuck_rail_a = if rng.chance(300) { Some(true) } else { None };
            b.auto_reply = !rng.chance(60);
            if rng.chance(15) {
                b.requests.push(match rng.next() % 4 {
                    0 => LinkRequest::RequestArm,
                    1 => LinkRequest::RequestLease {
                        class: HazardClass::Class3,
                        target: LeaseTarget {
                            port: Port((rng.next() % 4) as u8),
                            action_digest: ActionDigest([1; 32]),
                        },
                    },
                    2 => LinkRequest::LeaseUsed { target: TARGET },
                    _ => LinkRequest::ClearFault,
                });
            }
            if rng.chance(30) {
                b.replies.push(HeartbeatReply(rng.next()));
            }
            tick(&mut root);
            if !root.board().violations.is_empty() {
                panic!("seed {seed}: {:?}", root.board().violations);
            }
        }
    }
}
