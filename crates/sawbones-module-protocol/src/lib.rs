//! # sawbones-module-protocol: PROVISIONAL
//!
//! **This crate is provisional. The Sawbones design is still under review, and
//! every type here may change or be replaced without notice.**
//!
//! The wire protocol between the bench supervisor (the host, a Raspberry Pi)
//! and its hardware modules (RP2350 microcontrollers), #98. The same types
//! compile into the supervisor and into module firmware, so the two cannot
//! drift. It reads no clock, allocates nothing and is `no_std`.
//!
//! ## The calls
//!
//! Every built module answers the same [`Request`]s: identify, describe,
//! health, self-test, time, config read and write, log, stage an update,
//! reset, safe state, and actuate. Each has one expected [`Response`]
//! ([`RequestKind::expected_reply`]), or [`Response::Refused`].
//!
//! ## The frame
//!
//! One frame is [`HEADER_LEN`] bytes of header, a payload of at most
//! [`MAX_PAYLOAD_LEN`] bytes, and a 4-byte check. Integers are
//! little-endian.
//!
//! | Offset | Size | Field |
//! | --- | --- | --- |
//! | 0 | 2 | [`MAGIC`], `"SB"` |
//! | 2 | 1 | [`VERSION`] |
//! | 3 | 1 | kind: a [`RequestKind`] (`0x01` to `0x7f`) or a [`ResponseKind`] (`0x81` to `0xff`) |
//! | 4 | 2 | payload length |
//! | 6 | 8 | session |
//! | 14 | 4 | sequence number |
//! | 18 | 4 | reply-to: the request's sequence number; 0 in a request |
//! | 22 | n | payload |
//! | 22 + n | 4 | CRC-32C ([`crc32c`]) of every byte before it |
//!
//! The check catches line noise, not an attacker: anyone can compute a CRC.
//! What resists an attacker is the session, the sequence numbers, the
//! identity signature and the interlock lease, below. [`frame_len`] reads the
//! total length from the first [`PREFIX_LEN`] bytes, so a byte-stream
//! transport can read exactly one frame. The golden tests in
//! `tests/pinned.rs` pin the encoding, so the format cannot drift silently.
//!
//! ## Decoding is total
//!
//! [`decode_request`] and [`decode_response`] take any byte string and
//! either return a typed message or a [`DecodeError`]. They never panic and
//! never read past the input. Decoding is canonical too: bytes that decode
//! re-encode to exactly the same bytes, so one message has one encoding.
//! Every size is bounded on the wire: chunks, manifest pages, log batches.
//!
//! ## Sessions, sequence numbers and replay
//!
//! The module, which receives commands, picks the session. A host opens one
//! with [`Request::Identify`] in session 0, carrying a fresh [`Challenge`].
//! The module answers from a new non-zero session id drawn from its own
//! hardware random number generator. Every later request must carry that
//! session and a sequence number above the last one accepted, so a frame
//! captured from an old session, or sent twice, is dropped
//! ([`ModuleLink::accept`]). The host side ([`HostLink`]) checks the same of
//! the module: one request in flight, each reply naming it, module sequence
//! numbers rising within the session.
//!
//! Opening a session ends the previous one. The firmware treats that as a
//! reason to go to its safe state ([`Accepted::new_session`]).
//!
//! ## What the module says is a claim
//!
//! **The Pi carries data but does not vouch for it.** Everything decoded from
//! a module reaches the host wrapped in [`FromModule`], which has no public
//! constructor and no `Deref`: the host reads it with
//! [`FromModule::claim`] and knows it is reading a claim. Text from a module
//! is [`ModuleBytes`], with no `Display` and no `&str` view, to be turned into
//! fenced world text by the host. Log entries are codes, never text.
//!
//! A module's identity is a signature, not a recited id (#98, review round
//! 2). [`IdentityClaim`] carries the module's id, its [`Capabilities`] (with
//! `can_emit` for anything that transmits), the digest of its manifest and
//! of its firmware, and an Ed25519 signature over all of them, the host's
//! challenge and the session ([`IdentityClaim::signed_bytes`]). With the
//! `verify` feature, `IdentityClaim::verify` checks it against the key
//! enrolled for that module and returns a `VerifiedIdentity`, which has
//! private fields and no other constructor. A self-test passing raises no
//! trust: trust returns after the known-answer check against the reference
//! (#178), which is not this crate's to decide.
//!
//! ## Commands with a hazard
//!
//! A config write or an actuation carries an [`Authority`], which names the
//! interlock's hazard class and, for class 3, the interlock's own
//! [`LeaseTarget`]: the port and the digest of exactly this action on exactly
//! this module ([`action_digest`]). Class 4 has no encoding. The module checks
//! the authority against its own [`ClassTable`] in [`ModuleLink::accept`]:
//! the host's word for the class counts for nothing, a lease for another
//! action is refused, and an unknown key or channel is refused. A firmware
//! activation carries a trust-root release approval ([`ReleaseApproval`]),
//! which the firmware checks with `sawbones-approvals`.
//!
//! ## The firmware's half of the contract
//!
//! This crate is pure, so some rules are for the firmware that calls it:
//!
//! - draw the session id from the hardware random number generator, never a
//!   counter; a zero is refused;
//! - go to the safe state when a new session opens, on
//!   [`Request::SafeState`], and when no request has been accepted for the
//!   module's link timeout;
//! - for a class 2 command, check rail A is sensed live; for class 3, that
//!   rail B is sensed live on the lease's port. The supervisor reports the
//!   use to the interlock with `Event::LeaseUsed` for
//!   [`Request::lease_target`];
//! - before [`ResetMode::ActivateStaged`], verify the release approval and
//!   that it names the staged image's digest.
//!
//! What is deliberately not here yet: binding a module to its hub port and
//! refusing duplicate ids, which is the supervisor's registry (#100); the
//! manifest's schema; signatures on readings; and a timeout, since the crate
//! has no clock.

#![no_std]
#![forbid(unsafe_code)]

mod authority;
mod crc;
mod frame;
mod identity;
mod link;
mod message;
mod untrusted;
mod wire;

pub use authority::{action_digest, ActionBody, Authority, Grant, ACTION_DOMAIN};
pub use crc::crc32c;
pub use frame::{
    decode_request, decode_response, encode_request, encode_response, frame_len, RequestFrame,
    RequestHeader, ResponseFrame, ResponseHeader, CHECK_LEN, HEADER_LEN, MAGIC, MAX_FRAME_LEN,
    MAX_PAYLOAD_LEN, MIN_FRAME_LEN, PREFIX_LEN, VERSION,
};
pub use identity::{
    IdentityClaim, IdentitySignature, ManifestDigest, ManifestHasher, IDENTITY_DOMAIN,
    IDENTITY_SIGNED_LEN, MANIFEST_DOMAIN,
};
#[cfg(feature = "verify")]
pub use identity::{IdentityError, ModuleKey, VerifiedIdentity};
pub use link::{
    Accepted, ClassTable, Dropped, HostError, HostLink, LinkCounters, ModuleLink, Rejection,
};
pub use message::{
    Actuate, Capabilities, Challenge, Channel, ConfigKey, ConfigWrite, DescriptionPage,
    HealthReport, HealthStatus, ImageDigest, LogEntries, LogEntry, LogLevel, RefusalCode,
    ReleaseApproval, Request, RequestKind, ResetMode, Response, ResponseKind, SelfTestOutcome,
    SelfTestReport, TimeReport, UpdateChunk, APPROVAL_MESSAGE_LEN, APPROVAL_SIGNATURE_LEN,
    LOG_ENTRY_LEN, MAX_CHUNK_LEN, MAX_LOG_BYTES, MAX_LOG_ENTRIES, MAX_MANIFEST_PAGES, MAX_PAGE_LEN,
};
pub use untrusted::{FromModule, ModuleBytes};

pub use sawbones_interlock::{ActionDigest, HazardClass, LeaseTarget, Port};

/// A module, by the id it was enrolled under. The same numbering as
/// `sawbones-approvals`' `ModuleId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId(pub u32);

/// Why bytes did not decode. Every input that does not decode gets one of
/// these; none panics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecodeError {
    /// Fewer bytes than the frame needs: `expected` is the minimum frame
    /// length, or the length the header declares.
    Truncated {
        /// How many bytes the frame needs.
        expected: usize,
        /// How many there are.
        actual: usize,
    },
    /// More bytes than the header declares.
    TrailingBytes {
        /// How many bytes are left over.
        extra: usize,
    },
    /// The frame does not start with [`MAGIC`].
    BadMagic,
    /// A version this build does not speak.
    UnsupportedVersion(u8),
    /// The header declares a payload longer than [`MAX_PAYLOAD_LEN`].
    Oversized {
        /// The declared payload length.
        declared: usize,
    },
    /// The check value does not match.
    BadChecksum {
        /// The CRC-32C of the bytes received.
        computed: u32,
        /// The check value in the frame.
        received: u32,
    },
    /// No message has this kind byte.
    UnknownKind(u8),
    /// A response kind where a request was expected, or the reverse.
    WrongDirection(u8),
    /// A request with a non-zero reply-to field.
    ReplyToInRequest,
    /// The payload ended before the message did.
    PayloadTooShort,
    /// The payload has bytes after the message.
    PayloadTrailing,
    /// A boolean byte other than 0 or 1.
    BadBool,
    /// An enumerated field with a value that names nothing.
    BadTag {
        /// Which field.
        field: &'static str,
        /// The value received.
        value: u8,
    },
    /// An authority for hazard class 4, which has no rail and is never
    /// offered.
    ClassNeverPermitted,
    /// A length field over its bound, or a field out of its range.
    OutOfBounds {
        /// Which field.
        field: &'static str,
    },
}

/// Why a message could not be encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EncodeError {
    /// The output buffer is too small. [`MAX_FRAME_LEN`] always suffices.
    BufferTooSmall {
        /// How many bytes were needed at least.
        needed: usize,
    },
    /// The message would not decode: the same check [`decode_request`] or
    /// [`decode_response`] makes failed, so it is refused before it is sent.
    Invalid(DecodeError),
}

impl From<DecodeError> for EncodeError {
    fn from(e: DecodeError) -> Self {
        EncodeError::Invalid(e)
    }
}
