//! Sessions and sequence numbers on both ends of the link.

use core::num::NonZeroU64;

use crate::authority::action_digest;
use crate::frame::{
    decode_request, decode_response, encode_request, encode_response, RequestFrame, RequestHeader,
    ResponseFrame, ResponseHeader,
};
use crate::message::{
    Challenge, Channel, ConfigKey, RefusalCode, Request, RequestKind, Response, ResponseKind,
};
use crate::untrusted::FromModule;
use crate::{Authority, DecodeError, EncodeError, HazardClass, ModuleId};

/// The module's own table of hazard classes, from its manifest. The module
/// decides a command's class; the host's authority only has to reach it.
pub trait ClassTable {
    /// The class of writing `key`, or `None` if there is no such key.
    fn config_class(&self, key: ConfigKey) -> Option<HazardClass>;
    /// The class of driving `channel`, or `None` if there is no such channel.
    fn channel_class(&self, channel: Channel) -> Option<HazardClass>;
}

/// Counts the module keeps for its health report. They saturate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LinkCounters {
    /// Frames that did not decode.
    pub bad_frames: u32,
    /// Frames dropped for their session or sequence number.
    pub dropped: u32,
    /// Requests refused for their authority or key.
    pub refused: u32,
}

/// Why a module dropped a frame without answering it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dropped {
    /// It did not decode.
    Malformed(DecodeError),
    /// No session is open, and it is not an [`Request::Identify`].
    NoSession,
    /// It names a session other than the open one, or an
    /// [`Request::Identify`] names any session but 0.
    WrongSession,
    /// Its sequence number is not above the last one accepted: a duplicate,
    /// a replay or a reordered frame.
    Replayed {
        /// The frame's sequence number.
        seq: u32,
        /// The last one accepted in this session.
        last: u32,
    },
    /// The random number generator gave 0 for a new session. No session is
    /// opened and the old one stays.
    ZeroNonce,
}

/// Why a module did not act on a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rejection {
    /// Drop it: send nothing back.
    Drop(Dropped),
    /// Answer it with [`Response::Refused`] carrying `code`, in reply to
    /// `reply_to`. The sequence number is spent, so the same frame again is
    /// a replay.
    Refuse {
        /// The request's sequence number.
        reply_to: u32,
        /// Why.
        code: RefusalCode,
    },
}

/// A request the module may act on: in the open session, fresh, and with an
/// authority that reaches the class the module's own table gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Accepted<'a> {
    /// The request.
    pub request: Request<'a>,
    /// Its sequence number, for the reply.
    pub seq: u32,
    /// The hazard class the module's table gives it.
    pub class: HazardClass,
    /// True if this request opened a new session. The firmware goes to its
    /// safe state before answering: whatever the old session set up is
    /// void.
    pub new_session: bool,
}

/// The module's end of the link: one open session, the last host sequence
/// number accepted in it, and the module's own sequence numbers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleLink {
    module: ModuleId,
    session: Option<NonZeroU64>,
    last_seq: u32,
    next_reply_seq: u32,
    counters: LinkCounters,
}

impl ModuleLink {
    /// A link for `module` with no session open.
    #[must_use]
    pub const fn new(module: ModuleId) -> Self {
        ModuleLink {
            module,
            session: None,
            last_seq: 0,
            next_reply_seq: 1,
            counters: LinkCounters {
                bad_frames: 0,
                dropped: 0,
                refused: 0,
            },
        }
    }

    /// The open session, if any.
    #[must_use]
    pub const fn session(&self) -> Option<NonZeroU64> {
        self.session
    }

    /// The counts for the health report.
    #[must_use]
    pub const fn counters(&self) -> LinkCounters {
        self.counters
    }

    /// Ends the session: after a reset, or the firmware's link timeout.
    pub fn end_session(&mut self) {
        self.session = None;
    }

    /// Judges one frame from the host.
    ///
    /// - A frame that does not decode is dropped.
    /// - [`Request::Identify`] in session 0 opens a new session with the id
    ///   `nonce` gives (the hardware random number generator; called only
    ///   then), and ends the old one.
    /// - Any other request must name the open session and carry a sequence
    ///   number above the last one accepted, or it is dropped.
    /// - A config write or an actuation must carry an authority that reaches
    ///   the class `table` gives it, and a class 3 lease must be for exactly
    ///   this action on this module, or it is refused. Class 4 is always
    ///   refused, as is an unknown key or channel.
    ///
    /// # Errors
    ///
    /// A [`Rejection`]: drop the frame, or answer it with a refusal.
    pub fn accept<'a>(
        &mut self,
        bytes: &'a [u8],
        nonce: impl FnOnce() -> u64,
        table: &impl ClassTable,
    ) -> Result<Accepted<'a>, Rejection> {
        let RequestFrame { header, request } = match decode_request(bytes) {
            Ok(frame) => frame,
            Err(e) => {
                self.counters.bad_frames = self.counters.bad_frames.saturating_add(1);
                return Err(Rejection::Drop(Dropped::Malformed(e)));
            }
        };
        let new_session = match request {
            Request::Identify { .. } => {
                if header.session != 0 {
                    return Err(self.drop(Dropped::WrongSession));
                }
                let Some(session) = NonZeroU64::new(nonce()) else {
                    return Err(self.drop(Dropped::ZeroNonce));
                };
                self.session = Some(session);
                self.last_seq = header.seq;
                self.next_reply_seq = 1;
                true
            }
            _ => {
                let Some(session) = self.session else {
                    return Err(self.drop(Dropped::NoSession));
                };
                if header.session != session.get() {
                    return Err(self.drop(Dropped::WrongSession));
                }
                if header.seq <= self.last_seq {
                    return Err(self.drop(Dropped::Replayed {
                        seq: header.seq,
                        last: self.last_seq,
                    }));
                }
                self.last_seq = header.seq;
                false
            }
        };
        match self.classify(&request, table) {
            Ok(class) => Ok(Accepted {
                request,
                seq: header.seq,
                class,
                new_session,
            }),
            Err(code) => {
                self.counters.refused = self.counters.refused.saturating_add(1);
                Err(Rejection::Refuse {
                    reply_to: header.seq,
                    code,
                })
            }
        }
    }

    fn drop(&mut self, why: Dropped) -> Rejection {
        self.counters.dropped = self.counters.dropped.saturating_add(1);
        Rejection::Drop(why)
    }

    fn classify(
        &self,
        request: &Request<'_>,
        table: &impl ClassTable,
    ) -> Result<HazardClass, RefusalCode> {
        if let Some(class) = request.fixed_class() {
            return Ok(class);
        }
        let (class, authority) = match request {
            Request::ConfigWrite(w) => (table.config_class(w.key), w.authority),
            Request::Actuate(a) => (table.channel_class(a.channel), a.authority),
            // Every other request has a fixed class.
            _ => (None, Authority::None),
        };
        let class = class.ok_or(RefusalCode::UnknownKey)?;
        if matches!(class, HazardClass::Class4) {
            return Err(RefusalCode::ClassNeverPermitted);
        }
        if !authority.permits(class) {
            return Err(RefusalCode::AuthorityTooLow);
        }
        if let (Authority::Lease(target), Some(body)) = (authority, request.action_body()) {
            if target.action_digest != action_digest(self.module, body) {
                return Err(RefusalCode::LeaseNotForThisAction);
            }
        }
        Ok(class)
    }

    /// Encodes `response` in reply to request `reply_to`, in the open
    /// session, with the module's next sequence number.
    ///
    /// # Errors
    ///
    /// [`EncodeError`] if it does not fit or would not decode, or
    /// [`EncodeError::Invalid`] with [`DecodeError::OutOfBounds`] if no
    /// session is open or the module's sequence numbers are used up.
    pub fn reply(
        &mut self,
        reply_to: u32,
        response: &Response<'_>,
        out: &mut [u8],
    ) -> Result<usize, EncodeError> {
        let no_session = EncodeError::Invalid(DecodeError::OutOfBounds { field: "session" });
        let session = self.session.ok_or(no_session)?;
        let seq = self.next_reply_seq;
        let next = seq
            .checked_add(1)
            .ok_or(EncodeError::Invalid(DecodeError::OutOfBounds {
                field: "module seq",
            }))?;
        let len = encode_response(
            &ResponseFrame {
                header: ResponseHeader {
                    session: session.get(),
                    seq,
                    reply_to,
                },
                response: *response,
            },
            out,
        )?;
        self.next_reply_seq = next;
        Ok(len)
    }
}

/// Why the host did not take a frame or could not send one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostError {
    /// It did not decode.
    Malformed(DecodeError),
    /// No request is outstanding: a late reply to an abandoned request, or
    /// one nobody asked for.
    Unsolicited,
    /// It answers a request other than the outstanding one.
    WrongReply {
        /// The outstanding request's sequence number.
        expected: u32,
        /// The reply-to it carries.
        reply_to: u32,
    },
    /// Not the response this request expects, nor a refusal.
    WrongKind {
        /// What was expected.
        expected: ResponseKind,
        /// What arrived.
        received: ResponseKind,
    },
    /// The identity came from session 0, which is never a session.
    ZeroSession,
    /// It names a session other than the open one.
    WrongSession,
    /// Its sequence number is not above the module's last one: a duplicate
    /// or replay.
    Replayed {
        /// Its sequence number.
        seq: u32,
        /// The module's last one in this session.
        last: u32,
    },
    /// No session is open: call [`HostLink::open`] first.
    NotOpen,
    /// A request is already outstanding: one at a time.
    Busy,
    /// [`Request::Identify`] goes through [`HostLink::open`].
    UseOpen,
    /// The host's sequence numbers are used up. Open a new session.
    SeqExhausted,
    /// Encoding failed.
    Encode(EncodeError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Phase {
    Closed,
    Opening,
    Open(NonZeroU64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Outstanding {
    seq: u32,
    kind: RequestKind,
}

/// The host's end of the link to one module: the session, one request in
/// flight, and the module's last sequence number.
///
/// What it returns is [`FromModule`]: a session being open says nothing
/// about who is on the other end until the identity verifies.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HostLink {
    phase: Phase,
    next_seq: u32,
    outstanding: Option<Outstanding>,
    last_module_seq: u32,
}

impl Default for HostLink {
    fn default() -> Self {
        HostLink::new()
    }
}

impl HostLink {
    /// A link with no session.
    #[must_use]
    pub const fn new() -> Self {
        HostLink {
            phase: Phase::Closed,
            next_seq: 1,
            outstanding: None,
            last_module_seq: 0,
        }
    }

    /// The open session, if any.
    #[must_use]
    pub const fn session(&self) -> Option<NonZeroU64> {
        match self.phase {
            Phase::Open(session) => Some(session),
            Phase::Closed | Phase::Opening => None,
        }
    }

    /// True while a request is outstanding.
    #[must_use]
    pub const fn busy(&self) -> bool {
        self.outstanding.is_some()
    }

    fn take_seq(&mut self) -> Result<u32, HostError> {
        let seq = self.next_seq;
        self.next_seq = seq.checked_add(1).ok_or(HostError::SeqExhausted)?;
        Ok(seq)
    }

    /// Encodes [`Request::Identify`] with `challenge` in session 0, which
    /// opens a new session. Any outstanding request is abandoned. The host
    /// draws `challenge` fresh from its random number generator and keeps it
    /// to verify the identity.
    ///
    /// # Errors
    ///
    /// [`HostError::Encode`] or [`HostError::SeqExhausted`].
    pub fn open(&mut self, challenge: Challenge, out: &mut [u8]) -> Result<usize, HostError> {
        let seq = self.next_seq;
        let len = encode_request(
            &RequestFrame {
                header: RequestHeader { session: 0, seq },
                request: Request::Identify { challenge },
            },
            out,
        )
        .map_err(HostError::Encode)?;
        self.take_seq()?;
        self.phase = Phase::Opening;
        self.outstanding = Some(Outstanding {
            seq,
            kind: RequestKind::Identify,
        });
        self.last_module_seq = 0;
        Ok(len)
    }

    /// Encodes `request` in the open session with the next sequence number.
    ///
    /// # Errors
    ///
    /// [`HostError::NotOpen`], [`HostError::Busy`], [`HostError::UseOpen`],
    /// [`HostError::SeqExhausted`] or [`HostError::Encode`].
    pub fn send(&mut self, request: &Request<'_>, out: &mut [u8]) -> Result<usize, HostError> {
        let Phase::Open(session) = self.phase else {
            return Err(HostError::NotOpen);
        };
        if self.outstanding.is_some() {
            return Err(HostError::Busy);
        }
        if matches!(request, Request::Identify { .. }) {
            return Err(HostError::UseOpen);
        }
        let seq = self.next_seq;
        if seq == u32::MAX {
            return Err(HostError::SeqExhausted);
        }
        let len = encode_request(
            &RequestFrame {
                header: RequestHeader {
                    session: session.get(),
                    seq,
                },
                request: *request,
            },
            out,
        )
        .map_err(HostError::Encode)?;
        self.take_seq()?;
        self.outstanding = Some(Outstanding {
            seq,
            kind: request.kind(),
        });
        Ok(len)
    }

    /// Gives up on the outstanding request, after the host's timeout. A reply
    /// to it that arrives later is [`HostError::Unsolicited`].
    pub fn abandon(&mut self) {
        self.outstanding = None;
        if matches!(self.phase, Phase::Opening) {
            self.phase = Phase::Closed;
        }
    }

    /// Judges one frame from the module. It must decode, answer the
    /// outstanding request with the response it expects (or a refusal),
    /// name the open session, and carry a module sequence number above the
    /// last. The identity that answers [`HostLink::open`] opens the session
    /// it names; verify it before trusting anything else.
    ///
    /// A frame that fails changes nothing: the request stays outstanding.
    ///
    /// # Errors
    ///
    /// A [`HostError`].
    pub fn receive<'a>(&mut self, bytes: &'a [u8]) -> Result<FromModule<Response<'a>>, HostError> {
        let frame = decode_response(bytes).map_err(HostError::Malformed)?;
        let ResponseFrame { header, response } = frame.into_claim();
        let outstanding = self.outstanding.ok_or(HostError::Unsolicited)?;
        if header.reply_to != outstanding.seq {
            return Err(HostError::WrongReply {
                expected: outstanding.seq,
                reply_to: header.reply_to,
            });
        }
        let expected = outstanding.kind.expected_reply();
        let received = response.kind();
        if received != expected && received != ResponseKind::Refused {
            return Err(HostError::WrongKind { expected, received });
        }
        match self.phase {
            Phase::Opening => {
                if received == ResponseKind::Refused {
                    self.phase = Phase::Closed;
                } else {
                    let session = NonZeroU64::new(header.session).ok_or(HostError::ZeroSession)?;
                    self.phase = Phase::Open(session);
                    self.last_module_seq = header.seq;
                }
            }
            Phase::Open(session) => {
                if header.session != session.get() {
                    return Err(HostError::WrongSession);
                }
                if header.seq <= self.last_module_seq {
                    return Err(HostError::Replayed {
                        seq: header.seq,
                        last: self.last_module_seq,
                    });
                }
                self.last_module_seq = header.seq;
            }
            // Nothing is outstanding while closed, so this is unreachable;
            // refuse it rather than assume.
            Phase::Closed => return Err(HostError::NotOpen),
        }
        self.outstanding = None;
        Ok(FromModule::new(response))
    }
}
