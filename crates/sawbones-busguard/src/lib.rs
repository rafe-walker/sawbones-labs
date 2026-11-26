//! # sawbones-busguard: PROVISIONAL
//!
//! **This crate is provisional. The Sawbones design is still under review, and
//! every type here may change or be replaced without notice.**
//!
//! The logic of the bus guard (#169): a small microcontroller in the transmit
//! path of every control-bus pod, between the pod's UART and its RS-485 or
//! RS-232 line driver, that passes only frames that were approved.
//!
//! Why it exists: an armed power rail decides whether a transmitter has power,
//! not what it sends. On Modbus the hazard is the message. One write function
//! reaches both a harmless setting and a battery protection limit; only the
//! register address differs. The guard makes "write only what was approved" a
//! wire.
//!
//! The rules it holds, in [`BusGuard::decide`]:
//!
//! - **Only well-formed frames.** A frame must parse as one Modbus RTU request
//!   with function 0x03, 0x04, 0x06 or 0x10, the exact length that function
//!   needs and a correct CRC-16/MODBUS. Anything else, and any broadcast
//!   (unit address 0), is blocked with a [`FrameError`].
//! - **Reads need a rule.** Reads transmit too, so they are not free: a read
//!   passes only if an allow-list [`Rule`] permits it on that port.
//! - **Writes need a rule and a lease.** A write passes only if a rule permits
//!   it, a [`Lease`] is live on the same port, and the frame is byte for byte
//!   the one frame the lease's approved [`BusWriteAction`] encodes to
//!   ([`BusWriteAction::frame`], from the encoder the policy engine shares,
//!   #203) with the SHA-256 recorded at the grant. Passing spends the lease.
//!   A frame with any byte changed, another unit, register, value or
//!   function, is blocked.
//! - **No writes while another master talks.** If the firmware feeds the
//!   guard what it hears on the line ([`BusGuard::observe_line`]) and a
//!   request appears that the guard did not send, every write is blocked
//!   until the trust root sends a new allow-list (#203, bus-020).
//! - **Everything else blocks.**
//!
//! And around it:
//!
//! - **The bytes checked are the bytes sent (#203, bus-009).**
//!   [`BusGuard::decide`] copies the frame before checking it and returns a
//!   [`Decision::Pass`] that owns the copy. The firmware hands each port's
//!   transmitter to a [`GuardedLine`], whose only way to send is
//!   [`GuardedLine::transmit`] of a [`Pass`] by value: no raw slice, no
//!   second send.
//! - **Leases name one action.** A grant carries a [`BusWriteAction`] (bench
//!   port, unit address, function, register, values) and its
//!   [`ActionDigest`], the same 32 bytes the policy engine's class 3
//!   decision and the interlock's lease carry. [`LeaseGrant::new`] refuses a
//!   digest that is not the action's.
//! - **Updates come only from the trust root.** The allow-list is replaced,
//!   and a lease granted, only by [`BusGuard::apply`], which takes a
//!   [`TrustRootEvent`]. That type stands for "came from the trust root after
//!   a press of the panel button". There is no other way to change the
//!   allow-list or create a lease. With the `approvals` feature (on by
//!   default) a [`TrustRootEvent`] is built only from an
//!   `sawbones_approvals::Approved` lease or allow-list update: the trust
//!   root's Ed25519 signature, the role, a replay counter and an expiry were
//!   checked by a `sawbones_approvals::Verifier` (#172). Only the trust-root
//!   role may approve either; the owner's phone cannot.
//! - **At most one lease, used at most once.** A lease is present or absent;
//!   passing its write removes it, so it cannot be spent twice. A new grant
//!   replaces the old lease, and a new allow-list removes it.
//! - **Time.** A lease lasts [`LEASE_TTL_TICKS`] on the guard's own clock. A
//!   clock that goes backwards ends it. [`BusGuard::restart`] ends it too; the
//!   allow-list survives a restart.
//!
//! What is deliberately not here yet:
//!
//! - **Real keys (#172).** The signature check is built and tested with
//!   fixed test keys. It is switched on with real keys when hardware testing
//!   starts; where the guard's keys and its last replay counter are stored is
//!   for that phase. Within one program the event is neither `Clone` nor
//!   `Copy` and `apply` consumes it, so the same value cannot be applied
//!   twice; across messages the verifier's counter refuses a replay.
//! - **The unverified constructor.**
//!   `TrustRootEvent::unverified_from_trust_root` wraps an update with no
//!   check. It exists only with the `test-unverified` feature, for tests and
//!   simulation, and must never be in a firmware build.
//! - **The wire.** Framing by silence, inter-frame gaps and the
//!   driver-enable pin are the firmware's. The types make it send only bytes
//!   that passed ([`GuardedLine`]); the firmware must not keep another handle
//!   to the transmitter.
//! - **Responses.** The guard judges requests going out, not replies coming
//!   back. [`BusGuard::observe_line`] only looks for requests it did not
//!   send.
//!
//! The crate reads no clock, allocates nothing and is `no_std`, so the same
//! code runs under test on a laptop and in the RP2350's firmware.

#![no_std]
#![forbid(unsafe_code)]

mod frame;

pub use frame::{
    crc16_modbus, parse, Frame, FrameError, Function, MAX_FRAME_LEN, MAX_READ_QUANTITY,
    MAX_UNIT_ADDRESS, MAX_WRITE_QUANTITY,
};
pub use sawbones_modbus_encode::{
    ActionDigest, ActionError, BusWriteAction, RtuFrame, WriteFunction, MAX_ACTION_WORDS,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A monotonic tick count supplied by the caller. The tick period is the
/// caller's choice and is not fixed yet.
pub type Tick = u64;

/// How many ticks a granted lease stays live, counted on the guard's clock
/// from the tick the grant was applied. A placeholder: the real window is set
/// from how long the trust root's approval should stand.
pub const LEASE_TTL_TICKS: Tick = 100;

/// How many rules the allow-list holds.
pub const MAX_RULES: usize = 32;

/// The length of one rule in [`AllowList::encode`].
pub const RULE_ENCODED_LEN: usize = 9;

/// The length of [`AllowList::encode`]: a count, then [`MAX_RULES`] slots.
pub const ALLOW_LIST_ENCODED_LEN: usize = 1 + MAX_RULES * RULE_ENCODED_LEN;

/// Hashed before an allow-list's encoding in [`AllowList::digest`].
pub const ALLOW_LIST_DOMAIN: &[u8] = b"sawbones-busguard/allow-list/v1\0";

/// One of the guard's serial ports. One guard may sit in front of more than
/// one line driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Port(pub u8);

/// SHA-256 of a frame's exact bytes, CRC included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FrameDigest(pub [u8; 32]);

impl FrameDigest {
    /// The digest of `bytes`. A lease records it for the frame its approved
    /// action encodes to.
    #[must_use]
    pub fn of(bytes: &[u8]) -> FrameDigest {
        FrameDigest(Sha256::digest(bytes).into())
    }
}

/// One entry in the allow-list: the frames a port may carry to one unit with
/// one function, inside one register range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rule {
    /// The port the frame goes out on.
    pub port: Port,
    /// The unit (slave) address.
    pub unit: u8,
    /// The function.
    pub function: Function,
    /// The first register a frame may address.
    pub first: u16,
    /// The last register a frame may address, inclusive.
    pub last: u16,
    /// The most registers one frame may address.
    pub max_quantity: u16,
}

impl Rule {
    /// True if this rule permits `frame` on `port`: same port, unit and
    /// function, every addressed register inside `first..=last`, and no more
    /// than `max_quantity` of them.
    ///
    /// A rule with `first > last`, `max_quantity == 0` or a unit no frame can
    /// carry permits nothing.
    #[must_use]
    pub const fn permits(&self, port: Port, frame: &Frame<'_>) -> bool {
        self.port.0 == port.0
            && self.unit == frame.unit()
            && self.function as u8 == frame.function() as u8
            && frame.start() >= self.first
            && frame.end() <= self.last
            && frame.quantity() <= self.max_quantity
    }

    /// True if this rule can permit some frame. [`AllowList::try_push`]
    /// refuses a rule that cannot.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.unit >= 1
            && self.unit <= MAX_UNIT_ADDRESS
            && self.first <= self.last
            && self.max_quantity >= 1
    }
}

/// Why a rule could not be added to an [`AllowList`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum AllowListError {
    /// The list already holds [`MAX_RULES`] rules.
    Full,
    /// The rule can never permit a frame; see [`Rule::is_valid`].
    InvalidRule,
}

/// A fixed-capacity list of up to [`MAX_RULES`] rules. A frame is permitted
/// if any rule permits it.
///
/// Building one changes nothing: it is plain data until the trust root sends
/// it in [`Update::ReplaceAllowList`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AllowList {
    rules: [Option<Rule>; MAX_RULES],
}

impl AllowList {
    /// An empty list. It permits nothing.
    #[must_use]
    pub const fn new() -> AllowList {
        AllowList {
            rules: [None; MAX_RULES],
        }
    }

    /// Adds a rule.
    ///
    /// # Errors
    ///
    /// [`AllowListError::InvalidRule`] for a rule that can never permit a
    /// frame, [`AllowListError::Full`] if there is no room.
    pub fn try_push(&mut self, rule: Rule) -> Result<(), AllowListError> {
        if !rule.is_valid() {
            return Err(AllowListError::InvalidRule);
        }
        let slot = self
            .rules
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or(AllowListError::Full)?;
        *slot = Some(rule);
        Ok(())
    }

    /// The rules, in the order they were added.
    pub fn rules(&self) -> impl Iterator<Item = &Rule> {
        self.rules.iter().flatten()
    }

    /// How many rules the list holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules().count()
    }

    /// True if the list holds no rules.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules().next().is_none()
    }

    /// True if any rule permits `frame` on `port`.
    #[must_use]
    pub fn permits(&self, port: Port, frame: &Frame<'_>) -> bool {
        self.rules().any(|rule| rule.permits(port, frame))
    }

    /// The list's one encoding, [`ALLOW_LIST_ENCODED_LEN`] bytes: the number
    /// of rules (1 byte), then each rule in the order added as port, unit,
    /// function code (1 byte each), first, last and max quantity (2 bytes
    /// each, big-endian), then zeros for the unused slots.
    #[must_use]
    pub fn encode(&self) -> [u8; ALLOW_LIST_ENCODED_LEN] {
        let mut out = [0; ALLOW_LIST_ENCODED_LEN];
        let mut count: u8 = 0;
        for (slot, rule) in out[1..]
            .chunks_exact_mut(RULE_ENCODED_LEN)
            .zip(self.rules())
        {
            let [f0, f1] = rule.first.to_be_bytes();
            let [l0, l1] = rule.last.to_be_bytes();
            let [m0, m1] = rule.max_quantity.to_be_bytes();
            slot.copy_from_slice(&[
                rule.port.0,
                rule.unit,
                rule.function.code(),
                f0,
                f1,
                l0,
                l1,
                m0,
                m1,
            ]);
            count += 1;
        }
        out[0] = count;
        out
    }

    /// SHA-256 of [`ALLOW_LIST_DOMAIN`] followed by [`AllowList::encode`].
    /// The trust root signs this digest to approve the list.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(ALLOW_LIST_DOMAIN);
        hasher.update(self.encode());
        hasher.finalize().into()
    }
}

impl Default for AllowList {
    fn default() -> AllowList {
        AllowList::new()
    }
}

/// Permission to pass one write, once: the write of one approved
/// [`BusWriteAction`].
///
/// There is no count of uses: a lease is present or absent, and passing its
/// write removes it. Serialize only: a lease is reported, never loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Lease {
    /// The one write approved.
    pub action: BusWriteAction,
    /// The action's digest, as the policy engine, the interlock and the
    /// approval carry it.
    pub action_digest: ActionDigest,
    /// The digest of the one frame the action encodes to,
    /// [`BusWriteAction::frame`].
    pub digest: FrameDigest,
    /// The first tick on which the lease is no longer live.
    pub expires_at: Tick,
}

impl Lease {
    /// The port the write must go out on: the action's bench port.
    #[must_use]
    pub const fn port(&self) -> Port {
        Port(self.action.port())
    }
}

/// A lease grant's action digest is not the digest of its action (#203).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ActionDigestMismatch;

/// What the trust root asks for in a lease grant: one action, and the digest
/// the policy engine and the interlock bound the lease to.
///
/// The fields are private. [`LeaseGrant::new`] refuses a digest that is not
/// the action's, so every grant the guard can see names one action
/// consistently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct LeaseGrant {
    action: BusWriteAction,
    action_digest: ActionDigest,
}

impl LeaseGrant {
    /// A grant for `action`, bound to `action_digest`.
    ///
    /// # Errors
    ///
    /// [`ActionDigestMismatch`] if `action_digest` is not
    /// [`BusWriteAction::digest`] of `action`.
    pub fn new(
        action: BusWriteAction,
        action_digest: ActionDigest,
    ) -> Result<LeaseGrant, ActionDigestMismatch> {
        if action.digest() != action_digest {
            return Err(ActionDigestMismatch);
        }
        Ok(LeaseGrant {
            action,
            action_digest,
        })
    }

    /// A grant for `action`, with the digest computed here.
    #[must_use]
    pub fn for_action(action: BusWriteAction) -> LeaseGrant {
        LeaseGrant {
            action,
            action_digest: action.digest(),
        }
    }

    /// The action granted.
    #[must_use]
    pub const fn action(&self) -> &BusWriteAction {
        &self.action
    }

    /// The action's digest.
    #[must_use]
    pub const fn action_digest(&self) -> ActionDigest {
        self.action_digest
    }
}

/// The two changes the trust root can make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[expect(
    clippy::large_enum_variant,
    reason = "no allocation: the allow-list travels by value, and updates are rare"
)]
pub enum Update {
    /// Replace the whole allow-list. Any lease ends, because it was approved
    /// against the old list, and any record of another master ends, because
    /// the trust root has re-approved the bus.
    ReplaceAllowList(AllowList),
    /// Grant a lease for one write, replacing any lease already live.
    GrantLease(LeaseGrant),
}

/// An [`Update`] that came from the trust root after a person pressed the
/// panel button.
///
/// This is the only input that can change the allow-list or create a lease.
/// It is deliberately not `Clone`, `Copy`, `Default` or deserializable, and
/// its field is private. With the `approvals` feature it is made from a
/// checked approval, by [`TrustRootEvent::from_approved_lease`] or
/// [`TrustRootEvent::from_approved_allow_list`]. The unverified constructor
/// exists only with the `test-unverified` feature, for tests.
///
/// ```compile_fail
/// use sawbones_busguard::{AllowList, TrustRootEvent, Update};
/// // The field is private: no struct literal.
/// let _ = TrustRootEvent { update: Update::ReplaceAllowList(AllowList::new()) };
/// ```
///
/// ```compile_fail
/// use sawbones_busguard::{BusGuard, TrustRootEvent};
/// fn twice(guard: &mut BusGuard, event: TrustRootEvent) {
///     let _ = guard.apply(event, 0);
///     // Applying consumed it: the same event cannot be applied twice.
///     let _ = guard.apply(event, 0);
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct TrustRootEvent {
    update: Update,
}

/// An approved allow-list's digest is not the digest of the list given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AllowListMismatch;

impl TrustRootEvent {
    /// **TESTS AND SIMULATION ONLY.** Wraps an update with no check at all.
    ///
    /// Exists only with the `test-unverified` feature, which nothing but
    /// test builds turns on. Real code uses
    /// [`TrustRootEvent::from_approved_lease`] or
    /// [`TrustRootEvent::from_approved_allow_list`].
    #[cfg(feature = "test-unverified")]
    #[must_use]
    pub const fn unverified_from_trust_root(update: Update) -> TrustRootEvent {
        TrustRootEvent { update }
    }

    /// A lease grant from a checked approval. The verifier has already
    /// required the trust-root role, a valid signature, a fresh counter and
    /// an unexpired approval for this device.
    ///
    /// # Errors
    ///
    /// [`ActionDigestMismatch`] if the approval's action digest is not the
    /// digest of its action (#203). The approval is spent either way.
    #[cfg(feature = "approvals")]
    pub fn from_approved_lease(
        approved: sawbones_approvals::Approved<sawbones_approvals::GrantLease>,
    ) -> Result<TrustRootEvent, ActionDigestMismatch> {
        let grant = approved.into_approval();
        let grant = LeaseGrant::new(grant.action, ActionDigest(grant.action_digest.0))?;
        Ok(TrustRootEvent {
            update: Update::GrantLease(grant),
        })
    }

    /// An allow-list replacement from a checked approval of `list`'s digest.
    ///
    /// # Errors
    ///
    /// [`AllowListMismatch`] if the approval is for a list with another
    /// [`AllowList::digest`]. The approval is spent either way.
    #[cfg(feature = "approvals")]
    pub fn from_approved_allow_list(
        approved: sawbones_approvals::Approved<sawbones_approvals::BusGuardAllowList>,
        list: AllowList,
    ) -> Result<TrustRootEvent, AllowListMismatch> {
        if approved.into_approval().allow_list.0 != list.digest() {
            return Err(AllowListMismatch);
        }
        Ok(TrustRootEvent {
            update: Update::ReplaceAllowList(list),
        })
    }

    /// The update carried.
    #[must_use]
    pub const fn update(&self) -> &Update {
        &self.update
    }
}

/// Why [`BusGuard::apply`] refused an update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum UpdateRefusal {
    /// The clock went backwards on this call, so a lifetime counted from it
    /// means nothing. Any lease has ended; the trust root may grant again.
    ClockWentBackwards,
    /// The guard has seen another master on the bus (#203, bus-020), so no
    /// write may pass and a lease would mean nothing. A new allow-list from
    /// the trust root clears it.
    OtherMasterSeen,
}

/// Why a frame was blocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum BlockReason {
    /// The bytes are not a frame the guard will consider.
    Frame(FrameError),
    /// No allow-list rule permits this frame on this port.
    NotAllowListed,
    /// A write with no lease live.
    NoLease,
    /// A write on a port other than the lease's. The lease stays.
    LeaseOnOtherPort,
    /// A write whose lease expired; it has ended.
    LeaseExpired,
    /// A write whose lease ended on this call because the clock went
    /// backwards.
    ClockWentBackwards,
    /// A write whose bytes are not exactly the frame the lease's approved
    /// action encodes to: compared byte for byte with the frame re-derived
    /// from the action, and by digest. The lease stays, so the approved
    /// frame can still go.
    DigestMismatch,
    /// A write while the guard has seen another master on the bus
    /// ([`BusGuard::observe_line`]). Writes stay blocked until the trust
    /// root sends a new allow-list.
    OtherMasterSeen,
}

/// The guard's answer for one frame, as plain data: what
/// [`Decision::verdict`] reports, for logs and tests. It carries no bytes and
/// cannot be transmitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Verdict {
    /// The frame passed.
    Pass,
    /// The frame was blocked.
    Block(BlockReason),
}

impl Verdict {
    /// True for [`Verdict::Pass`].
    #[must_use]
    pub const fn is_pass(self) -> bool {
        matches!(self, Verdict::Pass)
    }
}

/// Permission to transmit one frame, once, holding the guard's own copy of
/// the exact bytes it checked (#203, bus-009).
///
/// [`BusGuard::decide`] copies the frame before it checks anything and
/// judges only its copy, so nothing the caller does to its buffer afterwards
/// (a DMA engine, another core, an interrupt) can change what is sent. The
/// only way to put bytes on a [`GuardedLine`] is
/// [`GuardedLine::transmit`], which takes a `Pass` by value.
///
/// Not `Clone`, `Copy`, `Default` or deserializable, with private fields, so
/// one decision is one transmission at most:
///
/// ```compile_fail
/// use sawbones_busguard::{Pass, Port};
/// // No struct literal: the fields are private.
/// let _ = Pass { port: Port(0), len: 8, bytes: [0; 256] };
/// ```
///
/// ```compile_fail
/// use sawbones_busguard::{FrameSink, GuardedLine, Pass};
/// fn twice<S: FrameSink>(line: &mut GuardedLine<S>, pass: Pass) {
///     let _ = line.transmit(pass);
///     // Transmitting consumed it: the same pass cannot be sent twice.
///     let _ = line.transmit(pass);
/// }
/// ```
///
/// ```compile_fail
/// use sawbones_busguard::{FrameSink, GuardedLine};
/// fn raw<S: FrameSink>(line: &mut GuardedLine<S>) {
///     // A raw slice is not a pass: it cannot be transmitted.
///     let _ = line.transmit(&[0x01, 0x06, 0x00, 0x10, 0x00, 0x03, 0x88, 0x0E][..]);
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
#[must_use = "a pass that is dropped transmits nothing"]
pub struct Pass {
    port: Port,
    len: u16,
    bytes: [u8; MAX_FRAME_LEN],
}

impl Pass {
    /// The port the frame may go out on.
    #[must_use]
    pub const fn port(&self) -> Port {
        self.port
    }

    /// The exact bytes that were checked.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }
}

/// The guard's answer for one frame.
///
/// Not `Clone`, `Copy` or deserializable: a [`Decision::Pass`] carries the
/// one [`Pass`] that lets the frame be transmitted. Compare it with a
/// [`Verdict`], or take [`Decision::verdict`], to inspect it.
#[derive(Debug, PartialEq, Eq)]
#[must_use]
#[expect(
    clippy::large_enum_variant,
    reason = "no allocation: the pass owns its copy of the frame by value"
)]
pub enum Decision {
    /// Transmit the frame, through [`GuardedLine::transmit`].
    Pass(Pass),
    /// Do not transmit any of it.
    Block(BlockReason),
}

impl Decision {
    /// True for [`Decision::Pass`].
    #[must_use]
    pub const fn is_pass(&self) -> bool {
        matches!(self, Decision::Pass(_))
    }

    /// The decision as plain data.
    #[must_use]
    pub const fn verdict(&self) -> Verdict {
        match self {
            Decision::Pass(_) => Verdict::Pass,
            Decision::Block(reason) => Verdict::Block(*reason),
        }
    }

    /// The pass, if the frame passed.
    #[must_use]
    pub fn into_pass(self) -> Option<Pass> {
        match self {
            Decision::Pass(pass) => Some(pass),
            Decision::Block(_) => None,
        }
    }
}

impl PartialEq<Verdict> for Decision {
    fn eq(&self, other: &Verdict) -> bool {
        self.verdict() == *other
    }
}

/// The raw transmitter behind one port: the firmware's UART and line driver.
/// It sends whatever it is given, so the firmware hands it to a
/// [`GuardedLine`] and keeps no other handle to it.
pub trait FrameSink {
    /// What the transmitter can fail with.
    type Error;

    /// Sends `bytes` as one frame.
    ///
    /// # Errors
    ///
    /// Whatever the transmitter reports.
    fn send_frame(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
}

/// Why [`GuardedLine::transmit`] sent nothing, or what the transmitter
/// reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransmitError<E> {
    /// The pass is for another port. Nothing was sent and the pass is spent.
    OtherPort,
    /// The transmitter failed.
    Sink(E),
}

/// One port's transmitter, owned, so that the only way to send on it is
/// with a [`Pass`] for that port (#203, bus-009).
///
/// There is no way to get the transmitter back out or to send raw bytes.
#[derive(Debug)]
pub struct GuardedLine<S> {
    port: Port,
    sink: S,
}

impl<S: FrameSink> GuardedLine<S> {
    /// Takes ownership of the transmitter for `port`.
    pub const fn new(port: Port, sink: S) -> GuardedLine<S> {
        GuardedLine { port, sink }
    }

    /// The port this line sends on.
    #[must_use]
    pub const fn port(&self) -> Port {
        self.port
    }

    /// Sends exactly the bytes `pass` holds, once.
    ///
    /// # Errors
    ///
    /// [`TransmitError::OtherPort`] for a pass for another port, which sends
    /// nothing; [`TransmitError::Sink`] if the transmitter fails.
    pub fn transmit(&mut self, pass: Pass) -> Result<(), TransmitError<S::Error>> {
        if pass.port != self.port {
            return Err(TransmitError::OtherPort);
        }
        self.sink
            .send_frame(pass.as_bytes())
            .map_err(TransmitError::Sink)
    }
}

/// How many ticks after passing a frame the guard still takes a request it
/// hears with the same bytes on the same port as its own echo, or a slave's
/// echo reply to a write single register. A placeholder, like
/// [`LEASE_TTL_TICKS`]: the real window is a few frame times at the line's
/// baud rate.
pub const ECHO_WINDOW_TICKS: Tick = 10;

/// How many recently passed frames the guard remembers as expected echoes.
pub const ECHO_SLOTS: usize = 4;

/// What the guard made of one frame heard on a line, from
/// [`BusGuard::observe_line`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Heard {
    /// Not a well-formed request: a response, noise or a function the guard
    /// does not parse. Nothing changes.
    NotARequest,
    /// A request this guard passed on this port within
    /// [`ECHO_WINDOW_TICKS`]: its own transmission heard back.
    OwnEcho,
    /// A request this guard did not send: another master is active. Writes
    /// are blocked and the lease has ended.
    ForeignRequest,
}

/// One frame the guard passed, remembered so that hearing it back is not
/// taken for another master.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Echo {
    port: Port,
    digest: FrameDigest,
    at: Tick,
}

/// How a lease ended on one call, if it did.
#[derive(Clone, Copy)]
enum LeaseEnd {
    Expired,
    ClockWentBackwards,
}

/// The bus guard's whole memory: the allow-list, at most one lease, the
/// last tick seen, the frames it recently passed and how many requests from
/// another master it has heard.
///
/// Not `Clone`: a copy would carry a second use of the same lease.
#[derive(Debug, PartialEq, Eq)]
pub struct BusGuard {
    allow_list: AllowList,
    lease: Option<Lease>,
    last_now: Option<Tick>,
    echoes: [Option<Echo>; ECHO_SLOTS],
    next_echo: usize,
    foreign_requests: u32,
}

impl BusGuard {
    /// A guard with an empty allow-list and no lease. It blocks everything.
    #[must_use]
    pub const fn new() -> BusGuard {
        BusGuard {
            allow_list: AllowList::new(),
            lease: None,
            last_now: None,
            echoes: [None; ECHO_SLOTS],
            next_echo: 0,
            foreign_requests: 0,
        }
    }

    /// The allow-list in force.
    #[must_use]
    pub const fn allow_list(&self) -> &AllowList {
        &self.allow_list
    }

    /// The lease, if one is held. It may have expired without a call since to
    /// notice; [`BusGuard::decide`] checks.
    #[must_use]
    pub const fn lease(&self) -> Option<Lease> {
        self.lease
    }

    /// How many requests from another master [`BusGuard::observe_line`] has
    /// heard since the trust root last sent an allow-list, saturating. While
    /// it is above zero every write is blocked.
    #[must_use]
    pub const fn foreign_requests_seen(&self) -> u32 {
        self.foreign_requests
    }

    /// The guard restarted: boot, reset, brownout or crash. The lease, the
    /// last tick and the remembered echoes are forgotten; the allow-list and
    /// the count of foreign requests are kept, so a restart cannot clear a
    /// sighting of another master. The firmware must persist both.
    pub fn restart(&mut self) {
        self.lease = None;
        self.last_now = None;
        self.echoes = [None; ECHO_SLOTS];
        self.next_echo = 0;
    }

    /// Applies an update from the trust root at tick `now`.
    ///
    /// Replacing the allow-list always succeeds, ends any lease and clears
    /// the count of foreign requests. A grant replaces any lease with one for
    /// the grant's action, live until `now + LEASE_TTL_TICKS`.
    ///
    /// # Errors
    ///
    /// For a grant, [`UpdateRefusal::ClockWentBackwards`] on a call where the
    /// clock went backwards, and [`UpdateRefusal::OtherMasterSeen`] while
    /// foreign requests have been heard. Nothing is granted.
    pub fn apply(&mut self, event: TrustRootEvent, now: Tick) -> Result<(), UpdateRefusal> {
        let (backwards, _) = self.tick(now);
        match event.update {
            Update::ReplaceAllowList(list) => {
                self.allow_list = list;
                self.lease = None;
                self.foreign_requests = 0;
                Ok(())
            }
            Update::GrantLease(_) if backwards => Err(UpdateRefusal::ClockWentBackwards),
            Update::GrantLease(_) if self.foreign_requests > 0 => {
                Err(UpdateRefusal::OtherMasterSeen)
            }
            Update::GrantLease(grant) => {
                self.lease = Some(Lease {
                    action: grant.action,
                    action_digest: grant.action_digest,
                    digest: FrameDigest::of(grant.action.frame().as_bytes()),
                    expires_at: now.saturating_add(LEASE_TTL_TICKS),
                });
                Ok(())
            }
        }
    }

    /// Decides whether `frame` may go out on `port` at tick `now`.
    ///
    /// The guard first copies `frame` into the [`Pass`] it may return and
    /// judges only that copy, so the bytes checked are the bytes sent.
    /// Passing a write spends the lease. Nothing else here creates or extends
    /// permission.
    pub fn decide(&mut self, port: Port, frame: &[u8], now: Tick) -> Decision {
        let (_, ended) = self.tick(now);
        if frame.len() > MAX_FRAME_LEN {
            return Decision::Block(BlockReason::Frame(FrameError::TooLong));
        }
        let mut bytes = [0; MAX_FRAME_LEN];
        bytes[..frame.len()].copy_from_slice(frame);
        // At most MAX_FRAME_LEN (256), checked above.
        let len = frame.len() as u16;
        let copy = &bytes[..frame.len()];
        if let Err(reason) = self.judge(port, copy, ended) {
            return Decision::Block(reason);
        }
        self.remember_echo(port, FrameDigest::of(copy), now);
        Decision::Pass(Pass { port, len, bytes })
    }

    /// The rules, on the guard's own copy of the frame. Spends the lease if a
    /// write passes.
    fn judge(
        &mut self,
        port: Port,
        frame: &[u8],
        ended: Option<LeaseEnd>,
    ) -> Result<(), BlockReason> {
        let parsed = parse(frame).map_err(BlockReason::Frame)?;
        if !self.allow_list.permits(port, &parsed) {
            return Err(BlockReason::NotAllowListed);
        }
        if !parsed.function().is_write() {
            return Ok(());
        }
        if self.foreign_requests > 0 {
            return Err(BlockReason::OtherMasterSeen);
        }
        let Some(lease) = self.lease else {
            return Err(match ended {
                Some(LeaseEnd::Expired) => BlockReason::LeaseExpired,
                Some(LeaseEnd::ClockWentBackwards) => BlockReason::ClockWentBackwards,
                None => BlockReason::NoLease,
            });
        };
        if lease.port() != port {
            return Err(BlockReason::LeaseOnOtherPort);
        }
        // The one frame the approved action encodes to, re-derived here, and
        // the digest recorded when the lease was granted. Both must match.
        if frame != lease.action.frame().as_bytes() || FrameDigest::of(frame) != lease.digest {
            return Err(BlockReason::DigestMismatch);
        }
        self.lease = None;
        Ok(())
    }

    /// Takes in one frame heard on `port`'s line at tick `now` (#203,
    /// bus-020). Optional: a guard whose firmware never calls this behaves
    /// exactly as before.
    ///
    /// A well-formed request (as [`parse`] accepts it, or a well-formed
    /// broadcast) that this guard did not pass on that port within
    /// [`ECHO_WINDOW_TICKS`] means another master is active. The guard counts
    /// it, ends any lease, and blocks every write with
    /// [`BlockReason::OtherMasterSeen`] until the trust root sends a new
    /// allow-list. Reads still follow their rules.
    ///
    /// What it cannot see: a master that only uses function codes the guard
    /// does not parse, or one that sends exactly a frame this guard just
    /// passed, inside the echo window. Responses are never counted: no read
    /// response has the length of a read request, and a slave's echo of a
    /// write single register is the guard's own frame.
    pub fn observe_line(&mut self, port: Port, frame: &[u8], now: Tick) -> Heard {
        self.tick(now);
        match parse(frame) {
            Ok(_) | Err(FrameError::Broadcast) => {}
            Err(_) => return Heard::NotARequest,
        }
        let digest = FrameDigest::of(frame);
        let echo = self.echoes.iter().flatten().any(|echo| {
            echo.port == port
                && echo.digest == digest
                && echo.at <= now
                && now - echo.at <= ECHO_WINDOW_TICKS
        });
        if echo {
            return Heard::OwnEcho;
        }
        self.foreign_requests = self.foreign_requests.saturating_add(1);
        self.lease = None;
        Heard::ForeignRequest
    }

    /// Remembers a frame just passed, overwriting the oldest.
    fn remember_echo(&mut self, port: Port, digest: FrameDigest, at: Tick) {
        self.echoes[self.next_echo] = Some(Echo { port, digest, at });
        self.next_echo = (self.next_echo + 1) % ECHO_SLOTS;
    }

    /// Takes in the tick of this call. Ends the lease if the clock went
    /// backwards or the lease has expired, and says which.
    fn tick(&mut self, now: Tick) -> (bool, Option<LeaseEnd>) {
        let backwards = self.last_now.is_some_and(|last| now < last);
        self.last_now = Some(now);
        let ended = match self.lease {
            Some(_) if backwards => Some(LeaseEnd::ClockWentBackwards),
            Some(lease) if now >= lease.expires_at => Some(LeaseEnd::Expired),
            _ => None,
        };
        if ended.is_some() {
            self.lease = None;
        }
        (backwards, ended)
    }
}

impl Default for BusGuard {
    fn default() -> BusGuard {
        BusGuard::new()
    }
}
