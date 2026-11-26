//! The byte framing between the host and the runner child.
//!
//! Both directions are fixed-layout, little-endian and length-prefixed, with
//! a magic number and no optional parts. A reader consumes exactly the bytes
//! it was given: a short read, an unknown tag, a count over its limit or a
//! trailing byte is an error. The host treats the child's reply as hostile
//! input, because the child runs plugin code: nothing in a reply is trusted
//! beyond its shape, and every value is checked again on the host side.
//!
//! This module is public only so tests can drive the runner directly
//! (`#[doc(hidden)]`); it is not a stable interface.

/// First four bytes of a request (host to child).
pub const REQUEST_MAGIC: [u8; 4] = *b"SBQ0";
/// First four bytes of a reply (child to host).
pub const REPLY_MAGIC: [u8; 4] = *b"SBR0";
/// Bytes one observation takes in a reply: quantity u16, mantissa i64,
/// exponent i8.
pub const OBSERVATION_BYTES: usize = 2 + 8 + 1;
/// Longest detail text a reply may carry, in bytes. It always becomes
/// [`WorldText`](sawbones_reading::WorldText) on the host.
pub const MAX_DETAIL_BYTES: usize = 512;

/// What the host asks the child to do. The child trusts these limits: they
/// come from the host, which spawned it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Fuel for each frame.
    pub fuel_per_frame: u64,
    /// Linear memory cap per frame, in bytes.
    pub memory_bytes: u64,
    /// Most observations one frame may return.
    pub max_observations: u32,
    /// The component, as `.wasm` bytes. Never precompiled code.
    pub component: Vec<u8>,
    /// The frames to decode, each in a fresh instance.
    pub frames: Vec<Vec<u8>>,
}

/// One value as the plugin returned it: an index into the manifest and an
/// integer mantissa and exponent. No tier, no text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawObservation {
    /// Index into the decoder manifest's quantity list.
    pub quantity: u16,
    /// The value is `mantissa * 10^exponent`.
    pub mantissa: i64,
    /// Power of ten.
    pub exponent: i8,
}

/// Why a decoder declined a frame, as the plugin itself said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecodeFault {
    /// The frame is not well formed.
    Malformed,
    /// The frame is well formed but this decoder does not know it.
    UnknownFrame,
    /// The decoder knows the frame but does not support it.
    Unsupported,
}

/// How a frame's run ended badly, as the runner saw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunFault {
    /// The plugin used up its fuel (a loop, or too much work).
    OutOfFuel,
    /// The plugin asked for more memory or table space than its cap.
    MemoryLimit,
    /// The plugin overflowed its wasm stack.
    StackOverflow,
    /// The plugin trapped for another reason (`unreachable`, a bad access,
    /// a division by zero).
    Trap,
    /// The plugin's return value did not lift: a bad discriminant, a list
    /// out of bounds, a misaligned pointer.
    BadOutput,
    /// The plugin returned more observations than the cap.
    TooManyObservations,
}

/// The child's answer for one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameReply {
    /// The plugin returned values.
    Values(Vec<RawObservation>),
    /// The plugin declined the frame.
    Declined(DecodeFault),
    /// The run ended badly.
    Fault(RunFault),
}

/// Why the child refused the component before running any frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefusalCode {
    /// The component has an import.
    HasImports,
    /// Compilation or validation failed.
    Compile,
    /// The `decode` export is missing or has another signature.
    WrongSignature,
    /// Instantiation failed for a reason other than a limit.
    Instantiate,
}

/// The child's whole answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// The component was accepted and each frame ran, in order.
    Ran(Vec<FrameReply>),
    /// The component was refused. `detail` is untrusted text.
    Refused {
        /// Why.
        code: RefusalCode,
        /// Diagnostic bytes, at most [`MAX_DETAIL_BYTES`].
        detail: Vec<u8>,
    },
    /// The sandbox could not be set up, so nothing ran.
    SandboxFailed {
        /// Diagnostic bytes, at most [`MAX_DETAIL_BYTES`].
        detail: Vec<u8>,
    },
}

/// A framing error. Carries no text from the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    /// The magic number is wrong.
    BadMagic,
    /// The input ended early.
    Truncated,
    /// A tag byte is not one this version knows.
    BadTag,
    /// A count or length is over its limit.
    OverLimit,
    /// Bytes are left over after a complete message.
    Trailing,
}

struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], WireError> {
        if self.bytes.len() < n {
            return Err(WireError::Truncated);
        }
        let (head, tail) = self.bytes.split_at(n);
        self.bytes = tail;
        Ok(head)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], WireError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }
    fn u8(&mut self) -> Result<u8, WireError> {
        Ok(self.array::<1>()?[0])
    }
    fn u16(&mut self) -> Result<u16, WireError> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, WireError> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, WireError> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    /// A length-prefixed byte string of at most `max` bytes.
    fn blob(&mut self, max: usize) -> Result<&'a [u8], WireError> {
        let len = usize::try_from(self.u32()?).map_err(|_| WireError::OverLimit)?;
        if len > max {
            return Err(WireError::OverLimit);
        }
        self.take(len)
    }
    fn finish(&self) -> Result<(), WireError> {
        if self.bytes.is_empty() {
            Ok(())
        } else {
            Err(WireError::Trailing)
        }
    }
}

fn put_blob(out: &mut Vec<u8>, bytes: &[u8]) {
    // Callers bound every blob far below u32::MAX.
    let len = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
}

/// Hard caps on a request, so the child's reader is bounded even before it
/// knows anything else.
pub mod caps {
    /// Largest component accepted at all, in bytes.
    pub const MAX_COMPONENT_BYTES: usize = 4 * 1024 * 1024;
    /// Largest single frame, in bytes.
    pub const MAX_FRAME_BYTES: usize = 64 * 1024;
    /// Most frames in one request.
    pub const MAX_FRAMES: usize = 64;
    /// Most observations per frame that any request may allow.
    pub const MAX_OBSERVATIONS: u32 = 1024;
    /// Largest request, in bytes.
    pub const MAX_REQUEST_BYTES: usize =
        64 + MAX_COMPONENT_BYTES + MAX_FRAMES * (4 + MAX_FRAME_BYTES);
}

impl Request {
    /// Encodes the request.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + self.component.len());
        out.extend_from_slice(&REQUEST_MAGIC);
        out.extend_from_slice(&self.fuel_per_frame.to_le_bytes());
        out.extend_from_slice(&self.memory_bytes.to_le_bytes());
        out.extend_from_slice(&self.max_observations.to_le_bytes());
        put_blob(&mut out, &self.component);
        let frames = u32::try_from(self.frames.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&frames.to_le_bytes());
        for frame in &self.frames {
            put_blob(&mut out, frame);
        }
        out
    }

    /// Decodes a request, refusing anything over [`caps`].
    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut r = Reader { bytes };
        if r.array::<4>()? != REQUEST_MAGIC {
            return Err(WireError::BadMagic);
        }
        let fuel_per_frame = r.u64()?;
        let memory_bytes = r.u64()?;
        let max_observations = r.u32()?;
        if max_observations > caps::MAX_OBSERVATIONS {
            return Err(WireError::OverLimit);
        }
        let component = r.blob(caps::MAX_COMPONENT_BYTES)?.to_vec();
        let count = usize::try_from(r.u32()?).map_err(|_| WireError::OverLimit)?;
        if count > caps::MAX_FRAMES {
            return Err(WireError::OverLimit);
        }
        let mut frames = Vec::with_capacity(count);
        for _ in 0..count {
            frames.push(r.blob(caps::MAX_FRAME_BYTES)?.to_vec());
        }
        r.finish()?;
        Ok(Self {
            fuel_per_frame,
            memory_bytes,
            max_observations,
            component,
            frames,
        })
    }
}

const TAG_RAN: u8 = 0;
const TAG_REFUSED: u8 = 1;
const TAG_SANDBOX: u8 = 2;

const FRAME_VALUES: u8 = 0;
const FRAME_DECLINED: u8 = 1;
const FRAME_FAULT: u8 = 2;

fn decode_fault_code(f: DecodeFault) -> u8 {
    match f {
        DecodeFault::Malformed => 0,
        DecodeFault::UnknownFrame => 1,
        DecodeFault::Unsupported => 2,
    }
}

fn decode_fault_from(code: u8) -> Result<DecodeFault, WireError> {
    Ok(match code {
        0 => DecodeFault::Malformed,
        1 => DecodeFault::UnknownFrame,
        2 => DecodeFault::Unsupported,
        _ => return Err(WireError::BadTag),
    })
}

fn run_fault_code(f: RunFault) -> u8 {
    match f {
        RunFault::OutOfFuel => 0,
        RunFault::MemoryLimit => 1,
        RunFault::StackOverflow => 2,
        RunFault::Trap => 3,
        RunFault::BadOutput => 4,
        RunFault::TooManyObservations => 5,
    }
}

fn run_fault_from(code: u8) -> Result<RunFault, WireError> {
    Ok(match code {
        0 => RunFault::OutOfFuel,
        1 => RunFault::MemoryLimit,
        2 => RunFault::StackOverflow,
        3 => RunFault::Trap,
        4 => RunFault::BadOutput,
        5 => RunFault::TooManyObservations,
        _ => return Err(WireError::BadTag),
    })
}

fn refusal_code(c: RefusalCode) -> u8 {
    match c {
        RefusalCode::HasImports => 0,
        RefusalCode::Compile => 1,
        RefusalCode::WrongSignature => 2,
        RefusalCode::Instantiate => 3,
    }
}

fn refusal_from(code: u8) -> Result<RefusalCode, WireError> {
    Ok(match code {
        0 => RefusalCode::HasImports,
        1 => RefusalCode::Compile,
        2 => RefusalCode::WrongSignature,
        3 => RefusalCode::Instantiate,
        _ => return Err(WireError::BadTag),
    })
}

fn bounded_detail(detail: &[u8]) -> &[u8] {
    &detail[..detail.len().min(MAX_DETAIL_BYTES)]
}

/// The longest reply the host will read for a request with `frames` frames
/// and `max_observations` per frame. Anything longer is refused unread.
pub fn max_reply_bytes(frames: usize, max_observations: u32) -> usize {
    let per_frame = 1 + 4 + (max_observations as usize) * OBSERVATION_BYTES;
    let ran = 4 + 1 + 4 + frames * per_frame;
    let other = 4 + 1 + 1 + 4 + MAX_DETAIL_BYTES;
    ran.max(other)
}

impl Reply {
    /// Encodes the reply. Detail text is cut to [`MAX_DETAIL_BYTES`].
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&REPLY_MAGIC);
        match self {
            Reply::Ran(frames) => {
                out.push(TAG_RAN);
                let n = u32::try_from(frames.len()).unwrap_or(u32::MAX);
                out.extend_from_slice(&n.to_le_bytes());
                for frame in frames {
                    match frame {
                        FrameReply::Values(values) => {
                            out.push(FRAME_VALUES);
                            let n = u32::try_from(values.len()).unwrap_or(u32::MAX);
                            out.extend_from_slice(&n.to_le_bytes());
                            for v in values {
                                out.extend_from_slice(&v.quantity.to_le_bytes());
                                out.extend_from_slice(&v.mantissa.to_le_bytes());
                                out.extend_from_slice(&v.exponent.to_le_bytes());
                            }
                        }
                        FrameReply::Declined(f) => {
                            out.push(FRAME_DECLINED);
                            out.push(decode_fault_code(*f));
                        }
                        FrameReply::Fault(f) => {
                            out.push(FRAME_FAULT);
                            out.push(run_fault_code(*f));
                        }
                    }
                }
            }
            Reply::Refused { code, detail } => {
                out.push(TAG_REFUSED);
                out.push(refusal_code(*code));
                put_blob(&mut out, bounded_detail(detail));
            }
            Reply::SandboxFailed { detail } => {
                out.push(TAG_SANDBOX);
                put_blob(&mut out, bounded_detail(detail));
            }
        }
        out
    }

    /// Decodes a reply to a request of `frames` frames with at most
    /// `max_observations` per frame. A reply for another number of frames,
    /// or with more observations in a frame, is refused.
    pub fn decode(bytes: &[u8], frames: usize, max_observations: u32) -> Result<Self, WireError> {
        let mut r = Reader { bytes };
        if r.array::<4>()? != REPLY_MAGIC {
            return Err(WireError::BadMagic);
        }
        let reply = match r.u8()? {
            TAG_RAN => {
                let n = usize::try_from(r.u32()?).map_err(|_| WireError::OverLimit)?;
                if n != frames {
                    return Err(WireError::OverLimit);
                }
                let mut out = Vec::with_capacity(n);
                for _ in 0..n {
                    out.push(match r.u8()? {
                        FRAME_VALUES => {
                            let count = r.u32()?;
                            if count > max_observations {
                                return Err(WireError::OverLimit);
                            }
                            let mut values = Vec::with_capacity(count as usize);
                            for _ in 0..count {
                                let quantity = r.u16()?;
                                let mantissa = i64::from_le_bytes(r.array()?);
                                let exponent = i8::from_le_bytes(r.array()?);
                                values.push(RawObservation {
                                    quantity,
                                    mantissa,
                                    exponent,
                                });
                            }
                            FrameReply::Values(values)
                        }
                        FRAME_DECLINED => FrameReply::Declined(decode_fault_from(r.u8()?)?),
                        FRAME_FAULT => FrameReply::Fault(run_fault_from(r.u8()?)?),
                        _ => return Err(WireError::BadTag),
                    });
                }
                Reply::Ran(out)
            }
            TAG_REFUSED => {
                let code = refusal_from(r.u8()?)?;
                let detail = r.blob(MAX_DETAIL_BYTES)?.to_vec();
                Reply::Refused { code, detail }
            }
            TAG_SANDBOX => Reply::SandboxFailed {
                detail: r.blob(MAX_DETAIL_BYTES)?.to_vec(),
            },
            _ => return Err(WireError::BadTag),
        };
        r.finish()?;
        Ok(reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_reply() -> Reply {
        Reply::Ran(vec![
            FrameReply::Values(vec![RawObservation {
                quantity: 3,
                mantissa: -1823,
                exponent: -1,
            }]),
            FrameReply::Declined(DecodeFault::UnknownFrame),
            FrameReply::Fault(RunFault::OutOfFuel),
        ])
    }

    #[test]
    fn request_round_trips() {
        let req = Request {
            fuel_per_frame: 7,
            memory_bytes: 1 << 20,
            max_observations: 4,
            component: vec![0, 97, 115, 109],
            frames: vec![vec![1, 2], vec![]],
        };
        assert_eq!(Request::decode(&req.encode()).unwrap(), req);
    }

    #[test]
    fn reply_round_trips() {
        let reply = sample_reply();
        assert_eq!(Reply::decode(&reply.encode(), 3, 4).unwrap(), reply);
        let refused = Reply::Refused {
            code: RefusalCode::HasImports,
            detail: b"x".to_vec(),
        };
        assert_eq!(Reply::decode(&refused.encode(), 3, 4).unwrap(), refused);
    }

    #[test]
    fn reply_with_wrong_frame_count_or_too_many_values_is_refused() {
        let bytes = sample_reply().encode();
        assert_eq!(Reply::decode(&bytes, 2, 4), Err(WireError::OverLimit));
        assert_eq!(Reply::decode(&bytes, 3, 0), Err(WireError::OverLimit));
    }

    #[test]
    fn every_truncation_and_every_extra_byte_is_refused() {
        let bytes = sample_reply().encode();
        for cut in 0..bytes.len() {
            assert!(Reply::decode(&bytes[..cut], 3, 4).is_err(), "cut at {cut}");
        }
        let mut long = bytes.clone();
        long.push(0);
        assert_eq!(Reply::decode(&long, 3, 4), Err(WireError::Trailing));
    }

    #[test]
    fn random_bytes_never_panic_and_rarely_parse() {
        // A fixed xorshift stream, so the test is deterministic.
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..20_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let len = (x % 64) as usize;
            let mut buf = Vec::with_capacity(len + 4);
            buf.extend_from_slice(&REPLY_MAGIC);
            let mut y = x;
            for _ in 0..len {
                y = y.wrapping_mul(6364136223846793005).wrapping_add(1);
                buf.push((y >> 33) as u8);
            }
            if let Ok(reply) = Reply::decode(&buf, 1, 2) {
                // Anything that parses re-encodes to the same bytes: the
                // format has one spelling per message.
                assert_eq!(reply.encode(), buf);
            }
        }
    }
}
