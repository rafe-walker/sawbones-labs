//! The output decision needs two independent reads to agree (#238): the
//! `Outputs` that `step` returned and the `State` mode it returned. A single
//! flipped value must leave the rails off. Host-only; says nothing about a
//! real glitch on the chip (docs/THREAT-MODEL.md, section 4a).

use sawbones_interlock::{
    ActionDigest, Approval, FaultReason, LeaseTarget, Mode, Outputs, Port, State,
};
use sawbones_trust_root::{permitted, Driven};

const OFF: Driven = Driven {
    rail_a: false,
    rail_b: None,
};

fn target(port: u8) -> LeaseTarget {
    LeaseTarget {
        port: Port(port),
        action_digest: ActionDigest([7; 32]),
    }
}

fn with_mode(mode: Mode) -> State {
    State {
        mode,
        ..State::POWER_ON
    }
}

fn states() -> Vec<State> {
    vec![
        with_mode(Mode::AwaitArmOff),
        with_mode(Mode::Disarmed),
        with_mode(Mode::FirmwareUpdate),
        with_mode(Mode::Faulted(FaultReason::FaultStateUnknown)),
        with_mode(Mode::Armed {
            since: 1,
            approval: Approval::Idle,
        }),
        with_mode(Mode::Armed {
            since: 1,
            approval: Approval::AwaitPress {
                requested_at: 2,
                target: target(2),
            },
        }),
        with_mode(Mode::Armed {
            since: 1,
            approval: Approval::Live {
                granted_at: 3,
                target: target(2),
            },
        }),
    ]
}

#[test]
fn two_agreeing_reads_give_exactly_what_the_state_says() {
    for state in states() {
        let outputs = state.outputs();
        let got = permitted(&outputs, &state);
        assert_eq!(got.rail_a, outputs.rail_a, "{state:?}");
        assert_eq!(got.rail_b, outputs.rail_b, "{state:?}");
    }
}

#[test]
fn a_single_flipped_output_value_leaves_the_rails_off() {
    for state in states() {
        let good = state.outputs();
        let armed = good.rail_a;

        // Flip rail A in the outputs only.
        let flipped = Outputs {
            rail_a: !good.rail_a,
            ..good
        };
        let got = permitted(&flipped, &state);
        if !armed {
            assert_eq!(got, OFF, "rail A flipped on in {state:?}");
        } else {
            assert!(!got.rail_a, "rail A flipped off still on in {state:?}");
        }

        // Rail B claimed for any port the state does not name.
        for port in 0..4 {
            let claimed = Outputs {
                rail_b: Some(Port(port)),
                ..good
            };
            let got = permitted(&claimed, &state);
            if good.rail_b != Some(Port(port)) {
                assert_eq!(got.rail_b, None, "rail B port {port} in {state:?}");
            }
        }

        // A fault value that disagrees with the mode.
        let faulted = Outputs {
            fault: Some(FaultReason::FaultStateUnknown),
            ..good
        };
        assert_eq!(permitted(&faulted, &state), OFF, "{state:?}");
    }
}

#[test]
fn a_single_flipped_state_leaves_the_rails_off() {
    // The outputs say on, the state says off: nothing turns on.
    let on = states()[6].outputs();
    for state in states().into_iter().take(4) {
        assert_eq!(permitted(&on, &state), OFF, "{state:?}");
    }
    // The outputs say off, the state says on: nothing turns on.
    let off = states()[1].outputs();
    for state in states().into_iter().skip(4) {
        assert_eq!(permitted(&off, &state), OFF, "{state:?}");
    }
}
