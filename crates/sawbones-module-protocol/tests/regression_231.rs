//! #231: tests for mutants the weekly mutation run found missed. Each test
//! fails with the named mutant applied.

mod common;

use common::*;
use sawbones_module_protocol::*;

const HDR: RequestHeader = RequestHeader { session: 5, seq: 9 };

fn encode(request: Request<'_>, out: &mut [u8]) -> Result<usize, EncodeError> {
    encode_request(
        &RequestFrame {
            header: HDR,
            request,
        },
        out,
    )
}

fn bad_port() -> Port {
    Port(u8::MAX)
}

/// Mutant: `>` with `>=` in `frame_len`. A declared payload of exactly
/// `MAX_PAYLOAD_LEN` is allowed; one more is not.
#[test]
fn regression_231_module_protocol_max_payload_is_allowed() {
    let mut prefix = request_bytes(HDR, Request::Health);
    prefix[4..6].copy_from_slice(&(MAX_PAYLOAD_LEN as u16).to_le_bytes());
    assert_eq!(frame_len(&prefix[..PREFIX_LEN]), Ok(MAX_FRAME_LEN));
    prefix[4..6].copy_from_slice(&(MAX_PAYLOAD_LEN as u16 + 1).to_le_bytes());
    assert_eq!(
        frame_len(&prefix[..PREFIX_LEN]),
        Err(DecodeError::Oversized {
            declared: MAX_PAYLOAD_LEN + 1
        })
    );
}

/// Mutant: `-` with `/` in `split`. The count of trailing bytes is exact.
#[test]
fn regression_231_module_protocol_trailing_bytes_are_counted() {
    let frame = request_bytes(HDR, Request::Health);
    for extra in [1usize, 2, 3, 7, 40] {
        let mut longer = frame.clone();
        longer.extend(core::iter::repeat_n(0xAA, extra));
        assert_eq!(
            decode_request(&longer).unwrap_err(),
            DecodeError::TrailingBytes { extra },
            "{extra} extra"
        );
    }
}

/// Mutants: `+` with `-` or `*` in `encode_frame`'s buffer-too-small `needed`,
/// and `<` with `==` or `<=` in its length check. A buffer one byte short of
/// the frame reports exactly the frame length, a buffer of exactly the frame
/// length works, and a short buffer never reports a smaller need than it has.
#[test]
fn regression_231_module_protocol_encode_reports_what_it_needs() {
    let request = Request::LogRead { from: 3, max: 4 };
    let mut big = [0u8; MAX_FRAME_LEN];
    let total = encode(request, &mut big).unwrap();
    assert_eq!(total, HEADER_LEN + 5 + CHECK_LEN);

    // Exactly enough.
    let mut exact = vec![0u8; total];
    assert_eq!(encode(request, &mut exact), Ok(total));
    assert_eq!(exact, &big[..total]);

    // Short by any amount: the error says the frame needs `total` once the
    // payload fits, and never less than the buffer plus one.
    for len in 0..total {
        let mut short = vec![0u8; len];
        match encode(request, &mut short) {
            Err(EncodeError::BufferTooSmall { needed }) => {
                assert!(needed > len, "len {len}: needed {needed}");
                assert!(needed <= total, "len {len}: needed {needed}");
                if len >= total - CHECK_LEN {
                    assert_eq!(needed, total, "len {len}");
                }
                // Growing to what it asked for gets closer, and finishes.
                let mut grown = len;
                for _ in 0..total {
                    let mut buf = vec![0u8; grown];
                    match encode(request, &mut buf) {
                        Ok(n) => {
                            assert_eq!(n, total);
                            break;
                        }
                        Err(EncodeError::BufferTooSmall { needed }) => {
                            assert!(needed > grown);
                            grown = needed;
                        }
                        Err(other) => panic!("{other:?}"),
                    }
                }
            }
            other => panic!("len {len}: {other:?}"),
        }
    }
}

/// Mutant: delete the `Request::Actuate` arm of `action_body`.
#[test]
fn regression_231_module_protocol_action_body_for_both_commands() {
    let write = ConfigWrite::new(MODULE, ConfigKey(3), 1, Grant::RailA);
    let actuate = Actuate::new(MODULE, Channel(2), -5, Grant::RailA);
    assert_eq!(
        Request::ConfigWrite(write).action_body(),
        Some(write.body())
    );
    assert_eq!(
        Request::Actuate(actuate).action_body(),
        Some(actuate.body())
    );
    assert_eq!(Request::Health.action_body(), None);
}

/// Mutants: delete the lease-port arm of `validate`, and make its guard false.
/// A command whose lease names a port that does not exist is refused, by
/// encode and by decode, for a config write and for an actuation.
#[test]
fn regression_231_module_protocol_lease_port_must_exist() {
    let lease = Grant::Lease { port: bad_port() };
    let refused = EncodeError::Invalid(DecodeError::OutOfBounds {
        field: "lease port",
    });
    let write = Request::ConfigWrite(ConfigWrite::new(MODULE, ConfigKey(3), 1, lease));
    let actuate = Request::Actuate(Actuate::new(MODULE, Channel(1), 9, lease));
    let mut out = [0u8; MAX_FRAME_LEN];
    assert_eq!(encode(write, &mut out), Err(refused));
    assert_eq!(encode(actuate, &mut out), Err(refused));

    // A real port encodes, and the same bytes with the port byte changed are
    // refused on decode.
    let good = Grant::Lease { port: Port(0) };
    for request in [
        Request::ConfigWrite(ConfigWrite::new(MODULE, ConfigKey(3), 1, good)),
        Request::Actuate(Actuate::new(MODULE, Channel(1), 9, good)),
    ] {
        let mut bytes = request_bytes(HDR, request);
        assert!(decode_request(&bytes).is_ok());
        // The lease target is the tail of the payload: tag, port, digest.
        let port_at = bytes.len() - CHECK_LEN - 32 - 1;
        assert_eq!(bytes[port_at], 0);
        bytes[port_at] = u8::MAX;
        fix_crc(&mut bytes);
        assert_eq!(
            decode_request(&bytes).unwrap_err(),
            DecodeError::OutOfBounds {
                field: "lease port"
            }
        );
    }
}

/// Mutant: guard of the `Describe` arm of `validate` replaced with false. The
/// last page is `MAX_MANIFEST_PAGES - 1`.
#[test]
fn regression_231_module_protocol_describe_page_is_bounded() {
    let mut out = [0u8; MAX_FRAME_LEN];
    assert!(encode(
        Request::Describe {
            page: MAX_MANIFEST_PAGES - 1
        },
        &mut out
    )
    .is_ok());
    for page in [MAX_MANIFEST_PAGES, MAX_MANIFEST_PAGES + 1, u8::MAX] {
        assert_eq!(
            encode(Request::Describe { page }, &mut out),
            Err(EncodeError::Invalid(DecodeError::OutOfBounds {
                field: "page"
            })),
            "page {page}"
        );
    }
}

/// Mutants: guard of the `LogRead` arm replaced with false, and `||` with
/// `&&`. A log read wants 1 to `MAX_LOG_ENTRIES` entries.
#[test]
fn regression_231_module_protocol_log_read_max_is_bounded() {
    let mut out = [0u8; MAX_FRAME_LEN];
    for max in [1, 2, MAX_LOG_ENTRIES - 1, MAX_LOG_ENTRIES] {
        assert!(
            encode(Request::LogRead { from: 0, max }, &mut out).is_ok(),
            "{max}"
        );
    }
    for max in [0, MAX_LOG_ENTRIES + 1, u8::MAX] {
        assert_eq!(
            encode(Request::LogRead { from: 0, max }, &mut out),
            Err(EncodeError::Invalid(DecodeError::OutOfBounds {
                field: "log max"
            })),
            "max {max}"
        );
    }
    // And on decode.
    let mut bytes = request_bytes(HDR, Request::LogRead { from: 0, max: 1 });
    for max in [0u8, MAX_LOG_ENTRIES + 1] {
        let at = bytes.len() - CHECK_LEN - 1;
        bytes[at] = max;
        fix_crc(&mut bytes);
        assert_eq!(
            decode_request(&bytes).unwrap_err(),
            DecodeError::OutOfBounds { field: "log max" }
        );
    }
}

/// Mutant: `LogEntries::is_empty -> true`.
#[test]
fn regression_231_module_protocol_log_entries_is_empty_is_truthful() {
    assert!(LogEntries::EMPTY.is_empty());
    assert_eq!(LogEntries::EMPTY.len(), 0);
    let entry = LogEntry {
        index: 1,
        tick: 2,
        level: LogLevel::Info,
        code: 3,
    };
    let mut buf = [0u8; MAX_LOG_BYTES];
    let entries = LogEntries::write(&[entry], &mut buf).unwrap();
    assert!(!entries.is_empty());
    assert_eq!(entries.len(), 1);
}

/// Mutants: `ModuleBytes::len -> 0 / 1`, `is_empty -> true / false`, and the
/// `Debug` impls returning nothing.
#[test]
fn regression_231_module_protocol_module_bytes_report_their_length() {
    let empty = ModuleBytes::new(b"").unwrap();
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
    let text = ModuleBytes::new(b"abcde").unwrap();
    assert_eq!(text.len(), 5);
    assert!(!text.is_empty());
    assert_eq!(format!("{text:?}"), "ModuleBytes(untrusted, 5 bytes)");
    assert_eq!(format!("{empty:?}"), "ModuleBytes(untrusted, 0 bytes)");
}

/// Mutant: `FromModule`'s `Debug` writes nothing.
#[test]
fn regression_231_module_protocol_from_module_debug_names_itself() {
    let frame = response_bytes(
        ResponseHeader {
            session: 5,
            seq: 1,
            reply_to: 9,
        },
        Response::SafeStateEntered,
    );
    let claim = decode_response(&frame).unwrap();
    let shown = format!("{claim:?}");
    assert!(shown.starts_with("FromModule"), "{shown}");
    assert!(shown.contains("SafeStateEntered"), "{shown}");
}

/// Mutant: `HostLink::busy -> false`. A request in flight makes the link busy.
#[test]
fn regression_231_module_protocol_host_link_is_busy_with_a_request_out() {
    let mut link = HostLink::new();
    let mut out = [0u8; MAX_FRAME_LEN];
    assert!(!link.busy());
    link.open(Challenge([4; 16]), &mut out).unwrap();
    assert!(link.busy(), "an identify is outstanding");
    link.abandon();
    assert!(!link.busy());
}
