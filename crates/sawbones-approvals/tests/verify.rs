//! What the verifier accepts and what it refuses, and that a refusal changes
//! nothing.

mod common;

use common::{
    every_approval, lease, lease_grant, message, seed_for, signed, test_verifier, DEVICE,
};
use sawbones_approvals::test_keys::{
    public_key, sign, MODULE_SEED, NOT_ENROLLED_SEED, OWNER_PHONE_SEED, TEST_MODULE,
    TRUST_ROOT_SEED,
};
use sawbones_approvals::{
    may_approve, Approval, BusGuardAllowList, DecodeError, DeviceId, Digest, GrantLease,
    InstallCoreRelease, KeyError, Message, ModuleId, NativeInstall, OperatorAttestation,
    PluginPermissionGrowth, PublicKey, Refusal, Role, Signature, UnknownSigner, Verifier,
    MAX_MODULES, MESSAGE_LEN, TEST_PUBLIC_KEYS,
};

#[test]
fn a_valid_trust_root_lease_passes_and_carries_what_was_signed() {
    let mut verifier = test_verifier();
    let (bytes, signature) = signed(&message(Role::TrustRoot, 1, lease(3, 0xAB)));
    let approved = verifier
        .verify::<GrantLease>(&bytes, &signature, 999)
        .unwrap();
    assert_eq!(*approved.approval(), lease_grant(3, 0xAB));
    assert_eq!(approved.signer(), Role::TrustRoot);
    assert_eq!(approved.counter(), 1);
    assert_eq!(approved.expires_at(), 1000);
    assert_eq!(verifier.last_counter(Role::TrustRoot), Some(1));
}

#[test]
fn every_kind_passes_from_every_role_the_policy_allows_and_no_other() {
    let roles = [Role::TrustRoot, Role::OwnerPhone, Role::Module(TEST_MODULE)];
    for approval in every_approval() {
        for role in roles {
            let mut verifier = test_verifier();
            let (bytes, signature) = signed(&message(role, 1, approval));
            let result = verifier.verify::<Approval>(&bytes, &signature, 0);
            if may_approve(role, &approval) {
                assert_eq!(result.unwrap().into_approval(), approval);
            } else {
                assert_eq!(
                    result,
                    Err(Refusal::RoleNotAllowed),
                    "{role:?} {approval:?}"
                );
                assert_eq!(verifier.last_counter(role), Some(0));
            }
        }
    }
}

#[test]
fn the_policy_table() {
    let [core, lease, allow_list, plugin, native, attest] = every_approval();
    for approval in [core, lease, allow_list, plugin, native, attest] {
        assert!(may_approve(Role::TrustRoot, &approval));
        assert!(!may_approve(Role::Module(TEST_MODULE), &approval));
    }
    assert!(lease.is_energy() && allow_list.is_energy());
    assert!(!core.is_energy() && !plugin.is_energy() && !native.is_energy());
    assert!(!attest.is_energy());
    for needs_panel in [core, lease, allow_list] {
        assert!(needs_panel.needs_trust_root());
        assert!(!may_approve(Role::OwnerPhone, &needs_panel));
    }
    for phone_ok in [plugin, native, attest] {
        assert!(may_approve(Role::OwnerPhone, &phone_ok));
    }
}

#[test]
fn flipping_any_bit_of_the_message_or_the_signature_fails() {
    let (bytes, signature) = signed(&message(Role::TrustRoot, 1, lease(0, 1)));
    for byte in 0..MESSAGE_LEN {
        for bit in 0..8 {
            let mut changed = bytes;
            changed[byte] ^= 1 << bit;
            let mut verifier = test_verifier();
            let result = verifier.verify::<Approval>(&changed, &signature, 0);
            assert!(result.is_err(), "message byte {byte} bit {bit}");
            assert_eq!(verifier.last_counter(Role::TrustRoot), Some(0));
        }
    }
    for byte in 0..64 {
        for bit in 0..8 {
            let mut changed = signature;
            changed.0[byte] ^= 1 << bit;
            let mut verifier = test_verifier();
            assert_eq!(
                verifier.verify::<Approval>(&bytes, &changed, 0),
                Err(Refusal::BadSignature),
                "signature byte {byte} bit {bit}"
            );
        }
    }
}

#[test]
fn an_owner_phone_lease_is_refused_even_with_a_valid_phone_signature() {
    let mut verifier = test_verifier();
    let (bytes, signature) = signed(&message(Role::OwnerPhone, 1, lease(0, 1)));
    assert_eq!(
        verifier.verify::<GrantLease>(&bytes, &signature, 0),
        Err(Refusal::RoleNotAllowed)
    );
    assert_eq!(verifier.last_counter(Role::OwnerPhone), Some(0));
}

#[test]
fn energy_kinds_from_the_wrong_role_are_refused() {
    let allow_list = Approval::BusGuardAllowList(BusGuardAllowList {
        allow_list: Digest([9; 32]),
    });
    for approval in [lease(1, 2), allow_list] {
        for role in [Role::OwnerPhone, Role::Module(TEST_MODULE)] {
            let mut verifier = test_verifier();
            let (bytes, signature) = signed(&message(role, 1, approval));
            assert_eq!(
                verifier.verify::<Approval>(&bytes, &signature, 0),
                Err(Refusal::RoleNotAllowed)
            );
        }
    }
}

#[test]
fn a_core_release_needs_the_trust_root() {
    let core = Approval::InstallCoreRelease(InstallCoreRelease {
        release: Digest([1; 32]),
    });
    let mut verifier = test_verifier();
    let (bytes, signature) = signed(&message(Role::OwnerPhone, 1, core));
    assert_eq!(
        verifier.verify::<InstallCoreRelease>(&bytes, &signature, 0),
        Err(Refusal::RoleNotAllowed)
    );
    let (bytes, signature) = signed(&message(Role::TrustRoot, 1, core));
    assert!(verifier
        .verify::<InstallCoreRelease>(&bytes, &signature, 0)
        .is_ok());
}

#[test]
fn a_trust_root_role_signed_with_another_key_is_refused() {
    let mut verifier = test_verifier();
    let bytes = message(Role::TrustRoot, 1, lease(0, 1)).encode();
    for seed in [OWNER_PHONE_SEED, MODULE_SEED, NOT_ENROLLED_SEED] {
        let signature = sign(&seed, &bytes);
        assert_eq!(
            verifier.verify::<GrantLease>(&bytes, &signature, 0),
            Err(Refusal::BadSignature)
        );
    }
    assert_eq!(verifier.last_counter(Role::TrustRoot), Some(0));
}

#[test]
fn a_replayed_counter_is_refused() {
    let mut verifier = test_verifier();
    let (bytes, signature) = signed(&message(Role::TrustRoot, 5, lease(0, 1)));
    assert!(verifier.verify::<GrantLease>(&bytes, &signature, 0).is_ok());
    assert_eq!(
        verifier.verify::<GrantLease>(&bytes, &signature, 1),
        Err(Refusal::Replayed { last: 5 })
    );
}

#[test]
fn a_lower_or_equal_counter_is_refused_after_a_higher_one() {
    let mut verifier = test_verifier();
    let (bytes, signature) = signed(&message(Role::TrustRoot, 10, lease(0, 1)));
    assert!(verifier.verify::<GrantLease>(&bytes, &signature, 0).is_ok());
    for counter in [0, 1, 9, 10] {
        let (bytes, signature) = signed(&message(Role::TrustRoot, counter, lease(0, 2)));
        assert_eq!(
            verifier.verify::<GrantLease>(&bytes, &signature, 0),
            Err(Refusal::Replayed { last: 10 })
        );
    }
    let (bytes, signature) = signed(&message(Role::TrustRoot, 11, lease(0, 2)));
    assert!(verifier.verify::<GrantLease>(&bytes, &signature, 0).is_ok());
}

#[test]
fn counter_zero_is_never_accepted() {
    let mut verifier = test_verifier();
    let (bytes, signature) = signed(&message(Role::TrustRoot, 0, lease(0, 1)));
    assert_eq!(
        verifier.verify::<GrantLease>(&bytes, &signature, 0),
        Err(Refusal::Replayed { last: 0 })
    );
}

#[test]
fn counters_are_kept_per_signer() {
    let mut verifier = test_verifier();
    let attest = Approval::OperatorAttestation(OperatorAttestation {
        fact: Digest([1; 32]),
    });
    let (bytes, signature) = signed(&message(Role::TrustRoot, 50, attest));
    assert!(verifier.verify::<Approval>(&bytes, &signature, 0).is_ok());
    let (bytes, signature) = signed(&message(Role::OwnerPhone, 1, attest));
    assert!(verifier.verify::<Approval>(&bytes, &signature, 0).is_ok());
    assert_eq!(verifier.last_counter(Role::TrustRoot), Some(50));
    assert_eq!(verifier.last_counter(Role::OwnerPhone), Some(1));
    assert_eq!(verifier.last_counter(Role::Module(TEST_MODULE)), Some(0));
    assert_eq!(verifier.last_counter(Role::Module(ModuleId(99))), None);
}

#[test]
fn an_expired_approval_is_refused_at_and_after_its_expiry() {
    let (bytes, signature) = signed(&message(Role::TrustRoot, 1, lease(0, 1)));
    for now in [1000, 1001, u64::MAX] {
        let mut verifier = test_verifier();
        assert_eq!(
            verifier.verify::<GrantLease>(&bytes, &signature, now),
            Err(Refusal::Expired)
        );
        assert_eq!(verifier.last_counter(Role::TrustRoot), Some(0));
    }
    let mut verifier = test_verifier();
    assert!(verifier
        .verify::<GrantLease>(&bytes, &signature, 999)
        .is_ok());
}

#[test]
fn a_refused_approval_does_not_burn_its_counter() {
    let mut verifier = test_verifier();
    // Expired, with a high counter: refused, and the counter is not taken.
    let (bytes, signature) = signed(&message(Role::TrustRoot, 100, lease(0, 1)));
    assert_eq!(
        verifier.verify::<GrantLease>(&bytes, &signature, 5000),
        Err(Refusal::Expired)
    );
    // A forged one with a high counter: refused, not taken.
    let forged = message(Role::TrustRoot, 200, lease(0, 1)).encode();
    let bad = sign(&NOT_ENROLLED_SEED, &forged);
    assert_eq!(
        verifier.verify::<GrantLease>(&forged, &bad, 0),
        Err(Refusal::BadSignature)
    );
    assert_eq!(verifier.last_counter(Role::TrustRoot), Some(0));
    let (bytes, signature) = signed(&message(Role::TrustRoot, 1, lease(0, 1)));
    assert!(verifier.verify::<GrantLease>(&bytes, &signature, 0).is_ok());
}

#[test]
fn an_approval_for_another_device_is_refused() {
    let mut verifier = test_verifier();
    let mut other = message(Role::TrustRoot, 1, lease(0, 1));
    other.target = DeviceId(DEVICE.0 + 1);
    let (bytes, signature) = signed(&other);
    assert_eq!(
        verifier.verify::<GrantLease>(&bytes, &signature, 0),
        Err(Refusal::WrongDevice)
    );
}

#[test]
fn asking_for_one_kind_refuses_another() {
    let mut verifier = test_verifier();
    let (bytes, signature) = signed(&message(Role::TrustRoot, 1, lease(0, 1)));
    assert_eq!(
        verifier.verify::<BusGuardAllowList>(&bytes, &signature, 0),
        Err(Refusal::WrongKind)
    );
    assert_eq!(
        verifier.verify::<NativeInstall>(&bytes, &signature, 0),
        Err(Refusal::WrongKind)
    );
    assert_eq!(
        verifier.verify::<PluginPermissionGrowth>(&bytes, &signature, 0),
        Err(Refusal::WrongKind)
    );
    assert_eq!(verifier.last_counter(Role::TrustRoot), Some(0));
    assert!(verifier.verify::<GrantLease>(&bytes, &signature, 0).is_ok());
}

#[test]
fn a_signer_without_a_key_is_refused() {
    let attest = Approval::OperatorAttestation(OperatorAttestation {
        fact: Digest([1; 32]),
    });
    // No phone enrolled.
    let mut verifier = Verifier::new(DEVICE, public_key(&TRUST_ROOT_SEED)).unwrap();
    let (bytes, signature) = signed(&message(Role::OwnerPhone, 1, attest));
    assert_eq!(
        verifier.verify::<Approval>(&bytes, &signature, 0),
        Err(Refusal::UnknownSigner)
    );
    // An unknown module: modules approve nothing, so the role check refuses
    // it before the key lookup.
    let mut verifier = test_verifier();
    let bytes = message(Role::Module(ModuleId(42)), 1, attest).encode();
    let signature = sign(&seed_for(Role::Module(TEST_MODULE)), &bytes);
    assert_eq!(
        verifier.verify::<Approval>(&bytes, &signature, 0),
        Err(Refusal::RoleNotAllowed)
    );
}

#[test]
fn malformed_bytes_are_refused_with_the_reason() {
    let mut verifier = test_verifier();
    let signature = Signature([0; 64]);
    assert_eq!(
        verifier.verify::<Approval>(&[0; MESSAGE_LEN - 1], &signature, 0),
        Err(Refusal::Malformed(DecodeError::Length))
    );
    assert_eq!(
        verifier.verify::<Approval>(&[0; MESSAGE_LEN + 1], &signature, 0),
        Err(Refusal::Malformed(DecodeError::Length))
    );
    assert_eq!(
        verifier.verify::<Approval>(&[0; MESSAGE_LEN], &signature, 0),
        Err(Refusal::Malformed(DecodeError::Version))
    );
}

#[test]
fn restore_counter_only_raises() {
    let mut verifier = test_verifier();
    verifier.restore_counter(Role::TrustRoot, 20).unwrap();
    verifier.restore_counter(Role::TrustRoot, 3).unwrap();
    assert_eq!(verifier.last_counter(Role::TrustRoot), Some(20));
    assert_eq!(
        verifier.restore_counter(Role::Module(ModuleId(42)), 1),
        Err(UnknownSigner)
    );
    let (bytes, signature) = signed(&message(Role::TrustRoot, 20, lease(0, 1)));
    assert_eq!(
        verifier.verify::<GrantLease>(&bytes, &signature, 0),
        Err(Refusal::Replayed { last: 20 })
    );
}

#[test]
fn a_key_cannot_serve_two_roles() {
    let mut verifier = Verifier::new(DEVICE, public_key(&TRUST_ROOT_SEED)).unwrap();
    assert_eq!(
        verifier.enroll_owner_phone(public_key(&TRUST_ROOT_SEED)),
        Err(KeyError::Reused)
    );
    verifier
        .enroll_owner_phone(public_key(&OWNER_PHONE_SEED))
        .unwrap();
    assert_eq!(
        verifier.enroll_owner_phone(public_key(&MODULE_SEED)),
        Err(KeyError::AlreadyEnrolled)
    );
    assert_eq!(
        verifier.enroll_module(ModuleId(1), public_key(&OWNER_PHONE_SEED)),
        Err(KeyError::Reused)
    );
    verifier
        .enroll_module(ModuleId(1), public_key(&MODULE_SEED))
        .unwrap();
    assert_eq!(
        verifier.enroll_module(ModuleId(1), public_key(&NOT_ENROLLED_SEED)),
        Err(KeyError::AlreadyEnrolled)
    );
}

#[test]
fn the_module_table_has_a_fixed_size() {
    let mut verifier = Verifier::new(DEVICE, public_key(&TRUST_ROOT_SEED)).unwrap();
    for i in 0..MAX_MODULES {
        let mut seed = [0x5A; 32];
        seed[0] = u8::try_from(i).unwrap();
        let id = ModuleId(u32::try_from(i).unwrap());
        verifier.enroll_module(id, public_key(&seed)).unwrap();
    }
    assert_eq!(
        verifier.enroll_module(ModuleId(1000), public_key(&NOT_ENROLLED_SEED)),
        Err(KeyError::Full)
    );
}

#[test]
fn weak_and_invalid_keys_are_refused() {
    // The identity point: small order.
    let mut identity = [0; 32];
    identity[0] = 1;
    assert_eq!(
        Verifier::new(DEVICE, PublicKey(identity)).unwrap_err(),
        KeyError::Weak
    );
    // y = 2 is not on the curve.
    let mut off_curve = [0; 32];
    off_curve[0] = 2;
    assert_eq!(
        Verifier::new(DEVICE, PublicKey(off_curve)).unwrap_err(),
        KeyError::Invalid
    );
}

#[test]
fn the_test_public_keys_are_the_test_seeds_and_are_flagged() {
    use sawbones_approvals::test_keys::ALL_SEEDS;
    for (seed, public) in ALL_SEEDS.iter().zip(TEST_PUBLIC_KEYS) {
        assert_eq!(public_key(seed).0, public);
        assert!(public_key(seed).is_test_key());
    }
    let mut seed = TRUST_ROOT_SEED;
    seed[31] ^= 1;
    assert!(!public_key(&seed).is_test_key());
}

#[test]
fn a_message_round_trips_through_the_verifier_unchanged() {
    for approval in every_approval() {
        let role = if approval.needs_trust_root() {
            Role::TrustRoot
        } else {
            Role::OwnerPhone
        };
        let original = Message {
            target: DEVICE,
            signer: role,
            counter: 77,
            expires_at: 12345,
            approval,
        };
        let (bytes, signature) = signed(&original);
        let mut verifier = test_verifier();
        let approved = verifier.verify::<Approval>(&bytes, &signature, 0).unwrap();
        assert_eq!(*approved.approval(), approval);
        assert_eq!(approved.counter(), 77);
        assert_eq!(approved.expires_at(), 12345);
    }
}
