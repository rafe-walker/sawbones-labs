//! The plain data types survive a trip through JSON, so the supervisor can log
//! decisions and ship allow-lists.
#![cfg(feature = "serde")]

mod common;

use common::{allow_list, rule};
use sawbones_busguard::{
    AllowList, BlockReason, Decision, FrameDigest, FrameError, Function, Lease, Port,
};

#[test]
fn decisions_and_allow_lists_round_trip_through_json() {
    let decision = Decision::Block(BlockReason::Frame(FrameError::UnsupportedFunction(5)));
    let json = serde_json::to_string(&decision).unwrap();
    assert_eq!(json, r#"{"block":{"frame":{"unsupported_function":5}}}"#);
    assert_eq!(serde_json::from_str::<Decision>(&json).unwrap(), decision);
    assert_eq!(serde_json::to_string(&Decision::Pass).unwrap(), r#""pass""#);

    let list = allow_list(&[rule(0, 1, Function::WriteSingleRegister, 16, 16, 1)]);
    let json = serde_json::to_string(&list).unwrap();
    assert_eq!(serde_json::from_str::<AllowList>(&json).unwrap(), list);

    let lease = Lease {
        port: Port(1),
        digest: FrameDigest::of(b"frame"),
        expires_at: 7,
    };
    let json = serde_json::to_string(&lease).unwrap();
    assert_eq!(serde_json::from_str::<Lease>(&json).unwrap(), lease);
}
