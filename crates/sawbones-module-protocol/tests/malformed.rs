//! Truncated, oversized, wrong-version, bad-checksum and otherwise malformed
//! frames each get their own typed error.

mod common;

use common::*;
use sawbones_module_protocol::*;

fn health() -> Vec<u8> {
    request_bytes(RequestHeader { session: 5, seq: 9 }, Request::Health)
}

fn config_write() -> Vec<u8> {
    let w = ConfigWrite::new(MODULE, ConfigKey(3), 1, Grant::Lease { port: Port(0) });
    request_bytes(
        RequestHeader { session: 5, seq: 9 },
        Request::ConfigWrite(w),
    )
}

/// Replaces the payload of a valid frame, fixing the length and the check.
fn with_payload(frame: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut out = frame[..HEADER_LEN].to_vec();
    out[4..6].copy_from_slice(&(payload.len() as u16).to_le_bytes());
    out.extend_from_slice(payload);
    out.extend_from_slice(&[0; CHECK_LEN]);
    fix_crc(&mut out);
    out
}

#[test]
fn every_truncation_is_refused_as_truncated() {
    for frame in [health(), config_write()] {
        for len in 0..frame.len() {
            let cut = &frame[..len];
            let err = decode_request(cut).unwrap_err();
            assert!(
                matches!(err, DecodeError::Truncated { actual, .. } if actual == len),
                "{len}: {err:?}"
            );
            assert!(decode_response(cut).is_err());
        }
    }
}

#[test]
fn frame_len_needs_the_prefix_and_reports_the_total() {
    let frame = config_write();
    for len in 0..PREFIX_LEN {
        assert!(matches!(
            frame_len(&frame[..len]),
            Err(DecodeError::Truncated {
                expected: PREFIX_LEN,
                ..
            })
        ));
    }
    assert_eq!(frame_len(&frame[..PREFIX_LEN]), Ok(frame.len()));
}

#[test]
fn an_oversized_declared_length_is_refused_before_anything_is_read() {
    let mut frame = health();
    frame[4..6].copy_from_slice(&((MAX_PAYLOAD_LEN + 1) as u16).to_le_bytes());
    assert_eq!(
        decode_request(&frame),
        Err(DecodeError::Oversized {
            declared: MAX_PAYLOAD_LEN + 1
        })
    );
    frame[4..6].copy_from_slice(&u16::MAX.to_le_bytes());
    assert_eq!(
        frame_len(&frame),
        Err(DecodeError::Oversized {
            declared: usize::from(u16::MAX)
        })
    );
    // A huge input is refused for its length, not read.
    let huge = vec![0u8; 1 << 16];
    assert!(decode_request(&huge).is_err());
}

#[test]
fn trailing_bytes_are_refused() {
    let mut frame = health();
    frame.push(0);
    assert_eq!(
        decode_request(&frame),
        Err(DecodeError::TrailingBytes { extra: 1 })
    );
}

#[test]
fn a_wrong_version_or_magic_is_refused() {
    for version in (0..=255u8).filter(|v| *v != VERSION) {
        let mut frame = health();
        frame[2] = version;
        fix_crc(&mut frame);
        assert_eq!(
            decode_request(&frame),
            Err(DecodeError::UnsupportedVersion(version))
        );
        assert!(decode_response(&frame).is_err());
    }
    let mut frame = health();
    frame[0] = b'X';
    fix_crc(&mut frame);
    assert_eq!(decode_request(&frame), Err(DecodeError::BadMagic));
}

#[test]
fn a_short_frame_of_another_version_says_so() {
    let mut frame = health();
    frame[2] = 2;
    assert_eq!(
        decode_request(&frame[..10]),
        Err(DecodeError::UnsupportedVersion(2))
    );
}

#[test]
fn every_single_bit_flip_fails_the_checksum() {
    let frame = config_write();
    // Bytes 0 to 5 are checked before the check value: a flip there may be
    // refused as a bad magic, version, kind or length first.
    for at in PREFIX_LEN..frame.len() {
        for bit in 0..8 {
            let mut bad = frame.clone();
            bad[at] ^= 1 << bit;
            let err = decode_request(&bad).unwrap_err();
            assert!(
                matches!(err, DecodeError::BadChecksum { .. }),
                "{at}/{bit}: {err:?}"
            );
        }
    }
}

#[test]
fn an_unknown_kind_or_the_wrong_direction_is_refused() {
    let mut frame = health();
    frame[3] = 0x7f;
    fix_crc(&mut frame);
    assert_eq!(decode_request(&frame), Err(DecodeError::UnknownKind(0x7f)));
    assert_eq!(
        decode_response(&frame).unwrap_err(),
        DecodeError::UnknownKind(0x7f)
    );

    // A request reflected back at the host is not a response.
    let frame = health();
    assert_eq!(
        decode_response(&frame).unwrap_err(),
        DecodeError::WrongDirection(RequestKind::Health.to_u8())
    );
    let reply = response_bytes(
        ResponseHeader {
            session: 5,
            seq: 1,
            reply_to: 9,
        },
        Response::SafeStateEntered,
    );
    assert_eq!(
        decode_request(&reply),
        Err(DecodeError::WrongDirection(
            ResponseKind::SafeStateEntered.to_u8()
        ))
    );
}

#[test]
fn a_request_with_a_reply_to_is_refused() {
    let mut frame = health();
    frame[18] = 1;
    fix_crc(&mut frame);
    assert_eq!(decode_request(&frame), Err(DecodeError::ReplyToInRequest));
}

#[test]
fn short_and_long_payloads_are_refused() {
    let frame = config_write();
    let payload = &frame[HEADER_LEN..frame.len() - CHECK_LEN];
    assert_eq!(
        decode_request(&with_payload(&frame, &payload[..payload.len() - 1])),
        Err(DecodeError::PayloadTooShort)
    );
    let mut long = payload.to_vec();
    long.push(0);
    assert_eq!(
        decode_request(&with_payload(&frame, &long)),
        Err(DecodeError::PayloadTrailing)
    );
}

#[test]
fn a_class_4_authority_never_decodes() {
    let frame = config_write();
    let mut payload = frame[HEADER_LEN..frame.len() - CHECK_LEN].to_vec();
    // key (2) and value (8), then the authority's tag.
    payload.truncate(10);
    payload.push(4);
    assert_eq!(
        decode_request(&with_payload(&frame, &payload)),
        Err(DecodeError::ClassNeverPermitted)
    );
    payload[10] = 1;
    assert_eq!(
        decode_request(&with_payload(&frame, &payload)),
        Err(DecodeError::BadTag {
            field: "authority",
            value: 1
        })
    );
}

#[test]
fn a_lease_for_a_port_rail_b_does_not_have_is_refused() {
    let frame = config_write();
    let mut payload = frame[HEADER_LEN..frame.len() - CHECK_LEN].to_vec();
    payload[11] = sawbones_interlock::RAIL_B_PORTS as u8;
    assert_eq!(
        decode_request(&with_payload(&frame, &payload)),
        Err(DecodeError::OutOfBounds {
            field: "lease port"
        })
    );
}

#[test]
fn update_chunks_are_bounded() {
    let header = RequestHeader { session: 1, seq: 1 };
    let big = [0u8; MAX_CHUNK_LEN + 1];
    let cases = [
        (&big[..], 0, 10_000),
        (&big[..0], 0, 10),
        (&big[..10], 5, 14),
        (&big[..10], u32::MAX - 3, u32::MAX),
    ];
    for (bytes, offset, image_len) in cases {
        let chunk = Request::StageUpdate(UpdateChunk {
            image_digest: ImageDigest([0; 32]),
            image_len,
            offset,
            bytes,
        });
        let mut out = [0u8; MAX_FRAME_LEN];
        let err = encode_request(
            &RequestFrame {
                header,
                request: chunk,
            },
            &mut out,
        )
        .unwrap_err();
        assert!(
            matches!(err, EncodeError::Invalid(DecodeError::OutOfBounds { .. })),
            "{err:?}"
        );
    }
    // The decoder refuses the same chunk when its bytes are forged.
    let good = request_bytes(
        header,
        Request::StageUpdate(UpdateChunk {
            image_digest: ImageDigest([0; 32]),
            image_len: 10,
            offset: 0,
            bytes: &[1; 10],
        }),
    );
    let mut payload = good[HEADER_LEN..good.len() - CHECK_LEN].to_vec();
    payload[32..36].copy_from_slice(&9u32.to_le_bytes());
    assert_eq!(
        decode_request(&with_payload(&good, &payload)),
        Err(DecodeError::OutOfBounds {
            field: "chunk range"
        })
    );
}

#[test]
fn manifest_pages_and_log_batches_are_bounded() {
    let header = ResponseHeader {
        session: 1,
        seq: 1,
        reply_to: 1,
    };
    let mut out = [0u8; MAX_FRAME_LEN];
    for (page, page_count) in [(0, 0), (1, 1), (0, MAX_MANIFEST_PAGES + 1)] {
        let r = Response::Description(DescriptionPage {
            page,
            page_count,
            text: ModuleBytes::new(b"x").unwrap(),
        });
        let frame = ResponseFrame {
            header,
            response: r,
        };
        assert!(matches!(
            encode_response(&frame, &mut out),
            Err(EncodeError::Invalid(DecodeError::OutOfBounds { .. }))
        ));
    }
    assert!(ModuleBytes::new(&[0; MAX_PAGE_LEN + 1]).is_none());

    let entry = |index| LogEntry {
        index,
        tick: 0,
        level: LogLevel::Info,
        code: 0,
    };
    let mut buf = [0u8; MAX_LOG_BYTES];
    assert!(LogEntries::write(&[entry(2), entry(2)], &mut buf).is_err());
    let too_many: Vec<_> = (0..=u32::from(MAX_LOG_ENTRIES)).map(entry).collect();
    assert!(LogEntries::write(&too_many, &mut buf).is_err());

    // Forged on the wire: too many entries, a bad level, falling indexes.
    let mut good_buf = [0u8; MAX_LOG_BYTES];
    let good = response_bytes(
        header,
        Response::Log(LogEntries::write(&[entry(1), entry(2)], &mut good_buf).unwrap()),
    );
    let payload = good[HEADER_LEN..good.len() - CHECK_LEN].to_vec();
    let mut p = payload.clone();
    p[0] = MAX_LOG_ENTRIES + 1;
    assert_eq!(
        decode_response(&with_payload(&good, &p)).unwrap_err(),
        DecodeError::OutOfBounds {
            field: "log entries"
        }
    );
    let mut p = payload.clone();
    p[1 + 12] = 4;
    assert_eq!(
        decode_response(&with_payload(&good, &p)).unwrap_err(),
        DecodeError::BadTag {
            field: "log level",
            value: 4
        }
    );
    let mut p = payload;
    p[1 + LOG_ENTRY_LEN..1 + LOG_ENTRY_LEN + 4].copy_from_slice(&1u32.to_le_bytes());
    assert_eq!(
        decode_response(&with_payload(&good, &p)).unwrap_err(),
        DecodeError::OutOfBounds { field: "log index" }
    );
}

#[test]
fn bad_enumerations_and_booleans_are_refused() {
    let header = ResponseHeader {
        session: 1,
        seq: 1,
        reply_to: 1,
    };
    let staged = response_bytes(
        header,
        Response::UpdateStaged {
            received: 1,
            complete: true,
        },
    );
    let mut p = staged[HEADER_LEN..staged.len() - CHECK_LEN].to_vec();
    p[4] = 2;
    assert_eq!(
        decode_response(&with_payload(&staged, &p)).unwrap_err(),
        DecodeError::BadBool
    );

    let refused = response_bytes(header, Response::Refused(RefusalCode::Busy));
    assert_eq!(
        decode_response(&with_payload(&refused, &[0])).unwrap_err(),
        DecodeError::BadTag {
            field: "refusal code",
            value: 0
        }
    );

    let time = response_bytes(
        header,
        Response::Time(TimeReport {
            host_time_us: 1,
            module_ticks: 1,
            tick_hz: 1,
        }),
    );
    let mut p = time[HEADER_LEN..time.len() - CHECK_LEN].to_vec();
    p[16..20].copy_from_slice(&0u32.to_le_bytes());
    assert_eq!(
        decode_response(&with_payload(&time, &p)).unwrap_err(),
        DecodeError::OutOfBounds { field: "tick hz" }
    );
}

#[test]
fn a_capability_claim_of_class_4_never_decodes() {
    let header = ResponseHeader {
        session: 1,
        seq: 1,
        reply_to: 1,
    };
    let mut claim = signed_identity(
        &MODULE_SEED,
        MODULE,
        CAPS,
        &Challenge([0; 16]),
        core::num::NonZeroU64::new(1).unwrap(),
    );
    let good = response_bytes(header, Response::Identity(claim));
    let mut p = good[HEADER_LEN..good.len() - CHECK_LEN].to_vec();
    p[5] = 4;
    assert_eq!(
        decode_response(&with_payload(&good, &p)).unwrap_err(),
        DecodeError::ClassNeverPermitted
    );
    claim.capabilities.max_class = HazardClass::Class4;
    let mut out = [0u8; MAX_FRAME_LEN];
    assert_eq!(
        encode_response(
            &ResponseFrame {
                header,
                response: Response::Identity(claim)
            },
            &mut out
        ),
        Err(EncodeError::Invalid(DecodeError::ClassNeverPermitted))
    );
}

#[test]
fn a_small_buffer_is_refused_not_overrun() {
    let frame = RequestFrame {
        header: RequestHeader { session: 1, seq: 1 },
        request: Request::Time { host_time_us: 3 },
    };
    let needed = request_bytes(frame.header, frame.request).len();
    for size in 0..needed {
        let mut out = vec![0u8; size];
        assert!(matches!(
            encode_request(&frame, &mut out),
            Err(EncodeError::BufferTooSmall { .. })
        ));
    }
}
