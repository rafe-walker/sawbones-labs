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
//!   it, the SHA-256 of its exact bytes equals the active [`Lease`]'s digest,
//!   the lease is on the same port and it has not expired. Passing spends the
//!   lease. A frame with any byte changed has a different digest and is
//!   blocked.
//! - **Everything else blocks.**
//!
//! And around it:
//!
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
//! - **The wire.** Framing by silence, inter-frame gaps, the driver-enable
//!   pin and buffer handling are the firmware's. The firmware must copy a
//!   frame into its own buffer, decide on that buffer, and transmit exactly
//!   those bytes as one frame, and nothing it did not decide on.
//! - **Responses.** The guard judges requests going out, not replies coming
//!   back.
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
    /// The digest of `bytes`. The trust root computes the same thing over the
    /// frame it approved.
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

/// Permission to pass one write, once.
///
/// There is no count of uses: a lease is present or absent, and passing its
/// write removes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Lease {
    /// The port the write must go out on.
    pub port: Port,
    /// The digest of the exact frame approved.
    pub digest: FrameDigest,
    /// The first tick on which the lease is no longer live.
    pub expires_at: Tick,
}

/// What the trust root asks for in a lease grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LeaseGrant {
    /// The port the write must go out on.
    pub port: Port,
    /// The digest of the exact frame approved, from [`FrameDigest::of`].
    pub digest: FrameDigest,
}

/// The two changes the trust root can make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[expect(
    clippy::large_enum_variant,
    reason = "no allocation: the allow-list travels by value, and updates are rare"
)]
pub enum Update {
    /// Replace the whole allow-list. Any lease ends, because it was approved
    /// against the old list.
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
    #[cfg(feature = "approvals")]
    pub fn from_approved_lease(
        approved: sawbones_approvals::Approved<sawbones_approvals::GrantLease>,
    ) -> TrustRootEvent {
        let grant = approved.into_approval();
        TrustRootEvent {
            update: Update::GrantLease(LeaseGrant {
                port: Port(grant.port),
                digest: FrameDigest(grant.action.0),
            }),
        }
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
    /// A write whose bytes are not the ones the lease approved. The lease
    /// stays, so the approved frame can still go.
    DigestMismatch,
}

/// The guard's answer for one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Decision {
    /// Transmit the frame, exactly as given.
    Pass,
    /// Do not transmit any of it.
    Block(BlockReason),
}

impl Decision {
    /// True for [`Decision::Pass`].
    #[must_use]
    pub const fn is_pass(self) -> bool {
        matches!(self, Decision::Pass)
    }
}

/// How a lease ended on one call, if it did.
#[derive(Clone, Copy)]
enum LeaseEnd {
    Expired,
    ClockWentBackwards,
}

/// The bus guard's whole memory: the allow-list, at most one lease, and the
/// last tick seen.
///
/// Not `Clone`: a copy would carry a second use of the same lease.
#[derive(Debug, PartialEq, Eq)]
pub struct BusGuard {
    allow_list: AllowList,
    lease: Option<Lease>,
    last_now: Option<Tick>,
}

impl BusGuard {
    /// A guard with an empty allow-list and no lease. It blocks everything.
    #[must_use]
    pub const fn new() -> BusGuard {
        BusGuard {
            allow_list: AllowList::new(),
            lease: None,
            last_now: None,
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

    /// The guard restarted: boot, reset, brownout or crash. The lease and the
    /// last tick are forgotten; the allow-list is kept.
    pub fn restart(&mut self) {
        self.lease = None;
        self.last_now = None;
    }

    /// Applies an update from the trust root at tick `now`.
    ///
    /// Replacing the allow-list always succeeds and ends any lease. A grant
    /// replaces any lease with one for the given port and digest, live until
    /// `now + LEASE_TTL_TICKS`.
    ///
    /// # Errors
    ///
    /// [`UpdateRefusal::ClockWentBackwards`] for a grant on a call where the
    /// clock went backwards. Nothing is granted.
    pub fn apply(&mut self, event: TrustRootEvent, now: Tick) -> Result<(), UpdateRefusal> {
        let (backwards, _) = self.observe(now);
        match event.update {
            Update::ReplaceAllowList(list) => {
                self.allow_list = list;
                self.lease = None;
                Ok(())
            }
            Update::GrantLease(_) if backwards => Err(UpdateRefusal::ClockWentBackwards),
            Update::GrantLease(grant) => {
                self.lease = Some(Lease {
                    port: grant.port,
                    digest: grant.digest,
                    expires_at: now.saturating_add(LEASE_TTL_TICKS),
                });
                Ok(())
            }
        }
    }

    /// Decides whether `frame` may go out on `port` at tick `now`.
    ///
    /// Passing a write spends the lease. Nothing else here creates or extends
    /// permission.
    #[must_use]
    pub fn decide(&mut self, port: Port, frame: &[u8], now: Tick) -> Decision {
        let (_, ended) = self.observe(now);
        let parsed = match parse(frame) {
            Ok(parsed) => parsed,
            Err(error) => return Decision::Block(BlockReason::Frame(error)),
        };
        if !self.allow_list.permits(port, &parsed) {
            return Decision::Block(BlockReason::NotAllowListed);
        }
        if !parsed.function().is_write() {
            return Decision::Pass;
        }
        let Some(lease) = self.lease else {
            return Decision::Block(match ended {
                Some(LeaseEnd::Expired) => BlockReason::LeaseExpired,
                Some(LeaseEnd::ClockWentBackwards) => BlockReason::ClockWentBackwards,
                None => BlockReason::NoLease,
            });
        };
        if lease.port != port {
            return Decision::Block(BlockReason::LeaseOnOtherPort);
        }
        if FrameDigest::of(frame) != lease.digest {
            return Decision::Block(BlockReason::DigestMismatch);
        }
        self.lease = None;
        Decision::Pass
    }

    /// Takes in the tick of this call. Ends the lease if the clock went
    /// backwards or the lease has expired, and says which.
    fn observe(&mut self, now: Tick) -> (bool, Option<LeaseEnd>) {
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
