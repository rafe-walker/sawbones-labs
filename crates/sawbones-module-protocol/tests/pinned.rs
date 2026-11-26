//! Golden encodings. These pin the wire format: a change to any byte of a
//! frame, a digest or the signed identity layout fails here, so the format
//! cannot drift silently. A deliberate change bumps [`VERSION`] and updates
//! these vectors in the same pull request.

mod common;

use core::num::NonZeroU64;

use common::*;
use sawbones_module_protocol::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

const SESSION: u64 = 0x0102_0304_0506_0708;

#[test]
fn the_constants_are_pinned() {
    assert_eq!(MAGIC, *b"SB");
    assert_eq!(VERSION, 1);
    assert_eq!(HEADER_LEN, 22);
    assert_eq!(CHECK_LEN, 4);
    assert_eq!(MAX_PAYLOAD_LEN, 320);
    assert_eq!(MAX_FRAME_LEN, 346);
    assert_eq!(MAX_CHUNK_LEN, 256);
    assert_eq!(MAX_PAGE_LEN, 256);
    assert_eq!(MAX_MANIFEST_PAGES, 16);
    assert_eq!(MAX_LOG_ENTRIES, 16);
    assert_eq!(LOG_ENTRY_LEN, 15);
    assert_eq!(IDENTITY_SIGNED_LEN, 122);
    assert_eq!(&ACTION_DOMAIN, b"sawbones-module-protocol/action/v1\0");
    assert_eq!(&IDENTITY_DOMAIN, b"sawbones-module-identity/v1\0");
    assert_eq!(&MANIFEST_DOMAIN, b"sawbones-module-manifest/v1\0");
    let kinds: Vec<u8> = RequestKind::ALL.iter().map(|k| k.to_u8()).collect();
    assert_eq!(kinds, (0x01..=0x0c).collect::<Vec<u8>>());
    let kinds: Vec<u8> = ResponseKind::ALL.iter().map(|k| k.to_u8()).collect();
    assert_eq!(kinds, (0x81..=0x8c).collect::<Vec<u8>>());
    let codes: Vec<u8> = RefusalCode::ALL.iter().map(|k| k.to_u8()).collect();
    assert_eq!(codes, (1..=11).collect::<Vec<u8>>());
}

#[test]
fn the_check_is_crc32c() {
    assert_eq!(crc32c(b"123456789"), 0xE306_9283);
    assert_eq!(crc32c(b""), 0);
}

#[test]
fn an_identify_frame_is_pinned() {
    let golden = "5342\
                  01\
                  01\
                  1000\
                  0000000000000000\
                  01000000\
                  00000000\
                  30313233343536373839616263646566\
                  1c652314";
    let frame = request_bytes(
        RequestHeader { session: 0, seq: 1 },
        Request::Identify {
            challenge: Challenge(*b"0123456789abcdef"),
        },
    );
    assert_eq!(hex(&frame), golden.replace(' ', ""));
    assert_eq!(
        decode_request(&unhex(&golden.replace(' ', "")))
            .unwrap()
            .header
            .seq,
        1
    );
}

#[test]
fn a_class_3_config_write_and_its_lease_digest_are_pinned() {
    let w = ConfigWrite::new(MODULE, ConfigKey(3), -2, Grant::Lease { port: Port(1) });
    let digest = "4225dab2337d0b099e2c296337c36df7d6b76fd3ce9cc7c3ae327dd75397a494";
    assert_eq!(hex(&action_digest(MODULE, w.body()).0), digest);
    let golden = format!(
        "5342 01 07 2c00 0807060504030201 02000000 00000000 \
         0300 feffffffffffffff 03 01 {digest} 7cdf4c21"
    )
    .replace(' ', "");
    let frame = request_bytes(
        RequestHeader {
            session: SESSION,
            seq: 2,
        },
        Request::ConfigWrite(w),
    );
    assert_eq!(hex(&frame), golden);
    let bytes = unhex(&golden);
    let decoded = decode_request(&bytes).unwrap();
    assert_eq!(decoded.request, Request::ConfigWrite(w));
}

#[test]
fn a_health_report_is_pinned() {
    let golden = "5342 01 83 1300 0807060504030201 01000000 02000000 \
                  01 e803000000000000 ad0b 03000000 04000000 b97f40e9"
        .replace(' ', "");
    let response = Response::Health(HealthReport {
        status: HealthStatus::Degraded,
        uptime_ticks: 1000,
        fault_code: 0x0bad,
        bad_frames: 3,
        refused_frames: 4,
    });
    let header = ResponseHeader {
        session: SESSION,
        seq: 1,
        reply_to: 2,
    };
    assert_eq!(hex(&response_bytes(header, response)), golden);
    assert_eq!(
        decode_response(&unhex(&golden))
            .unwrap()
            .into_claim()
            .response,
        response
    );
}

#[test]
fn an_identity_frame_and_its_signed_bytes_are_pinned() {
    let claim = IdentityClaim {
        module: MODULE,
        capabilities: Capabilities {
            can_emit: true,
            max_class: HazardClass::Class2,
        },
        manifest_digest: ManifestDigest([0x11; 32]),
        firmware_digest: ImageDigest([0x22; 32]),
        signature: IdentitySignature([0x33; 64]),
    };
    let golden = format!(
        "5342 01 81 8600 0900000000000000 01000000 01000000 07000000 01 02 {} {} {} 85abe621",
        "11".repeat(32),
        "22".repeat(32),
        "33".repeat(64)
    )
    .replace(' ', "");
    let header = ResponseHeader {
        session: 9,
        seq: 1,
        reply_to: 1,
    };
    assert_eq!(
        hex(&response_bytes(header, Response::Identity(claim))),
        golden
    );

    let signed = format!(
        "{} 30313233343536373839616263646566 0900000000000000 07000000 01 02 {} {}",
        hex(b"sawbones-module-identity/v1\0"),
        "11".repeat(32),
        "22".repeat(32)
    )
    .replace(' ', "");
    let bytes = claim.signed_bytes(
        &Challenge(*b"0123456789abcdef"),
        NonZeroU64::new(9).unwrap(),
    );
    assert_eq!(hex(&bytes), signed);
}

#[test]
fn a_refusal_and_the_manifest_digest_are_pinned() {
    let header = ResponseHeader {
        session: 9,
        seq: 2,
        reply_to: 3,
    };
    assert_eq!(
        hex(&response_bytes(
            header,
            Response::Refused(RefusalCode::AuthorityTooLow)
        )),
        "5342018c0100090000000000000002000000030000000330 3d9465".replace(' ', "")
    );
    let mut h = ManifestHasher::new();
    h.bytes(b"name: example\n");
    assert_eq!(
        hex(&h.finish().0),
        "c8ae460eb24efb579ad13e5445433a288ae75343fa75831cf5ee33c67af6244c"
    );
}
