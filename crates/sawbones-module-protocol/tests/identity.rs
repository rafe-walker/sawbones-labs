//! Module identity by a signed challenge, and the release approval the
//! protocol carries.

mod common;

use core::num::NonZeroU64;

use common::*;
use sawbones_module_protocol::*;

const CHALLENGE: Challenge = Challenge(*b"0123456789abcdef");

fn session(n: u64) -> NonZeroU64 {
    NonZeroU64::new(n).unwrap()
}

#[test]
fn a_signed_identity_verifies_for_its_module_challenge_and_session() {
    let claim = signed_identity(&MODULE_SEED, MODULE, CAPS, &CHALLENGE, session(5));
    let verified = claim
        .verify(MODULE, &key(&MODULE_SEED), &CHALLENGE, session(5))
        .unwrap();
    assert_eq!(verified.module(), MODULE);
    assert_eq!(verified.capabilities(), CAPS);
    assert_eq!(verified.firmware_digest(), FIRMWARE);
    assert_eq!(verified.session(), session(5));
}

#[test]
fn an_identity_for_another_challenge_or_session_does_not_verify() {
    let claim = signed_identity(&MODULE_SEED, MODULE, CAPS, &CHALLENGE, session(5));
    let k = key(&MODULE_SEED);
    assert_eq!(
        claim.verify(MODULE, &k, &Challenge([0; 16]), session(5)),
        Err(IdentityError::BadSignature)
    );
    assert_eq!(
        claim.verify(MODULE, &k, &CHALLENGE, session(6)),
        Err(IdentityError::BadSignature)
    );
}

#[test]
fn a_changed_claim_does_not_verify() {
    let claim = signed_identity(&MODULE_SEED, MODULE, CAPS, &CHALLENGE, session(5));
    let k = key(&MODULE_SEED);
    let mut c = claim;
    c.capabilities.can_emit = true;
    assert_eq!(
        c.verify(MODULE, &k, &CHALLENGE, session(5)),
        Err(IdentityError::BadSignature)
    );
    let mut c = claim;
    c.capabilities.max_class = HazardClass::Class2;
    assert_eq!(
        c.verify(MODULE, &k, &CHALLENGE, session(5)),
        Err(IdentityError::BadSignature)
    );
    let mut c = claim;
    c.manifest_digest.0[0] ^= 1;
    assert_eq!(
        c.verify(MODULE, &k, &CHALLENGE, session(5)),
        Err(IdentityError::BadSignature)
    );
    let mut c = claim;
    c.firmware_digest.0[31] ^= 1;
    assert_eq!(
        c.verify(MODULE, &k, &CHALLENGE, session(5)),
        Err(IdentityError::BadSignature)
    );
    let mut c = claim;
    c.signature.0[0] ^= 1;
    assert_eq!(
        c.verify(MODULE, &k, &CHALLENGE, session(5)),
        Err(IdentityError::BadSignature)
    );
}

#[test]
fn a_module_cannot_claim_another_modules_identity() {
    // Signed with its own key, claiming the other id: the other id's key
    // does not verify it, and its own enrolment names another id.
    let claim = signed_identity(&MODULE_SEED, OTHER_MODULE, CAPS, &CHALLENGE, session(5));
    assert_eq!(
        claim.verify(OTHER_MODULE, &key(&OTHER_SEED), &CHALLENGE, session(5)),
        Err(IdentityError::BadSignature)
    );
    assert_eq!(
        claim.verify(MODULE, &key(&MODULE_SEED), &CHALLENGE, session(5)),
        Err(IdentityError::WrongModule {
            expected: MODULE,
            claimed: OTHER_MODULE
        })
    );
    // Bytes that are not a point on the curve are not a key.
    let not_a_point = (0u8..=255)
        .map(|b| {
            let mut k = [0u8; 32];
            k[0] = b;
            k[31] = 0x7f;
            k
        })
        .find(|k| ed25519_dalek::VerifyingKey::from_bytes(k).is_err())
        .expect("some y is not on the curve");
    assert_eq!(
        claim.verify(
            OTHER_MODULE,
            &ModuleKey(not_a_point),
            &CHALLENGE,
            session(5)
        ),
        Err(IdentityError::BadKey)
    );
}

#[test]
fn the_manifest_digest_covers_every_page_in_order() {
    let mut h = ManifestHasher::new();
    for page in MANIFEST.iter().rev() {
        h.bytes(page);
    }
    assert_ne!(h.finish(), manifest_digest());
    let mut h = ManifestHasher::default();
    h.bytes(MANIFEST[0]);
    assert_ne!(h.finish(), manifest_digest());
}

#[test]
fn a_release_approval_is_a_sawbones_approvals_message_and_survives_the_trip() {
    use sawbones_approvals::test_keys::{self, TRUST_ROOT_SEED};
    use sawbones_approvals::{
        Approval, DeviceId, Digest, InstallCoreRelease, Message, Role, MESSAGE_LEN,
    };

    assert_eq!(APPROVAL_MESSAGE_LEN, MESSAGE_LEN);
    let release = Digest([0x42; 32]);
    let message = Message {
        target: DeviceId(7),
        signer: Role::TrustRoot,
        counter: 1,
        expires_at: 1_000,
        approval: Approval::InstallCoreRelease(InstallCoreRelease { release }),
    }
    .encode();
    let signature = test_keys::sign(&TRUST_ROOT_SEED, &message);
    let request = Request::Reset(ResetMode::ActivateStaged {
        image_digest: ImageDigest(release.0),
        release: ReleaseApproval {
            message,
            signature: signature.0,
        },
    });
    let bytes = request_bytes(RequestHeader { session: 1, seq: 2 }, request);
    let Request::Reset(ResetMode::ActivateStaged {
        image_digest,
        release: carried,
    }) = decode_request(&bytes).unwrap().request
    else {
        panic!("not an activation");
    };
    let mut verifier = test_keys::verifier(DeviceId(7));
    let approved = verifier
        .verify::<InstallCoreRelease>(
            &carried.message,
            &sawbones_approvals::Signature(carried.signature),
            0,
        )
        .unwrap();
    assert_eq!(approved.approval().release.0, image_digest.0);
}
