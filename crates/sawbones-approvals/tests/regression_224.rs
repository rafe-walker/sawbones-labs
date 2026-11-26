//! #224: mutation testing found that nothing checked `GrantLease::is_consistent`
//! on a lease that is consistent, or on one that is not. The payload-length
//! constant is covered by a unit test inside the crate (it is private).

use sawbones_approvals::{BusWriteAction, Digest, GrantLease, WriteFunction};

fn action(register: u16) -> BusWriteAction {
    BusWriteAction::new(
        2,
        1,
        WriteFunction::WriteMultipleRegisters,
        register,
        &[1, 2],
    )
    .unwrap()
}

#[test]
fn regression_224_a_lease_built_by_new_is_consistent() {
    // Kills `is_consistent -> false`.
    assert!(GrantLease::new(action(0x22)).is_consistent());
}

#[test]
fn regression_224_a_lease_with_a_foreign_digest_is_inconsistent() {
    // Kills `is_consistent -> true`.
    let lease = GrantLease {
        action: action(0x22),
        action_digest: Digest([0x22; 32]),
    };
    assert!(!lease.is_consistent());
}

#[test]
fn regression_224_a_lease_with_another_actions_digest_is_inconsistent() {
    // Kills `== -> !=` together with the consistent case above: a digest of a
    // different action, and a one-bit change of the right digest, both fail.
    let mut lease = GrantLease::new(action(0x22));
    lease.action_digest = GrantLease::new(action(0x23)).action_digest;
    assert!(!lease.is_consistent());

    let mut lease = GrantLease::new(action(0x22));
    lease.action_digest.0[31] ^= 1;
    assert!(!lease.is_consistent());
}
