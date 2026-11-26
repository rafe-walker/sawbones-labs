//! The one byte encoding, pinned. If a golden here changes, every signature
//! ever made changes meaning: bump `FORMAT_VERSION` instead.
//!
//! The one exception so far is #203: the lease moved to a new kind (7)
//! carrying the typed action, and the old kind (2) is refused, so no old
//! signature can be read in the new layout. The lease goldens changed with
//! it.

mod common;

use common::every_approval;
use sawbones_approvals::test_keys::{sign, TRUST_ROOT_SEED};
use sawbones_approvals::{
    signed_bytes, ActionError, Approval, BusWriteAction, DecodeError, DeviceId, Digest, GrantLease,
    Message, ModuleId, OperatorAttestation, PluginId, PluginPermissionGrowth, Role, WriteFunction,
    DOMAIN, FORMAT_VERSION, MESSAGE_LEN, SIGNED_LEN,
};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

/// A trust-root lease on port 3 for device 0x01020304, counter
/// 0x1122334455667788, expiring at tick 0x0A0B0C0D0E0F1011: write 0x0003 to
/// register 0x0010 of unit 1. The digest bytes are a pattern, not the
/// action's digest: the verifier carries them, the bus guard checks them.
fn golden_lease() -> Message {
    let mut action = [0; 32];
    for (i, byte) in action.iter_mut().enumerate() {
        *byte = u8::try_from(i).unwrap();
    }
    Message {
        target: DeviceId(0x0102_0304),
        signer: Role::TrustRoot,
        counter: 0x1122_3344_5566_7788,
        expires_at: 0x0A0B_0C0D_0E0F_1011,
        approval: Approval::GrantLease(GrantLease {
            action: BusWriteAction::new(3, 1, WriteFunction::WriteSingleRegister, 0x0010, &[3])
                .unwrap(),
            action_digest: Digest(action),
        }),
    }
}

const GOLDEN_LEASE_HEX: &str = concat!(
    "01",                                                               // version
    "01020304",                                                         // target
    "01",                                                               // role: trust root
    "00000000",                                                         // module id
    "1122334455667788",                                                 // counter
    "0a0b0c0d0e0f1011",                                                 // expiry
    "07",                                                               // kind: lease
    "03",                                                               // action: port
    "01",                                                               // unit
    "06",                                                               // function
    "0010",                                                             // register
    "01",                                                               // value count
    "0003000000000000",                                                 // values (4 slots)
    "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f", // action digest
    "000000000000000000000000000000000000",                             // padding (18)
);

#[test]
fn the_lease_encoding_is_pinned() {
    assert_eq!(MESSAGE_LEN, 91);
    assert_eq!(FORMAT_VERSION, 1);
    assert_eq!(hex(&golden_lease().encode()), GOLDEN_LEASE_HEX);
}

#[test]
fn a_module_plugin_growth_encoding_is_pinned() {
    let message = Message {
        target: DeviceId(9),
        signer: Role::Module(ModuleId(0xDEAD_BEEF)),
        counter: 1,
        expires_at: 2,
        approval: Approval::PluginPermissionGrowth(PluginPermissionGrowth {
            plugin: PluginId([0xAA; 32]),
            manifest: Digest([0xBB; 32]),
        }),
    };
    let expected = format!(
        "01{}03{}{}{}04{}{}",
        "00000009",
        "deadbeef",
        "0000000000000001",
        "0000000000000002",
        "aa".repeat(32),
        "bb".repeat(32),
    );
    assert_eq!(hex(&message.encode()), expected);
}

#[test]
fn the_owner_phone_attestation_encoding_is_pinned() {
    let message = Message {
        target: DeviceId(0),
        signer: Role::OwnerPhone,
        counter: u64::MAX,
        expires_at: 0,
        approval: Approval::OperatorAttestation(OperatorAttestation {
            fact: Digest([0x5C; 32]),
        }),
    };
    let expected = format!(
        "01{}02{}{}{}06{}{}",
        "00000000",
        "00000000",
        "ffffffffffffffff",
        "0000000000000000",
        "5c".repeat(32),
        "00".repeat(32),
    );
    assert_eq!(hex(&message.encode()), expected);
}

#[test]
fn the_kind_tags_are_pinned() {
    let tags: Vec<u8> = every_approval()
        .iter()
        .map(|&approval| common::message(Role::TrustRoot, 1, approval).encode()[26])
        .collect();
    assert_eq!(tags, [1, 7, 3, 4, 5, 6]);
}

#[test]
fn the_domain_and_signed_bytes_are_pinned() {
    assert_eq!(&DOMAIN, b"sawbones-approvals/v1\0");
    let bytes = golden_lease().encode();
    let signed = signed_bytes(&bytes);
    assert_eq!(SIGNED_LEN, DOMAIN.len() + MESSAGE_LEN);
    assert_eq!(&signed[..DOMAIN.len()], &DOMAIN);
    assert_eq!(&signed[DOMAIN.len()..], &bytes);
}

/// Ed25519 is deterministic, so the test trust root's signature over the
/// golden lease is fixed too. This pins the domain prefix in use: a change to
/// what is signed changes this signature.
#[test]
fn the_golden_signature_is_pinned() {
    let signature = sign(&TRUST_ROOT_SEED, &golden_lease().encode());
    assert_eq!(hex(&signature.0), GOLDEN_SIGNATURE_HEX);
}

const GOLDEN_SIGNATURE_HEX: &str = concat!(
    "b661a4800a3b1a54d8f7bd08bae4f08c144fade4cc8897a15d278f4d9e57b14e",
    "76e480102e5034bca1fbca3a2b40878c7fa503d623757aa504583409f9316104",
);

#[test]
fn decode_inverts_encode_for_every_kind_and_role() {
    let roles = [
        Role::TrustRoot,
        Role::OwnerPhone,
        Role::Module(ModuleId(0)),
        Role::Module(ModuleId(u32::MAX)),
    ];
    for approval in every_approval() {
        for signer in roles {
            let message = Message {
                target: DeviceId(0xFFFF_FFFF),
                signer,
                counter: 3,
                expires_at: u64::MAX,
                approval,
            };
            assert_eq!(Message::decode(&message.encode()), Ok(message));
        }
    }
    assert_eq!(
        Message::decode(&unhex(GOLDEN_LEASE_HEX)),
        Ok(golden_lease())
    );
}

#[test]
fn decode_refuses_every_non_canonical_form() {
    let good = golden_lease().encode();
    let with = |at: usize, value: u8| {
        let mut bytes = good;
        bytes[at] = value;
        Message::decode(&bytes)
    };
    assert_eq!(with(0, 0), Err(DecodeError::Version));
    assert_eq!(with(0, 2), Err(DecodeError::Version));
    assert_eq!(with(5, 0), Err(DecodeError::UnknownRole));
    assert_eq!(with(5, 4), Err(DecodeError::UnknownRole));
    assert_eq!(with(9, 1), Err(DecodeError::StrayModuleId));
    assert_eq!(with(26, 0), Err(DecodeError::UnknownKind));
    assert_eq!(with(26, 8), Err(DecodeError::UnknownKind));
    // Kind 2, the frame-digest-only lease from before #203, is retired.
    assert_eq!(with(26, 2), Err(DecodeError::UnknownKind));
    // A lease whose action bytes are not an action.
    assert_eq!(
        with(27 + 1, 0),
        Err(DecodeError::Action(ActionError::BroadcastUnit))
    );
    assert_eq!(
        with(27 + 2, 0x05),
        Err(DecodeError::Action(ActionError::UnknownFunction(0x05)))
    );
    assert_eq!(
        with(27 + 5, 2),
        Err(DecodeError::Action(ActionError::SingleNeedsOneValue))
    );
    assert_eq!(
        with(27 + 9, 1),
        Err(DecodeError::Action(ActionError::NonZeroPadding))
    );
    for at in 27 + 46..MESSAGE_LEN {
        assert_eq!(with(at, 1), Err(DecodeError::NonZeroPadding), "byte {at}");
    }
    assert_eq!(Message::decode(&good[..90]), Err(DecodeError::Length));
    assert_eq!(Message::decode(&[]), Err(DecodeError::Length));
}

#[test]
fn padding_starts_right_after_each_kinds_payload() {
    let used = [32, 46, 32, 64, 32, 32];
    for (approval, used) in every_approval().into_iter().zip(used) {
        let good = common::message(Role::TrustRoot, 1, approval).encode();
        for at in 27 + used..MESSAGE_LEN {
            let mut bytes = good;
            bytes[at] = 0x80;
            assert_eq!(
                Message::decode(&bytes),
                Err(DecodeError::NonZeroPadding),
                "{approval:?} byte {at}"
            );
        }
    }
}
