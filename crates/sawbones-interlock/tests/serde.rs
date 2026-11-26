//! The state and outputs survive a trip through JSON, so the supervisor can
//! log them and read them back.
#![cfg(feature = "serde")]

use sawbones_interlock::{
    step, ActionDigest, Approval, ClearSource, Event, FaultReason, FaultRecord, HazardClass,
    HeartbeatWatch, Inputs, LeaseTarget, Mode, Outputs, PanelToken, Port, Rail, State,
    RAIL_B_PORTS,
};

#[test]
fn state_and_outputs_round_trip_through_json() {
    let target = LeaseTarget {
        port: Port(1),
        action_digest: ActionDigest([7; 32]),
    };
    let state = State {
        mode: Mode::Armed {
            since: 40,
            approval: Approval::Live {
                granted_at: 42,
                target,
            },
        },
        heartbeat: HeartbeatWatch {
            answered: true,
            alive: true,
            ..HeartbeatWatch::start(5, 41)
        },
    };
    let json = serde_json::to_string(&state).unwrap();
    let digest = ["7"; 32].join(",");
    assert_eq!(
        json,
        format!(
            r#"{{"mode":{{"armed":{{"since":40,"approval":{{"live":{{"granted_at":42,"target":{{"port":1,"action_digest":[{digest}]}}}}}}}}}},"heartbeat":{{"nonce":5,"seq":0,"issued_at":41,"answered":true,"alive":true}}}}"#
        )
    );
    assert_eq!(serde_json::from_str::<State>(&json).unwrap(), state);

    let mut rail_b_sensed = [false; RAIL_B_PORTS];
    rail_b_sensed[1] = true;
    let inputs = Inputs {
        estop_pressed: false,
        arm_switch_on: true,
        key_switch_on: true,
        heartbeat: None,
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
    let challenge = state.heartbeat.challenge().0;
    assert_eq!(
        json,
        format!(
            r#"{{"rail_a":true,"rail_b":1,"firmware_update_allowed":false,"panel":"lease_live","fault":null,"challenge":{challenge},"refusal":"class_never_permitted"}}"#
        )
    );
    assert_eq!(serde_json::from_str::<Outputs>(&json).unwrap(), outputs);
}

#[test]
fn a_feedback_fault_names_its_rail_in_json() {
    let mode = Mode::Faulted(FaultReason::OutputMismatch {
        rail: Rail::B(Port(2)),
        sensed_live: true,
    });
    let json = serde_json::to_string(&mode).unwrap();
    assert_eq!(
        json,
        r#"{"faulted":{"output_mismatch":{"rail":{"b":2},"sensed_live":true}}}"#
    );
    assert_eq!(serde_json::from_str::<Mode>(&json).unwrap(), mode);
}

/// The record the firmware keeps round-trips, so it can be stored with serde.
#[test]
fn regression_197_the_fault_record_round_trips() {
    for record in [
        FaultRecord::Clear,
        FaultRecord::Armed,
        FaultRecord::Latched(FaultReason::FaultStateUnknown),
        FaultRecord::Latched(FaultReason::OutputMismatch {
            rail: Rail::A,
            sensed_live: false,
        }),
    ] {
        let json = serde_json::to_string(&record).unwrap();
        assert_eq!(serde_json::from_str::<FaultRecord>(&json).unwrap(), record);
    }
    let restart = Event::Restart {
        record: None,
        nonce: 3,
    };
    let json = serde_json::to_string(&restart).unwrap();
    assert_eq!(json, r#"{"restart":{"record":null,"nonce":3}}"#);
}

/// No bytes from the link can become a panel clear: a panel token logs, but
/// never decodes. A supervisor clear decodes, and is refused by the rules.
#[test]
fn regression_197_a_panel_token_never_deserialises() {
    let panel = Event::ClearFault {
        source: ClearSource::Panel(PanelToken::from_panel_button()),
    };
    let json = serde_json::to_string(&panel).unwrap();
    assert_eq!(json, r#"{"clear_fault":{"source":{"panel":null}}}"#);
    assert!(serde_json::from_str::<Event>(&json).is_err());
    assert!(serde_json::from_str::<PanelToken>("null").is_err());

    let supervisor = Event::ClearFault {
        source: ClearSource::Supervisor,
    };
    let json = serde_json::to_string(&supervisor).unwrap();
    assert_eq!(json, r#"{"clear_fault":{"source":"supervisor"}}"#);
    assert_eq!(serde_json::from_str::<Event>(&json).unwrap(), supervisor);
}
