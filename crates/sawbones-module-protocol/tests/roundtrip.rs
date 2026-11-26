//! Round-trip property tests: every valid message encodes, decodes to
//! itself, and re-encodes to the same bytes.

mod common;

use common::*;
use sawbones_module_protocol::*;

const CASES: u64 = 5_000;

#[test]
fn every_request_round_trips() {
    let mut rng = Rng::new(0x0098_0001);
    let mut pool = [0u8; MAX_CHUNK_LEN];
    for _ in 0..CASES {
        rng.fill(&mut pool);
        let request = random_request(&mut rng, &pool);
        let header = RequestHeader {
            session: rng.next(),
            seq: rng.next() as u32,
        };
        let bytes = request_bytes(header, request);
        assert!(bytes.len() <= MAX_FRAME_LEN);
        assert_eq!(frame_len(&bytes), Ok(bytes.len()));
        let decoded = decode_request(&bytes).unwrap();
        assert_eq!(decoded, RequestFrame { header, request });
        assert_eq!(request_bytes(decoded.header, decoded.request), bytes);
    }
}

#[test]
fn every_response_round_trips() {
    let mut rng = Rng::new(0x0098_0002);
    let mut pool = [0u8; MAX_PAGE_LEN];
    for _ in 0..CASES {
        rng.fill(&mut pool);
        let mut log_buf = [0u8; MAX_LOG_BYTES];
        let response = random_response(&mut rng, &pool, &mut log_buf);
        let header = ResponseHeader {
            session: rng.next(),
            seq: rng.next() as u32,
            reply_to: rng.next() as u32,
        };
        let bytes = response_bytes(header, response);
        assert!(bytes.len() <= MAX_FRAME_LEN);
        assert_eq!(frame_len(&bytes), Ok(bytes.len()));
        let decoded = decode_response(&bytes).unwrap().into_claim();
        assert_eq!(decoded, ResponseFrame { header, response });
        assert_eq!(response_bytes(decoded.header, decoded.response), bytes);
    }
}

#[test]
fn every_kind_has_one_expected_reply_and_the_kinds_are_disjoint() {
    for kind in RequestKind::ALL {
        assert!(kind.to_u8() < 0x80);
        assert_eq!(RequestKind::from_u8(kind.to_u8()), Some(*kind));
        assert!(ResponseKind::from_u8(kind.to_u8()).is_none());
        assert_ne!(kind.expected_reply(), ResponseKind::Refused);
    }
    for kind in ResponseKind::ALL {
        assert!(kind.to_u8() >= 0x80);
        assert_eq!(ResponseKind::from_u8(kind.to_u8()), Some(*kind));
        assert!(RequestKind::from_u8(kind.to_u8()).is_none());
    }
}

#[test]
fn the_largest_messages_fit_in_one_frame() {
    let chunk = [0xAB; MAX_CHUNK_LEN];
    let request = Request::StageUpdate(UpdateChunk {
        image_digest: ImageDigest([1; 32]),
        image_len: u32::MAX,
        offset: 0,
        bytes: &chunk,
    });
    let bytes = request_bytes(RequestHeader { session: 1, seq: 1 }, request);
    assert!(bytes.len() <= MAX_FRAME_LEN);

    let page = [0xCD; MAX_PAGE_LEN];
    let response = Response::Description(DescriptionPage {
        page: MAX_MANIFEST_PAGES - 1,
        page_count: MAX_MANIFEST_PAGES,
        text: ModuleBytes::new(&page).unwrap(),
    });
    let header = ResponseHeader {
        session: 1,
        seq: 1,
        reply_to: 1,
    };
    assert!(response_bytes(header, response).len() <= MAX_FRAME_LEN);

    let entries: Vec<LogEntry> = (0..u32::from(MAX_LOG_ENTRIES))
        .map(|index| LogEntry {
            index,
            tick: u64::MAX,
            level: LogLevel::Error,
            code: u16::MAX,
        })
        .collect();
    let mut log_buf = [0u8; MAX_LOG_BYTES];
    let response = Response::Log(LogEntries::write(&entries, &mut log_buf).unwrap());
    assert!(response_bytes(header, response).len() <= MAX_FRAME_LEN);
}

#[test]
fn log_entries_iterate_in_order() {
    let entries = [
        LogEntry {
            index: 3,
            tick: 30,
            level: LogLevel::Warn,
            code: 7,
        },
        LogEntry {
            index: 9,
            tick: 90,
            level: LogLevel::Error,
            code: 8,
        },
    ];
    let mut buf = [0u8; MAX_LOG_BYTES];
    let log = LogEntries::write(&entries, &mut buf).unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log.iter().collect::<Vec<_>>(), entries);
    assert!(LogEntries::EMPTY.is_empty());
}

#[test]
fn an_authority_covers_its_class_and_below_and_never_class_4() {
    use HazardClass::*;
    let lease = Authority::Lease(LeaseTarget {
        port: Port(0),
        action_digest: ActionDigest([0; 32]),
    });
    let table = [
        (Authority::None, [true, true, false, false, false]),
        (Authority::RailA, [true, true, true, false, false]),
        (lease, [true, true, true, true, false]),
    ];
    for (authority, expected) in table {
        for (class, want) in [Class0, Class1, Class2, Class3, Class4]
            .into_iter()
            .zip(expected)
        {
            assert_eq!(authority.permits(class), want, "{authority:?} {class:?}");
        }
    }
}

#[test]
fn a_built_lease_is_bound_to_its_action_and_module() {
    let w = ConfigWrite::new(MODULE, ConfigKey(3), 42, Grant::Lease { port: Port(1) });
    let target = Request::ConfigWrite(w).lease_target().unwrap();
    assert_eq!(target.port, Port(1));
    assert_eq!(target.action_digest, action_digest(MODULE, w.body()));
    // Another value, key, module or kind of action gives another digest.
    let others = [
        action_digest(
            MODULE,
            ActionBody::ConfigWrite {
                key: ConfigKey(3),
                value: 43,
            },
        ),
        action_digest(
            MODULE,
            ActionBody::ConfigWrite {
                key: ConfigKey(4),
                value: 42,
            },
        ),
        action_digest(OTHER_MODULE, w.body()),
        action_digest(
            MODULE,
            ActionBody::Actuate {
                channel: Channel(3),
                setting: 42,
            },
        ),
    ];
    for other in others {
        assert_ne!(other, target.action_digest);
    }
    let a = Actuate::new(MODULE, Channel(1), -5, Grant::RailA);
    assert_eq!(a.authority, Authority::RailA);
    assert_eq!(Request::Actuate(a).lease_target(), None);
}
