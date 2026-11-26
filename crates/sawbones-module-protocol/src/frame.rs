//! The frame: header, payload and check.

use crate::crc::crc32c;
use crate::message::{Request, RequestKind, Response, ResponseKind};
use crate::untrusted::FromModule;
use crate::wire::{Reader, Writer};
use crate::{DecodeError, EncodeError};

/// The first two bytes of every frame: `"SB"`.
pub const MAGIC: [u8; 2] = *b"SB";

/// The protocol version this build speaks: the third byte of every frame.
pub const VERSION: u8 = 1;

/// The header's length: magic (2), version (1), kind (1), payload length
/// (2), session (8), sequence number (4), reply-to (4).
pub const HEADER_LEN: usize = 22;

/// The length of the check value after the payload.
pub const CHECK_LEN: usize = 4;

/// The longest payload a frame may declare.
pub const MAX_PAYLOAD_LEN: usize = 320;

/// The shortest frame: a header and a check, with no payload.
pub const MIN_FRAME_LEN: usize = HEADER_LEN + CHECK_LEN;

/// The longest frame. A buffer of this size holds any frame.
pub const MAX_FRAME_LEN: usize = HEADER_LEN + MAX_PAYLOAD_LEN + CHECK_LEN;

/// How many leading bytes [`frame_len`] needs: magic, version, kind and
/// payload length.
pub const PREFIX_LEN: usize = 6;

/// A request's header fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestHeader {
    /// The session the module opened, or 0 for [`Request::Identify`], which
    /// opens one.
    pub session: u64,
    /// The host's sequence number. Each request carries a higher one than the
    /// last.
    pub seq: u32,
}

/// A response's header fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResponseHeader {
    /// The session.
    pub session: u64,
    /// The module's sequence number, rising within the session.
    pub seq: u32,
    /// The sequence number of the request this answers.
    pub reply_to: u32,
}

/// A request with its header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestFrame<'a> {
    /// The header.
    pub header: RequestHeader,
    /// The request.
    pub request: Request<'a>,
}

/// A response with its header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResponseFrame<'a> {
    /// The header.
    pub header: ResponseHeader,
    /// The response.
    pub response: Response<'a>,
}

struct Raw<'a> {
    kind: u8,
    session: u64,
    seq: u32,
    reply_to: u32,
    payload: &'a [u8],
}

/// The total length of the frame that starts with `prefix`, from its first
/// [`PREFIX_LEN`] bytes, so a byte-stream transport can read exactly one
/// frame.
///
/// # Errors
///
/// [`DecodeError::Truncated`] with fewer than [`PREFIX_LEN`] bytes, and the
/// magic, version and length errors [`decode_request`] gives.
pub fn frame_len(prefix: &[u8]) -> Result<usize, DecodeError> {
    let head = match prefix.get(..PREFIX_LEN) {
        Some(head) => head,
        None => {
            return Err(DecodeError::Truncated {
                expected: PREFIX_LEN,
                actual: prefix.len(),
            })
        }
    };
    let mut r = Reader::new(head);
    if r.array::<2>()? != MAGIC {
        return Err(DecodeError::BadMagic);
    }
    let version = r.u8()?;
    if version != VERSION {
        return Err(DecodeError::UnsupportedVersion(version));
    }
    let _kind = r.u8()?;
    let declared = usize::from(r.u16()?);
    if declared > MAX_PAYLOAD_LEN {
        return Err(DecodeError::Oversized { declared });
    }
    Ok(HEADER_LEN + declared + CHECK_LEN)
}

fn split(bytes: &[u8]) -> Result<Raw<'_>, DecodeError> {
    if bytes.len() < MIN_FRAME_LEN {
        // Report a bad magic or version first when the bytes are there, so a
        // short frame of the wrong protocol says so.
        if bytes.len() >= PREFIX_LEN {
            frame_len(bytes)?;
        }
        return Err(DecodeError::Truncated {
            expected: MIN_FRAME_LEN,
            actual: bytes.len(),
        });
    }
    let total = frame_len(bytes)?;
    if bytes.len() < total {
        return Err(DecodeError::Truncated {
            expected: total,
            actual: bytes.len(),
        });
    }
    if bytes.len() > total {
        return Err(DecodeError::TrailingBytes {
            extra: bytes.len() - total,
        });
    }
    let (body, check) = bytes.split_at(total - CHECK_LEN);
    let mut check_reader = Reader::new(check);
    let received = check_reader.u32()?;
    let computed = crc32c(body);
    if computed != received {
        return Err(DecodeError::BadChecksum { computed, received });
    }
    let (header, payload) = body.split_at(HEADER_LEN);
    let mut r = Reader::new(header);
    let _magic_version = r.array::<3>()?;
    let kind = r.u8()?;
    let _len = r.u16()?;
    Ok(Raw {
        kind,
        session: r.u64()?,
        seq: r.u32()?,
        reply_to: r.u32()?,
        payload,
    })
}

/// Decodes one request frame, as a module receives it. Total: any byte
/// string decodes or returns a [`DecodeError`], and none panics.
///
/// This checks the frame and the message only. Sessions, sequence numbers
/// and authority are [`ModuleLink::accept`](crate::ModuleLink::accept)'s.
///
/// # Errors
///
/// The first check that failed, in this order: length, magic, version,
/// declared length, check value, kind, direction, reply-to, payload.
pub fn decode_request(bytes: &[u8]) -> Result<RequestFrame<'_>, DecodeError> {
    let raw = split(bytes)?;
    let kind = match RequestKind::from_u8(raw.kind) {
        Some(kind) => kind,
        None if ResponseKind::from_u8(raw.kind).is_some() => {
            return Err(DecodeError::WrongDirection(raw.kind))
        }
        None => return Err(DecodeError::UnknownKind(raw.kind)),
    };
    if raw.reply_to != 0 {
        return Err(DecodeError::ReplyToInRequest);
    }
    Ok(RequestFrame {
        header: RequestHeader {
            session: raw.session,
            seq: raw.seq,
        },
        request: Request::decode_payload(kind, raw.payload)?,
    })
}

/// Decodes one response frame, as the host receives it. Total, like
/// [`decode_request`]. The result is [`FromModule`]: what the module claims.
///
/// # Errors
///
/// As [`decode_request`], without the reply-to check.
pub fn decode_response(bytes: &[u8]) -> Result<FromModule<ResponseFrame<'_>>, DecodeError> {
    let raw = split(bytes)?;
    let kind = match ResponseKind::from_u8(raw.kind) {
        Some(kind) => kind,
        None if RequestKind::from_u8(raw.kind).is_some() => {
            return Err(DecodeError::WrongDirection(raw.kind))
        }
        None => return Err(DecodeError::UnknownKind(raw.kind)),
    };
    Ok(FromModule::new(ResponseFrame {
        header: ResponseHeader {
            session: raw.session,
            seq: raw.seq,
            reply_to: raw.reply_to,
        },
        response: Response::decode_payload(kind, raw.payload)?,
    }))
}

fn encode_frame(
    kind: u8,
    session: u64,
    seq: u32,
    reply_to: u32,
    out: &mut [u8],
    payload: impl FnOnce(&mut Writer<'_>) -> Result<(), EncodeError>,
) -> Result<usize, EncodeError> {
    let payload_len = {
        let region = match out.get_mut(HEADER_LEN..) {
            Some(region) => region,
            None => {
                return Err(EncodeError::BufferTooSmall {
                    needed: MIN_FRAME_LEN,
                })
            }
        };
        let limit = region.len().min(MAX_PAYLOAD_LEN);
        let mut w = Writer::new(region.get_mut(..limit).unwrap_or(&mut []));
        match payload(&mut w) {
            Ok(()) => w.len(),
            Err(EncodeError::BufferTooSmall { needed }) if limit == MAX_PAYLOAD_LEN => {
                // The payload itself is over the bound, not the buffer.
                return Err(EncodeError::Invalid(DecodeError::Oversized {
                    declared: needed,
                }));
            }
            Err(EncodeError::BufferTooSmall { needed }) => {
                return Err(EncodeError::BufferTooSmall {
                    needed: HEADER_LEN + needed + CHECK_LEN,
                })
            }
            Err(e) => return Err(e),
        }
    };
    let total = HEADER_LEN + payload_len + CHECK_LEN;
    if out.len() < total {
        return Err(EncodeError::BufferTooSmall { needed: total });
    }
    // At most MAX_PAYLOAD_LEN, which fits in a u16.
    let len16 = u16::try_from(payload_len).map_err(|_| {
        EncodeError::Invalid(DecodeError::Oversized {
            declared: payload_len,
        })
    })?;
    {
        let header = out.get_mut(..HEADER_LEN).unwrap_or(&mut []);
        let mut w = Writer::new(header);
        w.put(&MAGIC)?;
        w.u8(VERSION)?;
        w.u8(kind)?;
        w.u16(len16)?;
        w.u64(session)?;
        w.u32(seq)?;
        w.u32(reply_to)?;
    }
    let body_end = HEADER_LEN + payload_len;
    let crc = crc32c(out.get(..body_end).unwrap_or(&[]));
    let mut w = Writer::new(out.get_mut(body_end..total).unwrap_or(&mut []));
    w.u32(crc)?;
    Ok(total)
}

/// Encodes a request frame into `out` and returns its length.
///
/// # Errors
///
/// [`EncodeError::BufferTooSmall`], or [`EncodeError::Invalid`] for a
/// request that [`decode_request`] would refuse (a chunk out of range, a
/// page out of bounds), so nothing undecodable is ever sent.
pub fn encode_request(frame: &RequestFrame<'_>, out: &mut [u8]) -> Result<usize, EncodeError> {
    encode_frame(
        frame.request.kind().to_u8(),
        frame.header.session,
        frame.header.seq,
        0,
        out,
        |w| frame.request.encode_payload(w),
    )
}

/// Encodes a response frame into `out` and returns its length.
///
/// # Errors
///
/// As [`encode_request`].
pub fn encode_response(frame: &ResponseFrame<'_>, out: &mut [u8]) -> Result<usize, EncodeError> {
    encode_frame(
        frame.response.kind().to_u8(),
        frame.header.session,
        frame.header.seq,
        frame.header.reply_to,
        out,
        |w| frame.response.encode_payload(w),
    )
}
