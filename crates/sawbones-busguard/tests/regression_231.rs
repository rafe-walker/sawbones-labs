//! #231: tests for mutants the weekly mutation run found missed. Each test
//! fails with the named mutant applied.

mod common;

use common::{allow_list, read, replace, rule};
use sawbones_busguard::{
    parse, AllowList, BlockReason, BusGuard, FrameError, Function, Port, Verdict, MAX_FRAME_LEN,
};

/// Mutants: `>` with `>=` at the length cap in `parse` and in `decide`. A frame
/// of exactly `MAX_FRAME_LEN` bytes is not too long; one byte more is.
#[test]
fn regression_231_busguard_length_cap_is_inclusive() {
    // A write multiple registers header with the largest byte count that makes
    // the frame exactly 256 bytes. It is not a valid frame (odd byte count and
    // no CRC), but it is not too long, so it must reach the later checks.
    let mut at_cap = vec![0u8; MAX_FRAME_LEN];
    at_cap[0] = 1;
    at_cap[1] = 0x10;
    at_cap[6] = u8::try_from(MAX_FRAME_LEN - 9).unwrap();
    assert_ne!(parse(&at_cap).err(), Some(FrameError::TooLong));
    let over = [at_cap.clone(), vec![0]].concat();
    assert_eq!(parse(&over).err(), Some(FrameError::TooLong));

    let mut guard = BusGuard::new();
    guard
        .apply(
            replace(&[rule(0, 1, Function::ReadHoldingRegisters, 0, 9, 1)]),
            0,
        )
        .unwrap();
    assert_ne!(
        guard.decide(Port(0), &at_cap, 1),
        Verdict::Block(BlockReason::Frame(FrameError::TooLong))
    );
    assert_eq!(
        guard.decide(Port(0), &over, 1),
        Verdict::Block(BlockReason::Frame(FrameError::TooLong))
    );
}

/// Mutant: `<` with `<=` at the minimum length in `parse`. Four bytes are
/// enough to read the function code, so an unsupported code is named as such;
/// three bytes are truncated.
#[test]
fn regression_231_busguard_four_bytes_reach_the_function_check() {
    assert_eq!(
        parse(&[1, 0x99, 0, 0]).err(),
        Some(FrameError::UnsupportedFunction(0x99))
    );
    assert_eq!(parse(&[1, 0x99, 0]).err(), Some(FrameError::Truncated));
}

/// Mutant: `AllowList::is_empty -> true`.
#[test]
fn regression_231_busguard_a_list_with_rules_is_not_empty() {
    assert!(AllowList::new().is_empty());
    let list = allow_list(&[rule(0, 1, Function::ReadHoldingRegisters, 0, 9, 1)]);
    assert!(!list.is_empty());
    assert_eq!(list.len(), 1);
}

/// Mutants: `Verdict::is_pass -> true / false` and
/// `<Decision as PartialEq<Verdict>>::eq -> true`.
#[test]
fn regression_231_busguard_verdict_and_decision_compare_truthfully() {
    assert!(Verdict::Pass.is_pass());
    assert!(!Verdict::Block(BlockReason::NotAllowListed).is_pass());

    let mut guard = BusGuard::new();
    guard
        .apply(
            replace(&[rule(0, 1, Function::ReadHoldingRegisters, 0, 9, 1)]),
            0,
        )
        .unwrap();
    let passed = guard.decide(Port(0), &read(1, 3, 0, 1), 1);
    let blocked = guard.decide(Port(0), &read(1, 3, 100, 1), 1);
    assert!(passed == Verdict::Pass);
    assert!(passed != Verdict::Block(BlockReason::NotAllowListed));
    assert!(blocked != Verdict::Pass);
    assert!(blocked == Verdict::Block(BlockReason::NotAllowListed));
    assert!(blocked != Verdict::Block(BlockReason::NoLease));
}
