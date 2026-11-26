//! The state and outputs survive a trip through JSON, so the supervisor can
//! log them and read them back.
#![cfg(feature = "serde")]

use sawbones_interlock::{
    step, ActionDigest, Approval, Event, FaultReason, HazardClass, Inputs, LeaseTarget, Outputs,
    Port, Rail, State, RAIL_B_PORTS,
};

#[test]
fn state_and_outputs_round_trip_through_json() {
    let target = LeaseTarget {
        port: Port(1),
        action_digest: ActionDigest([7; 32]),
    };
    let state = State::Armed {
        since: 40,
        approval: Approval::Live {
            granted_at: 42,
            target,
        },
    };
    let json = serde_json::to_string(&state).unwrap();
    let digest = ["7"; 32].join(",");
    assert_eq!(
        json,
        format!(
            r#"{{"armed":{{"since":40,"approval":{{"live":{{"granted_at":42,"target":{{"port":1,"action_digest":[{digest}]}}}}}}}}}}"#
        )
    );
    assert_eq!(serde_json::from_str::<State>(&json).unwrap(), state);

    let mut rail_b_sensed = [false; RAIL_B_PORTS];
    rail_b_sensed[1] = true;
    let inputs = Inputs {
        estop_pressed: false,
        arm_switch_on: true,
        key_switch_on: true,
        heartbeat_healthy: true,
        panel_button_pressed: false,
        rail_a_sensed: true,
        rail_b_sensed,
        firmware_jumper_fitted: false,
        now: 43,
    };
    let event = Event::RequestLease {
        class: HazardClass::Class4,
        target,
    };
    let event: Event = serde_json::from_str(&serde_json::to_string(&event).unwrap()).unwrap();
    let (_, outputs) = step(state, inputs, Some(event));
    let json = serde_json::to_string(&outputs).unwrap();
    assert_eq!(
        json,
        r#"{"rail_a":true,"rail_b":1,"firmware_update_allowed":false,"panel":"lease_live","fault":null,"refusal":"class_never_permitted"}"#
    );
    assert_eq!(serde_json::from_str::<Outputs>(&json).unwrap(), outputs);
}

#[test]
fn a_feedback_fault_names_its_rail_in_json() {
    let state = State::Faulted(FaultReason::OutputMismatch {
        rail: Rail::B(Port(2)),
        sensed_live: true,
    });
    let json = serde_json::to_string(&state).unwrap();
    assert_eq!(
        json,
        r#"{"faulted":{"output_mismatch":{"rail":{"b":2},"sensed_live":true}}}"#
    );
    assert_eq!(serde_json::from_str::<State>(&json).unwrap(), state);
}
