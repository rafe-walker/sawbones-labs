//! Module link attacks (lane `interlock-hardware`, card hw-023), against the
//! real module protocol: frames on the wire, decoded by the host, identity
//! checked against the enrolled key.

use core::num::NonZeroU64;

use ed25519_dalek::{Signer, SigningKey};
use sawbones_module_protocol::{
    crc32c, decode_response, encode_response, Capabilities, Challenge, HazardClass, HostLink,
    IdentityClaim, IdentityError, IdentitySignature, ImageDigest, ManifestHasher, ModuleId,
    ModuleKey, Response, ResponseFrame, ResponseHeader, CHECK_LEN, HEADER_LEN, MAX_FRAME_LEN,
};

/// TEST ONLY. The meter's seed. Public, in tests only.
const METER_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY redteam-meter ";
/// TEST ONLY. A second module's seed.
const POD_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY redteam-pod   ";
const METER: ModuleId = ModuleId(1);
const POD: ModuleId = ModuleId(2);

const MANIFEST: [&[u8]; 2] = [b"name: dc meter\n", b"channels: none\n"];

fn key(seed: &[u8; 32]) -> ModuleKey {
    ModuleKey(SigningKey::from_bytes(seed).verifying_key().to_bytes())
}

fn manifest_digest(pages: &[&[u8]]) -> sawbones_module_protocol::ManifestDigest {
    let mut h = ManifestHasher::new();
    for page in pages {
        h.bytes(page);
    }
    h.finish()
}

fn sign(
    seed: &[u8; 32],
    module: ModuleId,
    can_emit: bool,
    challenge: &Challenge,
    session: u64,
) -> IdentityClaim {
    let mut claim = IdentityClaim {
        module,
        capabilities: Capabilities {
            can_emit,
            max_class: HazardClass::Class0,
        },
        manifest_digest: manifest_digest(&MANIFEST),
        firmware_digest: ImageDigest([7; 32]),
        signature: IdentitySignature([0; 64]),
    };
    let session = NonZeroU64::new(session).unwrap();
    claim.signature = IdentitySignature(
        SigningKey::from_bytes(seed)
            .sign(&claim.signed_bytes(challenge, session))
            .to_bytes(),
    );
    claim
}

/// The host opens a session with `challenge` and receives `claim` from
/// session `session` on the wire. Returns what the host decoded.
fn host_receives(
    challenge: Challenge,
    session: u64,
    claim: IdentityClaim,
) -> (HostLink, IdentityClaim) {
    let mut host = HostLink::new();
    let mut out = [0u8; MAX_FRAME_LEN];
    host.open(challenge, &mut out).unwrap();
    let mut reply = [0u8; MAX_FRAME_LEN];
    let n = encode_response(
        &ResponseFrame {
            header: ResponseHeader {
                session,
                seq: 1,
                reply_to: 1,
            },
            response: Response::Identity(claim),
        },
        &mut reply,
    )
    .unwrap();
    match host.receive(&reply[..n]).unwrap().into_claim() {
        Response::Identity(claim) => (host, claim),
        other => panic!("{other:?}"),
    }
}

#[test]
fn redteam_hw_023_a_module_cannot_claim_another_modules_id() {
    let challenge = Challenge([0xA1; 16]);
    // The pod says it is the meter, signing with its own key.
    let forged = sign(&POD_SEED, METER, false, &challenge, 40);
    let (host, claim) = host_receives(challenge, 40, forged);
    let session = host.session().unwrap();
    assert_eq!(
        claim.verify(METER, &key(&METER_SEED), &challenge, session),
        Err(IdentityError::BadSignature)
    );
    // Checked against the pod's own enrolment, the id is wrong.
    assert_eq!(
        claim.verify(POD, &key(&POD_SEED), &challenge, session),
        Err(IdentityError::WrongModule {
            expected: POD,
            claimed: METER
        })
    );
}

#[test]
fn redteam_hw_023_a_captured_identity_does_not_answer_a_new_challenge() {
    let old_challenge = Challenge([0x01; 16]);
    let captured = sign(&METER_SEED, METER, false, &old_challenge, 40);
    // Replayed into a new session, against a new challenge, from the same or
    // another session id.
    let new_challenge = Challenge([0x02; 16]);
    for session in [40, 41] {
        let (host, claim) = host_receives(new_challenge, session, captured);
        assert_eq!(
            claim.verify(
                METER,
                &key(&METER_SEED),
                &new_challenge,
                host.session().unwrap()
            ),
            Err(IdentityError::BadSignature)
        );
    }
    // The genuine answer verifies.
    let fresh = sign(&METER_SEED, METER, false, &new_challenge, 41);
    let (host, claim) = host_receives(new_challenge, 41, fresh);
    assert!(claim
        .verify(
            METER,
            &key(&METER_SEED),
            &new_challenge,
            host.session().unwrap()
        )
        .is_ok());
}

#[test]
fn redteam_hw_023_an_unsigned_capability_change_does_not_verify() {
    let challenge = Challenge([0x33; 16]);
    let genuine = sign(&METER_SEED, METER, false, &challenge, 9);
    let mut frame = [0u8; MAX_FRAME_LEN];
    let n = encode_response(
        &ResponseFrame {
            header: ResponseHeader {
                session: 9,
                seq: 1,
                reply_to: 1,
            },
            response: Response::Identity(genuine),
        },
        &mut frame,
    )
    .unwrap();
    // On the wire: set can_emit (payload byte 4) and the highest class
    // (byte 5), and fix the check value, as anyone on the link can.
    let mut tampered = frame[..n].to_vec();
    tampered[HEADER_LEN + 4] = 1;
    tampered[HEADER_LEN + 5] = 3;
    let body = tampered.len() - CHECK_LEN;
    let crc = crc32c(&tampered[..body]);
    tampered[body..].copy_from_slice(&crc.to_le_bytes());
    let Response::Identity(claim) = decode_response(&tampered).unwrap().into_claim().response
    else {
        panic!("not an identity");
    };
    assert!(claim.capabilities.can_emit);
    assert_eq!(
        claim.verify(
            METER,
            &key(&METER_SEED),
            &challenge,
            NonZeroU64::new(9).unwrap()
        ),
        Err(IdentityError::BadSignature)
    );
}

#[test]
fn redteam_hw_023_a_swapped_manifest_does_not_match_the_signed_digest() {
    let challenge = Challenge([0x44; 16]);
    let claim = sign(&METER_SEED, METER, false, &challenge, 9);
    let verified = claim
        .verify(
            METER,
            &key(&METER_SEED),
            &challenge,
            NonZeroU64::new(9).unwrap(),
        )
        .unwrap();
    assert_eq!(verified.manifest_digest(), manifest_digest(&MANIFEST));
    // A module that serves other pages under the same signed identity.
    let swapped: [&[u8]; 2] = [b"name: dc meter\n", b"channels: 1 relay class 1\n"];
    assert_ne!(verified.manifest_digest(), manifest_digest(&swapped));
}
