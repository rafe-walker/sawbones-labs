//! The plain data types survive a trip through JSON, so the supervisor can log
//! decisions and ship allow-lists. A lease is reported, never loaded.
#![cfg(feature = "serde")]

mod common;

use common::{allow_list, rule};
use sawbones_busguard::{
    AllowList, BlockReason, BusGuard, BusWriteAction, FrameError, Function, LeaseGrant, Port,
    TrustRootEvent, Update, Verdict, WriteFunction,
};

#[test]
fn verdicts_and_allow_lists_round_trip_through_json() {
    let verdict = Verdict::Block(BlockReason::Frame(FrameError::UnsupportedFunction(5)));
    let json = serde_json::to_string(&verdict).unwrap();
    assert_eq!(json, r#"{"block":{"frame":{"unsupported_function":5}}}"#);
    assert_eq!(serde_json::from_str::<Verdict>(&json).unwrap(), verdict);
    assert_eq!(serde_json::to_string(&Verdict::Pass).unwrap(), r#""pass""#);
    assert_eq!(
        serde_json::to_string(&Verdict::Block(BlockReason::OtherMasterSeen)).unwrap(),
        r#"{"block":"other_master_seen"}"#
    );

    let list = allow_list(&[rule(0, 1, Function::WriteSingleRegister, 16, 16, 1)]);
    let json = serde_json::to_string(&list).unwrap();
    assert_eq!(serde_json::from_str::<AllowList>(&json).unwrap(), list);
}

#[test]
fn a_lease_serialises_with_its_action() {
    let action = BusWriteAction::new(1, 2, WriteFunction::WriteSingleRegister, 0x10, &[3]).unwrap();
    let mut guard = BusGuard::new();
    guard
        .apply(
            TrustRootEvent::unverified_from_trust_root(Update::GrantLease(LeaseGrant::for_action(
                action,
            ))),
            0,
        )
        .unwrap();
    let lease = guard.lease().unwrap();
    assert_eq!(lease.port(), Port(1));
    let v = serde_json::to_value(lease).unwrap();
    assert_eq!(
        v["action"],
        serde_json::json!({
            "port": 1, "unit": 2, "function": "write_single_register",
            "register": 16, "values": [3]
        })
    );
    assert_eq!(v["action_digest"].as_str().unwrap().len(), 64);
    assert_eq!(v["expires_at"], 100);
}
