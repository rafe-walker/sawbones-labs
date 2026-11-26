//! #214: the guard agrees with the policy engine and the encoder on a
//! signal's register width. A lease approves one action, so a frame with
//! another register count never passes, and an allow-list rule sized to the
//! signal's width blocks the wider frame before the lease is looked at.

mod common;

use common::{grant_action, replace, rule, write_multiple, write_single};
use sawbones_busguard::{
    BlockReason, BusGuard, BusWriteAction, Function, Port, Verdict, WriteFunction,
};

fn guard_with(rules: &[sawbones_busguard::Rule]) -> BusGuard {
    let mut guard = BusGuard::new();
    guard.apply(replace(rules), 0).unwrap();
    guard
}

fn wide_rules() -> Vec<sawbones_busguard::Rule> {
    vec![
        rule(0, 1, Function::WriteSingleRegister, 0, 0xFFFF, 1),
        rule(0, 1, Function::WriteMultipleRegisters, 0, 0xFFFF, 4),
    ]
}

#[test]
fn regression_214_a_two_register_frame_never_spends_a_one_register_lease() {
    // The approved action: one register (width 1) at 0x0100.
    let action =
        BusWriteAction::new_with_width(0, 1, WriteFunction::WriteSingleRegister, 0x0100, 1, &[7])
            .unwrap();
    let approved = write_single(1, 0x0100, 7);
    assert_eq!(action.frame().as_bytes(), approved);
    // Frames that also write the next register, or write one register by the
    // multiple-register function.
    for frame in [
        write_multiple(1, 0x0100, &[7, 0]),
        write_multiple(1, 0x0100, &[7, 8]),
        write_multiple(1, 0x0100, &[7]),
        write_multiple(1, 0x00FF, &[0, 7]),
    ] {
        let mut g = guard_with(&wide_rules());
        g.apply(grant_action(action), 1).unwrap();
        assert_eq!(
            g.decide(Port(0), &frame, 2),
            Verdict::Block(BlockReason::DigestMismatch),
            "{frame:02X?}"
        );
        assert!(g.lease().is_some(), "a refusal keeps the lease");
        assert_eq!(g.decide(Port(0), &approved, 2), Verdict::Pass);
    }
}

#[test]
fn regression_214_a_one_register_frame_never_spends_a_two_register_lease() {
    let action = BusWriteAction::new_with_width(
        0,
        1,
        WriteFunction::WriteMultipleRegisters,
        0x0100,
        2,
        &[0x1234, 0x5678],
    )
    .unwrap();
    let approved = write_multiple(1, 0x0100, &[0x1234, 0x5678]);
    assert_eq!(action.frame().as_bytes(), approved);
    // Only the high word, only the low word, or a stale low word.
    for frame in [
        write_single(1, 0x0100, 0x1234),
        write_single(1, 0x0101, 0x5678),
        write_multiple(1, 0x0100, &[0x1234]),
        write_multiple(1, 0x0100, &[0x1234, 0]),
    ] {
        let mut g = guard_with(&wide_rules());
        g.apply(grant_action(action), 1).unwrap();
        assert_eq!(
            g.decide(Port(0), &frame, 2),
            Verdict::Block(BlockReason::DigestMismatch),
            "{frame:02X?}"
        );
        assert_eq!(g.decide(Port(0), &approved, 2), Verdict::Pass);
    }
}

#[test]
fn regression_214_a_rule_sized_to_the_width_blocks_a_wider_frame_outright() {
    // The same width the entry states, as the rule's most registers: even
    // with a lease for the wider frame, it is not allow-listed.
    let rules = [rule(
        0,
        1,
        Function::WriteMultipleRegisters,
        0x0100,
        0x0100,
        1,
    )];
    let wide = write_multiple(1, 0x0100, &[7, 8]);
    let action =
        BusWriteAction::new(0, 1, WriteFunction::WriteMultipleRegisters, 0x0100, &[7, 8]).unwrap();
    let mut g = guard_with(&rules);
    g.apply(grant_action(action), 1).unwrap();
    assert_eq!(
        g.decide(Port(0), &wide, 2),
        Verdict::Block(BlockReason::NotAllowListed)
    );
}
