//! The verified path (#172): a lease or an allow-list reaches the guard only
//! as an approval the trust root signed, checked by a verifier.

mod common;

use std::fs;
use std::path::Path;

use common::{allow_list, read, rule, write_single};
use sawbones_approvals::test_keys::{sign, verifier, OWNER_PHONE_SEED, TRUST_ROOT_SEED};
use sawbones_approvals::{
    Approval, BusGuardAllowList, DeviceId, Digest, GrantLease, Message, Refusal, Role, Verifier,
    MESSAGE_LEN,
};
use sawbones_busguard::{
    AllowList, AllowListMismatch, BlockReason, BusGuard, Decision, FrameDigest, Function::*, Port,
    TrustRootEvent, ALLOW_LIST_DOMAIN, ALLOW_LIST_ENCODED_LEN,
};

const GUARD: DeviceId = DeviceId(3);

fn signed(
    seed: &[u8; 32],
    counter: u64,
    approval: Approval,
) -> ([u8; MESSAGE_LEN], sawbones_approvals::Signature) {
    let bytes = Message {
        target: GUARD,
        signer: if *seed == OWNER_PHONE_SEED {
            Role::OwnerPhone
        } else {
            Role::TrustRoot
        },
        counter,
        expires_at: 1_000,
        approval,
    }
    .encode();
    let signature = sign(seed, &bytes);
    (bytes, signature)
}

fn lease_for(frame: &[u8], port: u8) -> Approval {
    Approval::GrantLease(GrantLease {
        port,
        action: Digest(FrameDigest::of(frame).0),
    })
}

fn list_approval(list: &AllowList) -> Approval {
    Approval::BusGuardAllowList(BusGuardAllowList {
        allow_list: Digest(list.digest()),
    })
}

/// Installs `list` through the verified path.
fn install(guard: &mut BusGuard, verifier: &mut Verifier, counter: u64, list: AllowList) {
    let (bytes, signature) = signed(&TRUST_ROOT_SEED, counter, list_approval(&list));
    let approved = verifier.verify(&bytes, &signature, 0).unwrap();
    let event = TrustRootEvent::from_approved_allow_list(approved, list).unwrap();
    guard.apply(event, 0).unwrap();
}

#[test]
fn a_signed_allow_list_and_lease_let_exactly_the_approved_write_through_once() {
    let mut guard = BusGuard::new();
    let mut verifier = verifier(GUARD);
    let list = allow_list(&[rule(0, 1, WriteSingleRegister, 100, 110, 1)]);
    install(&mut guard, &mut verifier, 1, list);
    assert_eq!(guard.allow_list(), &list);

    let frame = write_single(1, 105, 42);
    let (bytes, signature) = signed(&TRUST_ROOT_SEED, 2, lease_for(&frame, 0));
    let approved = verifier.verify(&bytes, &signature, 1).unwrap();
    guard
        .apply(TrustRootEvent::from_approved_lease(approved), 1)
        .unwrap();

    let other = write_single(1, 105, 43);
    assert_eq!(
        guard.decide(Port(0), &other, 2),
        Decision::Block(BlockReason::DigestMismatch)
    );
    assert_eq!(guard.decide(Port(0), &frame, 2), Decision::Pass);
    assert_eq!(
        guard.decide(Port(0), &frame, 3),
        Decision::Block(BlockReason::NoLease)
    );

    // Sending the same signed grant again is a replay: no second lease.
    assert_eq!(
        verifier.verify::<GrantLease>(&bytes, &signature, 4),
        Err(Refusal::Replayed { last: 2 })
    );
}

#[test]
fn an_owner_phone_cannot_grant_a_lease_or_change_the_allow_list() {
    let mut verifier = verifier(GUARD);
    let frame = write_single(1, 105, 42);
    let (bytes, signature) = signed(&OWNER_PHONE_SEED, 1, lease_for(&frame, 0));
    assert_eq!(
        verifier.verify::<GrantLease>(&bytes, &signature, 0),
        Err(Refusal::RoleNotAllowed)
    );
    let list = allow_list(&[rule(0, 1, ReadHoldingRegisters, 0, 9, 10)]);
    let (bytes, signature) = signed(&OWNER_PHONE_SEED, 2, list_approval(&list));
    assert_eq!(
        verifier.verify::<BusGuardAllowList>(&bytes, &signature, 0),
        Err(Refusal::RoleNotAllowed)
    );
}

#[test]
fn an_approval_for_one_list_does_not_install_another() {
    let mut verifier = verifier(GUARD);
    let approved_list = allow_list(&[rule(0, 1, ReadHoldingRegisters, 0, 9, 10)]);
    let wider = allow_list(&[rule(0, 1, ReadHoldingRegisters, 0, 99, 10)]);
    let (bytes, signature) = signed(&TRUST_ROOT_SEED, 1, list_approval(&approved_list));
    let approved = verifier.verify(&bytes, &signature, 0).unwrap();
    assert_eq!(
        TrustRootEvent::from_approved_allow_list(approved, wider).unwrap_err(),
        AllowListMismatch
    );
}

#[test]
fn an_approval_for_another_guard_is_refused() {
    let mut other = verifier(DeviceId(GUARD.0 + 1));
    let frame = write_single(1, 105, 42);
    let (bytes, signature) = signed(&TRUST_ROOT_SEED, 1, lease_for(&frame, 0));
    assert_eq!(
        other.verify::<GrantLease>(&bytes, &signature, 0),
        Err(Refusal::WrongDevice)
    );
}

#[test]
fn the_lease_carries_the_approved_port() {
    let mut guard = BusGuard::new();
    let mut verifier = verifier(GUARD);
    let list = allow_list(&[
        rule(0, 1, WriteSingleRegister, 100, 110, 1),
        rule(1, 1, WriteSingleRegister, 100, 110, 1),
    ]);
    install(&mut guard, &mut verifier, 1, list);
    let frame = write_single(1, 100, 1);
    let (bytes, signature) = signed(&TRUST_ROOT_SEED, 2, lease_for(&frame, 1));
    let approved = verifier.verify(&bytes, &signature, 0).unwrap();
    guard
        .apply(TrustRootEvent::from_approved_lease(approved), 0)
        .unwrap();
    assert_eq!(
        guard.decide(Port(0), &frame, 1),
        Decision::Block(BlockReason::LeaseOnOtherPort)
    );
    assert_eq!(guard.decide(Port(1), &frame, 1), Decision::Pass);
}

#[test]
fn the_allow_list_encoding_and_digest_are_pinned() {
    assert_eq!(ALLOW_LIST_ENCODED_LEN, 1 + 32 * 9);
    assert_eq!(ALLOW_LIST_DOMAIN, b"sawbones-busguard/allow-list/v1\0");
    let list = allow_list(&[
        rule(2, 0x11, WriteMultipleRegisters, 0x0102, 0x0304, 0x0506),
        rule(0, 1, ReadInputRegisters, 0, 0xFFFF, 125),
    ]);
    let encoded = list.encode();
    assert_eq!(
        encoded[..19],
        [
            2, // rules
            2, 0x11, 0x10, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, // first
            0, 1, 0x04, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x7D, // second
        ]
    );
    assert!(encoded[19..].iter().all(|&b| b == 0));
    assert_eq!(AllowList::new().encode(), [0; ALLOW_LIST_ENCODED_LEN]);
    let hex: String = list.digest().iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hex, GOLDEN_ALLOW_LIST_DIGEST);
}

const GOLDEN_ALLOW_LIST_DIGEST: &str =
    "7634e5043c7b41f58331576c202292668b79e319bf7dcce140d631e359470ae1";

#[test]
fn the_digest_depends_on_every_rule_field_and_the_order() {
    let base = rule(1, 2, WriteSingleRegister, 3, 4, 1);
    let variants = [
        rule(9, 2, WriteSingleRegister, 3, 4, 1),
        rule(1, 9, WriteSingleRegister, 3, 4, 1),
        rule(1, 2, ReadHoldingRegisters, 3, 4, 1),
        rule(1, 2, WriteSingleRegister, 2, 4, 1),
        rule(1, 2, WriteSingleRegister, 3, 9, 1),
        rule(1, 2, WriteSingleRegister, 3, 4, 2),
    ];
    let digest = allow_list(&[base]).digest();
    for variant in variants {
        assert_ne!(allow_list(&[variant]).digest(), digest, "{variant:?}");
    }
    let other = rule(1, 2, ReadInputRegisters, 0, 9, 10);
    assert_ne!(
        allow_list(&[base, other]).digest(),
        allow_list(&[other, base]).digest()
    );
    assert_ne!(allow_list(&[]).digest(), digest);
    // A read is allowed by a list installed through the verified path.
    let mut guard = BusGuard::new();
    let mut verifier = verifier(GUARD);
    install(&mut guard, &mut verifier, 1, allow_list(&[other]));
    assert_eq!(
        guard.decide(Port(1), &read(2, 0x04, 0, 9), 0),
        Decision::Pass
    );
}

/// The unverified constructor is for tests only: it is compiled only with
/// the `test-unverified` feature, the feature is off by default, and no
/// crate in the workspace turns it on outside its dev-dependencies.
#[test]
fn the_unverified_constructor_is_test_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lib = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    let at = lib
        .find("pub const fn unverified_from_trust_root")
        .expect("the constructor");
    let before = &lib[..at];
    let attributes = &before[before.rfind("///").unwrap()..];
    assert!(
        attributes.contains("#[cfg(feature = \"test-unverified\")]"),
        "unverified_from_trust_root must be gated on test-unverified"
    );
    assert_eq!(lib.matches("fn unverified_from_trust_root").count(), 1);

    let crates = root.parent().unwrap();
    for entry in fs::read_dir(crates).unwrap() {
        let manifest = entry.unwrap().path().join("Cargo.toml");
        let Ok(text) = fs::read_to_string(&manifest) else {
            continue;
        };
        let normal = text.split("[dev-dependencies]").next().unwrap();
        let features = normal.split("[dependencies]").next().unwrap();
        let deps = normal.split("[dependencies]").nth(1).unwrap_or("");
        assert!(
            !deps.contains("test-unverified"),
            "{} turns on test-unverified in a normal dependency",
            manifest.display()
        );
        if let Some(line) = features.lines().find(|l| l.starts_with("default")) {
            assert!(!line.contains("test-unverified"), "{}", manifest.display());
        }
    }
}
