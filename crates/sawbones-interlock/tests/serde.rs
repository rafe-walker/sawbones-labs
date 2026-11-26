//! The state and outputs survive a trip through JSON, so the supervisor can
//! log them and read them back.
#![cfg(feature = "serde")]

use sawbones_interlock::{step, Approval, Event, HazardClass, Inputs, Outputs, State};

#[test]
fn state_and_outputs_round_trip_through_json() {
    let state = State::Armed(Approval::Live { granted_at: 42 });
    let json = serde_json::to_string(&state).unwrap();
    assert_eq!(json, r#"{"armed":{"live":{"granted_at":42}}}"#);
    assert_eq!(serde_json::from_str::<State>(&json).unwrap(), state);

    let inputs = Inputs {
        estop_pressed: false,
        arm_switch_on: true,
        key_switch_on: true,
        heartbeat_healthy: true,
        panel_button_pressed: false,
        now: 43,
    };
    let event = Event::RequestLease {
        class: HazardClass::Class4,
    };
    let event: Event = serde_json::from_str(&serde_json::to_string(&event).unwrap()).unwrap();
    let (_, outputs) = step(state, inputs, Some(event));
    let json = serde_json::to_string(&outputs).unwrap();
    assert_eq!(
        json,
        r#"{"rail_a":true,"rail_b":true,"panel":"lease_live","fault":null,"refusal":"class_never_permitted"}"#
    );
    assert_eq!(serde_json::from_str::<Outputs>(&json).unwrap(), outputs);
}
