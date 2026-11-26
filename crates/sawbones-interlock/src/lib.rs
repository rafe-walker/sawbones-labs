//! # sawbones-interlock: PROVISIONAL
//!
//! **This crate is provisional. The Sawbones design is still under review, and
//! every type here may change or be replaced without notice.**
//!
//! The interlock's logic as a pure state machine with no hardware in it. One
//! function, [`step`], takes the previous [`State`], the [`Inputs`] sampled on
//! this tick and at most one [`Event`], and returns the next state and the
//! [`Outputs`] to drive. It reads no clock, allocates nothing and is `no_std`,
//! so the same code can run on a laptop under test and later in the
//! controller's firmware.
//!
//! The rules it holds:
//!
//! - **Off after restart, and a restart forgets nothing.** Every output is off
//!   at power-on and after [`Event::Restart`]. A restart reads the
//!   [`FaultRecord`] the firmware keeps in non-volatile memory. Only a record
//!   of [`FaultRecord::Clear`] lets the interlock come up unfaulted, and then
//!   it still waits for the arm switch to be seen off and then on. A missing
//!   or unreadable record, or one that says a rail may have been on
//!   ([`FaultRecord::Armed`]), comes up with
//!   [`FaultReason::FaultStateUnknown`] latched; a recorded fault comes up
//!   latched again. [`State::POWER_ON`] itself has the unknown fault latched.
//! - **Emergency stop and heartbeat.** A pressed emergency stop or a lost
//!   heartbeat turns every output off on that tick and latches a fault.
//! - **Heartbeat as a challenge.** The interlock issues a fresh
//!   [`HeartbeatChallenge`] every [`HEARTBEAT_PERIOD_TICKS`] (shown in
//!   [`Outputs::challenge`]). The supervisor must answer each one once, with
//!   [`HeartbeatChallenge::reply`], on a later tick and before the next is
//!   due. A reply that is wrong, stale, early or repeated, or no reply in
//!   time, is a lost heartbeat. Nothing arms until a correct reply has been
//!   seen since the last restart or loss.
//! - **Output feedback.** Each rail's sensed state is compared with what the
//!   previous tick commanded. A rail sensed live while commanded off is stuck
//!   on: everything drops and [`FaultReason::OutputMismatch`] latches on that
//!   tick. A rail sensed dead while commanded on latches the same fault once
//!   it has been commanded on for [`OUTPUT_SETTLE_TICKS`].
//! - **Latched fault, cleared only at the panel.** A fault clears only on
//!   [`Event::ClearFault`] carrying a [`PanelToken`], with the panel button
//!   sensed pressed, the emergency stop released, the heartbeat answered, no
//!   rail sensed live and the arm switch off. A clear from the supervisor is
//!   refused in every state with [`Refusal::ClearNotFromPanel`].
//! - **Rail A (hazard class 2)** is on while armed: no fault, the arm switch
//!   on, and a valid arm sequence behind it.
//! - **Rail B (hazard class 3)** also needs the key switch on and a live
//!   lease, and is on for the lease's port only ([`Outputs::rail_b`]). A
//!   lease is bound to one [`LeaseTarget`]: one port and the digest of one
//!   action, as the policy engine's `RailBLease` decision names them. It is
//!   granted only by a fresh press of the panel button while a request is
//!   pending. It ends after [`LEASE_TTL_TICKS`], on [`Event::LeaseUsed`] for
//!   the same target, or when the key or the arm switch goes off. A use that
//!   names another target is refused and ends the lease too.
//! - **Firmware jumper.** With the jumper fitted and nothing armed the
//!   interlock enters [`Mode::FirmwareUpdate`], the only mode in which
//!   [`Outputs::firmware_update_allowed`] is true. Every rail is off there and
//!   nothing can arm. Fitting the jumper while armed latches
//!   [`FaultReason::FirmwareJumperWhileArmed`]. Removing it needs a fresh arm
//!   sequence.
//! - **Hazard class 4** has no rail. A request for it is always refused.
//!
//! No event can turn a rail on by itself. Rail A comes on with the arm switch
//! and rail B with the panel button, both of which are a person's hand.
//!
//! The rule set is identified by [`rules_id`], which the trust root reports to
//! the supervisor so it refuses to plan against rules it was not built for.
//!
//! ## The firmware's half of the contract
//!
//! This crate is pure, so some rules can only be stated for the firmware that
//! calls it.
//!
//! **Persisting the fault record.** After each step the firmware keeps
//! [`State::fault_record`] in non-volatile memory (retained RAM,
//! ferroelectric RAM or flash) with a check value, so a torn or corrupt write
//! reads back as no record (`None`). The order matters:
//!
//! - Before driving any output on that was off, write the new record and wait
//!   for it to be durable. Arming writes [`FaultRecord::Armed`] first, so a
//!   reset while armed, or after a fault latched but before its record was
//!   written, comes back with [`FaultReason::FaultStateUnknown`].
//! - When outputs go off, drive them off first and write the record after.
//!   Never delay a cut for a write.
//!
//! At boot the firmware starts from [`State::POWER_ON`] (or the state it kept
//! in retained RAM) and makes its first step an [`Event::Restart`] carrying
//! the record it read and a fresh nonce from the hardware random number
//! generator. A restart is never cleaner than the worse of the state passed in
//! and the record: a state with a real fault latched, or an armed one, wins
//! over a record that says clear. [`State::POWER_ON`] says nothing, so the
//! record decides.
//!
//! Residual gap: a fault that latches while nothing is armed, followed by a
//! power loss inside the record's write time, comes back as unfaulted. Nothing
//! was on, and the arm switch must still go off then on.
//!
//! **Panel provenance.** Only the trust root's own panel driver, reading the
//! button pin itself, may make a [`PanelToken`]. A clear request that arrives
//! from the supervisor over the link is passed in as
//! [`ClearSource::Supervisor`] so it is refused and logged. The token cannot
//! be decoded from serialised bytes. Signed trust-root events (#172) will back
//! it with a key later.
//!
//! **The watchdog's hardware half.** The second cut, the relay in series with
//! the rails, is held closed by a charge pump: a capacitor stage that keeps
//! the relay transistor on only while a GPIO keeps toggling, so a pin stuck
//! high or low drops the relay. The firmware must toggle that pin by plain
//! software writes from the main loop, once per pass after [`step`] has
//! returned a rail on, and at no other time. It must never be driven by a
//! timer interrupt, PWM, PIO, DMA or any peripheral that keeps running when
//! the main loop has stopped, because then a hung firmware keeps the relay
//! closed. The pump's values, its drop-out time after toggling stops, a
//! toggle rate far above nominal, and each single component fault, are
//! simulated in ngspice under #123.
//!
//! What is deliberately not here yet: the trusted display's content beyond
//! [`PanelState`], the preflight token that binds a lease to fact versions
//! and the chain head, signatures on the heartbeat and the panel token, and
//! any I/O. The caller samples the pins, relays the supervisor's replies,
//! keeps the record and supplies the tick.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A monotonic tick count supplied by the caller. The tick period is the
/// caller's choice and is not fixed yet.
pub type Tick = u64;

/// How many ticks a granted lease stays live. A placeholder: the real window
/// is set from what the load can do in that time.
pub const LEASE_TTL_TICKS: Tick = 100;

/// How many ticks a lease request waits for the panel button before it is
/// dropped. A placeholder, like [`LEASE_TTL_TICKS`].
pub const REQUEST_TTL_TICKS: Tick = 300;

/// How many ticks a rail may be sensed dead after it is commanded on, while
/// its contactor closes. After that, every sample must show it live. A
/// placeholder: the real figure comes from the contactor's datasheet and the
/// tick period.
///
/// There is no matching window for a stuck-on rail. The sample on a tick is
/// compared with what the previous tick commanded, so a rail has one tick to
/// drop out; a restart counts as everything commanded off long ago.
pub const OUTPUT_SETTLE_TICKS: Tick = 5;

/// How many ports rail B is gated for. Ports are numbered from 0. A
/// placeholder until the bench's pod count is fixed.
pub const RAIL_B_PORTS: usize = 4;

/// How many ticks apart heartbeat challenges are issued, and how long the
/// supervisor has to answer each one. A reply counts only on a tick after the
/// challenge was issued and fewer than this many ticks after it.
///
/// The stopping-time budget follows from it: a supervisor that stops just
/// after a good reply is declared lost, and every rail dropped, within
/// `2 * HEARTBEAT_PERIOD_TICKS` ticks of that reply. A sample that comes two
/// periods or more after a challenge was issued is a loss in itself, so a
/// stalled step loop cannot stretch the budget. A placeholder: the real
/// figure is set from the stopping time the loads allow and the tick period.
pub const HEARTBEAT_PERIOD_TICKS: Tick = 10;

// Every port number must fit in a `Port`.
const _: () = assert!(RAIL_B_PORTS >= 1 && RAIL_B_PORTS <= 256);
// A reply must have at least one tick to arrive in.
const _: () = assert!(HEARTBEAT_PERIOD_TICKS >= 2);

/// The version of the rule set. Bump it with any change to what [`step`]
/// does, however small. The test `rules_version_pins_the_transition_table`
/// fails when the transitions change and this is not bumped.
pub const RULES_VERSION: u32 = 3;

/// FNV-1a over the little-endian bytes of each word.
const fn fnv1a(words: &[u64]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut w = 0;
    while w < words.len() {
        let bytes = words[w].to_le_bytes();
        let mut b = 0;
        while b < bytes.len() {
            hash ^= bytes[b] as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            b += 1;
        }
        w += 1;
    }
    hash
}

/// The id of the rule set this build enforces: a stable 64-bit hash of
/// [`RULES_VERSION`] and every placeholder constant ([`LEASE_TTL_TICKS`],
/// [`REQUEST_TTL_TICKS`], [`OUTPUT_SETTLE_TICKS`], [`RAIL_B_PORTS`],
/// [`HEARTBEAT_PERIOD_TICKS`]).
///
/// This is what the trust root reports. The supervisor compares it with the
/// id it was built and reviewed against and refuses to plan if they differ,
/// so a plan can never rest on rules the trust root does not enforce.
///
/// The hash is FNV-1a over the little-endian bytes of each value as a `u64`,
/// in the order above. It is an identifier, not a security measure.
#[must_use]
pub const fn rules_id() -> u64 {
    fnv1a(&[
        RULES_VERSION as u64,
        LEASE_TTL_TICKS,
        REQUEST_TTL_TICKS,
        OUTPUT_SETTLE_TICKS,
        RAIL_B_PORTS as u64,
        HEARTBEAT_PERIOD_TICKS,
    ])
}

/// The five hazard classes. The class decides which rail, if any, an action
/// needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum HazardClass {
    /// Sensing only. No rail and no permission needed.
    Class0,
    /// Safe actions on logic power. No rail and no permission needed.
    Class1,
    /// Low-energy actions powered from armed rail A.
    Class2,
    /// Higher-energy actions powered from rail B, one lease per action.
    Class3,
    /// Not offered. No rail exists for it.
    Class4,
}

/// A port rail B is gated for, numbered from 0. Only ports below
/// [`RAIL_B_PORTS`] exist; a request for any other is refused with
/// [`Refusal::UnknownPort`]. The supervisor maps the policy engine's named
/// port to this number from the bench's wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Port(pub u8);

impl Port {
    /// True if rail B is gated for this port.
    #[must_use]
    pub const fn exists(self) -> bool {
        (self.0 as usize) < RAIL_B_PORTS
    }
}

/// The digest of the exact action a class 3 lease approves: the 32 bytes of
/// the policy engine's `ActionDigest`. The interlock only compares it; it
/// never computes one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ActionDigest(pub [u8; 32]);

/// What one lease is for: one action on one port. A lease for one target
/// never enables rail B on another port or pays for another action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LeaseTarget {
    /// The port rail B is enabled for.
    pub port: Port,
    /// The digest of the one action the lease approves.
    pub action_digest: ActionDigest,
}

/// A heartbeat challenge: the value the supervisor must answer with
/// [`HeartbeatChallenge::reply`]. It is derived from the boot nonce and the
/// challenge's sequence number, so it is new every period and every boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HeartbeatChallenge(pub u64);

impl HeartbeatChallenge {
    /// The one reply that answers this challenge. The supervisor computes it
    /// from its own build of this crate, so the reply is also bound to the
    /// [`rules_id`] it was built against.
    ///
    /// This proves freshness, not identity: anything that can read the
    /// challenge can compute the reply. The supervisor must answer from its
    /// main loop, never from a task that runs on its own. A key from the
    /// trust root (#172) can make it a MAC later.
    #[must_use]
    pub const fn reply(self) -> HeartbeatReply {
        HeartbeatReply(fnv1a(&[REPLY_TAG, self.0, rules_id()]))
    }
}

/// Domain tags, so a challenge and a reply are never the same function.
const CHALLENGE_TAG: u64 = 0x6567_6e65_6c6c_6168;
const REPLY_TAG: u64 = 0x796c_7065_725f_6268;

/// The supervisor's answer to a [`HeartbeatChallenge`], as it arrived over the
/// link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HeartbeatReply(pub u64);

/// The interlock's side of the heartbeat: the challenge outstanding and
/// whether it has been answered. It lives inside [`State`] and only [`step`]
/// moves it on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HeartbeatWatch {
    /// The boot nonce the challenges are derived from, given with
    /// [`Event::Restart`].
    pub nonce: u64,
    /// The sequence number of the outstanding challenge since the restart.
    pub seq: u64,
    /// The tick the outstanding challenge was issued on.
    pub issued_at: Tick,
    /// The outstanding challenge has been answered correctly.
    pub answered: bool,
    /// A correct reply has arrived since the last restart or loss. Nothing
    /// arms and no fault clears without it.
    pub alive: bool,
}

impl HeartbeatWatch {
    /// A watch that has just issued its first challenge, at `now`, with no
    /// reply yet.
    #[must_use]
    pub const fn start(nonce: u64, now: Tick) -> Self {
        HeartbeatWatch {
            nonce,
            seq: 0,
            issued_at: now,
            answered: false,
            alive: false,
        }
    }

    /// The outstanding challenge.
    #[must_use]
    pub const fn challenge(self) -> HeartbeatChallenge {
        HeartbeatChallenge(fnv1a(&[CHALLENGE_TAG, self.nonce, self.seq]))
    }

    /// One tick: judge the reply that arrived, if any, and issue the next
    /// challenge once the period is over. Returns the watch and whether the
    /// heartbeat was lost on this tick.
    ///
    /// A reply counts only if it is the reply to the outstanding challenge,
    /// that challenge has not been answered yet, and it arrives on a tick
    /// after the challenge was issued and inside the period. Anything else is
    /// a loss, and so is a period that ends unanswered, and so is a sample two
    /// periods or more after the issue (the loop stalled) or with the clock
    /// gone backwards, answered or not. A wrong reply does not
    /// withdraw the outstanding challenge, so a reply already on its way can
    /// still count; only the end of the period issues a new one.
    fn observe(self, reply: Option<HeartbeatReply>, now: Tick) -> (Self, bool) {
        let mut next = self;
        let mut lost = false;
        if let Some(reply) = reply {
            let in_window = match now.checked_sub(self.issued_at) {
                Some(age) => (1..HEARTBEAT_PERIOD_TICKS).contains(&age),
                None => false,
            };
            if in_window && !self.answered && reply.0 == self.challenge().reply().0 {
                next.answered = true;
                next.alive = true;
            } else {
                lost = true;
            }
        }
        if expired(self.issued_at, now, HEARTBEAT_PERIOD_TICKS) {
            // Two periods or more since the issue, or a clock gone back: the
            // next challenge would have fallen due unanswered in between.
            let late = expired(self.issued_at, now, 2 * HEARTBEAT_PERIOD_TICKS);
            if !next.answered || late {
                lost = true;
            }
            next = HeartbeatWatch {
                seq: self.seq.wrapping_add(1),
                issued_at: now,
                answered: false,
                ..next
            };
        }
        if lost {
            next.alive = false;
        }
        (next, lost)
    }
}

/// The pins and flags sampled on one tick, and the supervisor's reply if one
/// arrived. Every combination is a valid sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Inputs {
    /// The emergency stop is pressed.
    pub estop_pressed: bool,
    /// The arm switch is in the on position.
    pub arm_switch_on: bool,
    /// The key switch is in the on position.
    pub key_switch_on: bool,
    /// The supervisor's heartbeat reply that arrived on this tick, if one
    /// did. At most one per tick: a link driver that receives more passes the
    /// rest on later ticks, where a second reply counts as a repeat.
    pub heartbeat: Option<HeartbeatReply>,
    /// The trusted panel's button is held down.
    pub panel_button_pressed: bool,
    /// Rail A's feedback: the rail is sensed live.
    pub rail_a_sensed: bool,
    /// Rail B's feedback per port: the rail is sensed live on that port.
    pub rail_b_sensed: [bool; RAIL_B_PORTS],
    /// The physical firmware-update jumper is fitted.
    pub firmware_jumper_fitted: bool,
    /// The current tick. It must not go backwards; if it does, any lease or
    /// pending request is dropped, a rail sensed dead while commanded on
    /// counts as past its settle window, and a tick before the outstanding
    /// challenge was issued counts as a lost heartbeat. The firmware samples
    /// every tick: a gap of two heartbeat periods is a lost heartbeat too.
    pub now: Tick,
}

impl Inputs {
    /// True if any rail is sensed live.
    #[must_use]
    pub const fn any_rail_sensed(&self) -> bool {
        if self.rail_a_sensed {
            return true;
        }
        let mut p = 0;
        while p < RAIL_B_PORTS {
            if self.rail_b_sensed[p] {
                return true;
            }
            p += 1;
        }
        false
    }
}

/// What the firmware keeps in non-volatile memory so a restart cannot forget
/// a fault. [`State::fault_record`] says what to write after each step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum FaultRecord {
    /// Nothing was armed and no fault was latched. The only record that lets
    /// a restart come up unfaulted.
    Clear,
    /// A rail may have been on. A restart from here cannot prove that no
    /// fault latched, so it comes up with [`FaultReason::FaultStateUnknown`].
    Armed,
    /// This fault was latched.
    Latched(FaultReason),
}

/// Proof that a clear came from the trusted panel's button.
///
/// Only the trust root's own panel driver, which reads the button pin itself,
/// makes one, with [`PanelToken::from_panel_button`]. Nothing that arrives
/// over the link from the supervisor may become one. It serialises (as a
/// unit) for logs but never deserialises, so no bytes from outside can be
/// decoded into it. #172's signed trust-root events will back it with a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelToken(());

impl PanelToken {
    /// For the trust root's panel driver only: a press of the panel button,
    /// read from its pin by this firmware. [`step`] also checks that the
    /// button is sensed pressed on the same tick.
    #[must_use]
    pub const fn from_panel_button() -> Self {
        PanelToken(())
    }
}

#[cfg(feature = "serde")]
impl Serialize for PanelToken {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_unit_struct("PanelToken")
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for PanelToken {
    fn deserialize<D: serde::Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
        Err(serde::de::Error::custom(
            "a panel token is made only by the trust root's panel driver and is never decoded",
        ))
    }
}

/// Where a request to clear a fault came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ClearSource {
    /// A press of the trusted panel's button.
    Panel(PanelToken),
    /// A request from the supervisor over the link. Always refused with
    /// [`Refusal::ClearNotFromPanel`].
    Supervisor,
}

/// Something that happened on this tick, besides the sampled inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Event {
    /// The controller restarted: boot, reset, brownout or crash. Every output
    /// is off from reset, so a rail sensed live on this tick is stuck on.
    Restart {
        /// The fault record read from non-volatile memory, or `None` if it was
        /// missing, unreadable or failed its check.
        record: Option<FaultRecord>,
        /// A fresh nonce from the hardware random number generator, from
        /// which this boot's heartbeat challenges are derived.
        nonce: u64,
    },
    /// The supervisor asks for rail A. This never arms anything, because only
    /// the arm switch can. It is refused with [`Refusal::NotArmed`] unless
    /// rail A is on.
    RequestArm,
    /// The supervisor asks for a lease to carry out one action of this class,
    /// on the port and with the digest the policy engine's decision names.
    RequestLease {
        /// The hazard class of the action.
        class: HazardClass,
        /// The port and action the lease is for.
        target: LeaseTarget,
    },
    /// The leased action has been carried out. The lease is spent. It must
    /// name the lease's own target; any other is refused with
    /// [`Refusal::LeaseMismatch`] and ends the lease.
    LeaseUsed {
        /// The port and action that were carried out.
        target: LeaseTarget,
    },
    /// A request for the latched fault to be cleared. Only one from the panel
    /// can succeed.
    ClearFault {
        /// Where the request came from.
        source: ClearSource,
    },
}

/// A rail, for naming which one a feedback fault is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Rail {
    /// Armed rail A.
    A,
    /// Keyed rail B on one port.
    B(Port),
}

/// Why a fault is latched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum FaultReason {
    /// The emergency stop was pressed.
    EmergencyStop,
    /// The supervisor's heartbeat was lost: a wrong, stale, early or repeated
    /// reply, or no reply in time.
    HeartbeatLost,
    /// A rail's feedback disagreed with its command.
    OutputMismatch {
        /// The rail.
        rail: Rail,
        /// True if it was sensed live while commanded off (stuck on), false
        /// if sensed dead while commanded on past [`OUTPUT_SETTLE_TICKS`].
        sensed_live: bool,
    },
    /// The firmware jumper was fitted while armed.
    FirmwareJumperWhileArmed,
    /// The controller restarted and could not prove that no fault was
    /// latched: no readable [`FaultRecord`], or one written while armed. Also
    /// the fault at [`State::POWER_ON`].
    FaultStateUnknown,
}

/// Where a class 3 approval stands. It exists only inside [`Mode::Armed`], so
/// a lease without an armed rail cannot be written down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Approval {
    /// No request and no lease.
    Idle,
    /// A request is pending, and the panel button has been held down since it
    /// arrived. It must be released before a press can count.
    AwaitRelease {
        /// The tick the request arrived on.
        requested_at: Tick,
        /// What the request is for.
        target: LeaseTarget,
    },
    /// A request is pending and the next press of the panel button grants it.
    AwaitPress {
        /// The tick the request arrived on.
        requested_at: Tick,
        /// What the request is for.
        target: LeaseTarget,
    },
    /// A lease is live. Rail B is on for the target's port.
    Live {
        /// The tick the panel button granted the lease on.
        granted_at: Tick,
        /// What the lease is for.
        target: LeaseTarget,
    },
}

/// Where the interlock stands: armed, disarmed, faulted and so on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Mode {
    /// Not armed, and the arm switch must be seen off before it can arm:
    /// after a clean restart, after the jumper is removed, or after an arm
    /// attempt with no heartbeat.
    AwaitArmOff,
    /// The arm switch has been seen off. Turning it on arms rail A.
    Disarmed,
    /// The firmware jumper is fitted and nothing is armed. Everything is off
    /// and a firmware update is allowed. Removing the jumper needs a fresh
    /// arm sequence.
    FirmwareUpdate,
    /// Rail A is on. Rail B follows the approval inside.
    Armed {
        /// The tick rail A was commanded on.
        since: Tick,
        /// Where the class 3 approval stands.
        approval: Approval,
    },
    /// A fault is latched. Everything is off until it is cleared.
    Faulted(FaultReason),
}

impl Mode {
    /// The mode at power-on: everything off, and the fault state unknown
    /// until a restart's record or the panel settles it.
    pub const POWER_ON: Mode = Mode::Faulted(FaultReason::FaultStateUnknown);

    /// What the firmware must keep in non-volatile memory for this mode.
    #[must_use]
    pub const fn fault_record(self) -> FaultRecord {
        match self {
            Mode::Faulted(reason) => FaultRecord::Latched(reason),
            Mode::Armed { .. } => FaultRecord::Armed,
            Mode::AwaitArmOff | Mode::Disarmed | Mode::FirmwareUpdate => FaultRecord::Clear,
        }
    }
}

/// The interlock's whole memory: its mode and its heartbeat watch.
///
/// A value that loses its memory starts again at [`State::POWER_ON`], which
/// has [`FaultReason::FaultStateUnknown`] latched. The first step after any
/// restart must carry [`Event::Restart`] with the persisted record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct State {
    /// Where the interlock stands.
    pub mode: Mode,
    /// The heartbeat challenge outstanding.
    pub heartbeat: HeartbeatWatch,
}

impl Default for State {
    fn default() -> Self {
        State::POWER_ON
    }
}

impl State {
    /// The state at power-on: everything off, the fault state unknown, and no
    /// heartbeat seen.
    pub const POWER_ON: State = State {
        mode: Mode::POWER_ON,
        heartbeat: HeartbeatWatch::start(0, 0),
    };

    /// What the firmware must keep in non-volatile memory after this step.
    /// See the crate docs for when to write it.
    #[must_use]
    pub const fn fault_record(self) -> FaultRecord {
        self.mode.fault_record()
    }

    /// The outputs this state drives, with no refusal attached.
    ///
    /// A state returned by [`step`] already accounts for the inputs of that
    /// tick, so these are the outputs for that tick.
    #[must_use]
    pub const fn outputs(self) -> Outputs {
        let (rail_a, rail_b, panel, fault) = match self.mode {
            Mode::AwaitArmOff => (false, None, PanelState::AwaitingArmOff, None),
            Mode::Disarmed => (false, None, PanelState::Disarmed, None),
            Mode::FirmwareUpdate => (false, None, PanelState::FirmwareUpdate, None),
            Mode::Armed {
                approval: Approval::Idle,
                ..
            } => (true, None, PanelState::Armed, None),
            Mode::Armed {
                approval: Approval::AwaitRelease { .. } | Approval::AwaitPress { .. },
                ..
            } => (true, None, PanelState::ApprovalRequested, None),
            Mode::Armed {
                approval: Approval::Live { target, .. },
                ..
            } => (true, Some(target.port), PanelState::LeaseLive, None),
            Mode::Faulted(reason) => (false, None, PanelState::Fault, Some(reason)),
        };
        Outputs {
            rail_a,
            rail_b,
            firmware_update_allowed: matches!(self.mode, Mode::FirmwareUpdate),
            panel,
            fault,
            challenge: self.heartbeat.challenge(),
            refusal: None,
        }
    }
}

/// What the trusted panel shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PanelState {
    /// Not armed, with the arm switch to be seen off. Turn it off to begin.
    AwaitingArmOff,
    /// Everything is off. Turn the arm switch on to arm rail A.
    Disarmed,
    /// The firmware jumper is fitted. Everything is off.
    FirmwareUpdate,
    /// Rail A is on.
    Armed,
    /// A class 3 action is waiting for a press of the panel button.
    ApprovalRequested,
    /// A lease is live and rail B is on.
    LeaseLive,
    /// A fault is latched.
    Fault,
}

/// Why an event was refused on this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Refusal {
    /// Hazard class 4 has no rail. It is never permitted.
    ClassNeverPermitted,
    /// Leases are for hazard class 3 only. Classes 0 and 1 need no permission
    /// and class 2 is governed by rail A.
    NoLeaseForClass,
    /// Rail B is not gated for the requested port.
    UnknownPort,
    /// A fault is latched, or was latched on this tick.
    Faulted,
    /// Rail A is off. Only the arm switch can turn it on.
    NotArmed,
    /// The firmware jumper is fitted, so nothing can arm.
    FirmwareJumperFitted,
    /// The key switch is off.
    KeyOff,
    /// A request is already pending or a lease is already live.
    LeaseBusy,
    /// There is no live lease to spend.
    NoLiveLease,
    /// The use named a port or action other than the live lease's. The lease
    /// has been ended.
    LeaseMismatch,
    /// The fault cannot be cleared while the emergency stop is pressed.
    EmergencyStopPressed,
    /// No correct heartbeat reply has arrived since the last restart or loss,
    /// so the fault cannot be cleared and nothing can arm.
    HeartbeatUnhealthy,
    /// The fault cannot be cleared while a rail is sensed live.
    RailSensedLive,
    /// The fault cannot be cleared while the arm switch is on.
    ArmSwitchOn,
    /// A clear came from the supervisor. Only the panel can clear a fault.
    ClearNotFromPanel,
    /// A panel clear arrived but the panel button was not sensed pressed on
    /// that tick.
    PanelButtonNotPressed,
}

/// What the interlock drives on one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Outputs {
    /// Enable for armed rail A (hazard class 2).
    pub rail_a: bool,
    /// The one port keyed rail B (hazard class 3) is enabled for, if any.
    /// Every other port's rail B enable is off.
    pub rail_b: Option<Port>,
    /// A firmware update may run: the jumper is fitted and everything is
    /// disarmed.
    pub firmware_update_allowed: bool,
    /// What the trusted panel shows.
    pub panel: PanelState,
    /// The latched fault, if there is one.
    pub fault: Option<FaultReason>,
    /// The heartbeat challenge outstanding, for the firmware to send to the
    /// supervisor.
    pub challenge: HeartbeatChallenge,
    /// Why this tick's event was refused, if it was.
    pub refusal: Option<Refusal>,
}

impl Outputs {
    /// True if rail B is enabled for `port`.
    #[must_use]
    pub fn rail_b_on(&self, port: Port) -> bool {
        self.rail_b == Some(port)
    }
}

/// Advances the interlock by one tick.
///
/// Deterministic: the result depends only on the arguments. The order within
/// a tick is restart first, then the heartbeat watch, then the hazards
/// (emergency stop, lost heartbeat, output feedback, firmware jumper while
/// armed, in that order of precedence), then everything else, so nothing an
/// event asks for can outrank a hazard.
#[must_use]
pub fn step(state: State, inputs: Inputs, event: Option<Event>) -> (State, Outputs) {
    // Refusals that hold in every state. The event then plays no further part.
    let by_kind = match event {
        Some(Event::RequestLease { class, target }) => match class {
            HazardClass::Class3 if target.port.exists() => None,
            HazardClass::Class3 => Some(Refusal::UnknownPort),
            HazardClass::Class4 => Some(Refusal::ClassNeverPermitted),
            HazardClass::Class0 | HazardClass::Class1 | HazardClass::Class2 => {
                Some(Refusal::NoLeaseForClass)
            }
        },
        Some(Event::ClearFault {
            source: ClearSource::Supervisor,
        }) => Some(Refusal::ClearNotFromPanel),
        _ => None,
    };
    let event = if by_kind.is_some() { None } else { event };
    let restart = match event {
        Some(Event::Restart { record, nonce }) => Some((record, nonce)),
        _ => None,
    };

    // Feedback is judged against what was driven before this tick. After a
    // restart that is nothing: every output is off from reset.
    let driven = if restart.is_some() {
        Mode::POWER_ON
    } else {
        state.mode
    };
    let mismatch = output_mismatch(driven, &inputs);

    // A restart keeps nothing but the fault, from memory or from the record,
    // and starts a new run of challenges from the boot nonce.
    let (mode, watch) = match restart {
        Some((record, nonce)) => (
            restored(state.mode, record),
            HeartbeatWatch::start(nonce, inputs.now),
        ),
        None => (state.mode, state.heartbeat),
    };
    let (heartbeat, lost) = watch.observe(inputs.heartbeat, inputs.now);

    let hazard = if inputs.estop_pressed {
        Some(FaultReason::EmergencyStop)
    } else if lost {
        Some(FaultReason::HeartbeatLost)
    } else if mismatch.is_some() {
        mismatch
    } else if inputs.firmware_jumper_fitted && matches!(mode, Mode::Armed { .. }) {
        Some(FaultReason::FirmwareJumperWhileArmed)
    } else {
        None
    };

    let (next, refusal) = match (mode, hazard) {
        (Mode::Faulted(reason), _) => step_faulted(reason, &inputs, heartbeat.alive, event),
        (_, Some(reason)) => (
            Mode::Faulted(reason),
            refuse_requests(event, Refusal::Faulted),
        ),
        (Mode::AwaitArmOff | Mode::FirmwareUpdate, None) if inputs.firmware_jumper_fitted => (
            Mode::FirmwareUpdate,
            refuse_requests(event, Refusal::FirmwareJumperFitted),
        ),
        (Mode::AwaitArmOff | Mode::FirmwareUpdate, None) => {
            let next = if inputs.arm_switch_on {
                Mode::AwaitArmOff
            } else {
                Mode::Disarmed
            };
            (next, refuse_requests(event, Refusal::NotArmed))
        }
        (Mode::Disarmed, None) => {
            if inputs.firmware_jumper_fitted {
                (
                    Mode::FirmwareUpdate,
                    refuse_requests(event, Refusal::FirmwareJumperFitted),
                )
            } else if inputs.arm_switch_on && !heartbeat.alive {
                // No proof the supervisor is answering. The switch has to go
                // off and on again, so a reply arriving later cannot arm the
                // rail by itself.
                (
                    Mode::AwaitArmOff,
                    refuse_requests(event, Refusal::HeartbeatUnhealthy),
                )
            } else if inputs.arm_switch_on {
                // Armed on this tick: a request for rail A is now satisfied,
                // but a lease has to be asked for again once armed.
                let refusal = match event {
                    Some(Event::RequestArm) => None,
                    _ => refuse_requests(event, Refusal::NotArmed),
                };
                let armed = Mode::Armed {
                    since: inputs.now,
                    approval: Approval::Idle,
                };
                (armed, refusal)
            } else {
                (Mode::Disarmed, refuse_requests(event, Refusal::NotArmed))
            }
        }
        (Mode::Armed { since, approval }, None) => {
            if inputs.arm_switch_on {
                let (approval, refusal) = step_approval(approval, &inputs, event);
                (Mode::Armed { since, approval }, refusal)
            } else {
                (Mode::Disarmed, refuse_requests(event, Refusal::NotArmed))
            }
        }
    };

    let next = State {
        mode: next,
        heartbeat,
    };
    let outputs = Outputs {
        refusal: by_kind.or(refusal),
        ..next.outputs()
    };
    (next, outputs)
}

/// The mode a restart comes up in: never cleaner than the worse of the mode
/// passed in and the record. [`Mode::POWER_ON`] passed in says nothing, so a
/// firmware that lost its memory gets exactly what the record says.
fn restored(kept: Mode, record: Option<FaultRecord>) -> Mode {
    let unknown = FaultReason::FaultStateUnknown;
    match (kept, record) {
        (Mode::Faulted(reason), _) if !matches!(reason, FaultReason::FaultStateUnknown) => kept,
        (_, Some(FaultRecord::Latched(reason))) => Mode::Faulted(reason),
        (Mode::Armed { .. }, _) | (_, Some(FaultRecord::Armed) | None) => Mode::Faulted(unknown),
        (_, Some(FaultRecord::Clear)) => Mode::AwaitArmOff,
    }
}

/// The first disagreement between the rails `driven` commands and the rails
/// sensed this tick: rail A first, then rail B by port. A rail commanded off
/// and sensed live is stuck on at once; a rail commanded on and sensed dead is
/// a fault only once [`OUTPUT_SETTLE_TICKS`] have passed since it was
/// commanded on.
fn output_mismatch(driven: Mode, inputs: &Inputs) -> Option<FaultReason> {
    let now = inputs.now;
    let (rail_a_since, rail_b) = match driven {
        Mode::Armed { since, approval } => {
            let rail_b = match approval {
                Approval::Live { granted_at, target } => Some((target.port, granted_at)),
                _ => None,
            };
            (Some(since), rail_b)
        }
        _ => (None, None),
    };

    let judge = |rail: Rail, commanded_since: Option<Tick>, sensed: bool| match commanded_since {
        None if sensed => Some(FaultReason::OutputMismatch {
            rail,
            sensed_live: true,
        }),
        Some(since) if !sensed && expired(since, now, OUTPUT_SETTLE_TICKS) => {
            Some(FaultReason::OutputMismatch {
                rail,
                sensed_live: false,
            })
        }
        _ => None,
    };

    if let Some(fault) = judge(Rail::A, rail_a_since, inputs.rail_a_sensed) {
        return Some(fault);
    }
    for (index, &sensed) in inputs.rail_b_sensed.iter().enumerate() {
        // In range: RAIL_B_PORTS fits in a u8, checked at compile time.
        #[allow(clippy::cast_possible_truncation)]
        let port = Port(index as u8);
        let since = match rail_b {
            Some((leased, granted_at)) if leased == port => Some(granted_at),
            _ => None,
        };
        if let Some(fault) = judge(Rail::B(port), since, sensed) {
            return Some(fault);
        }
    }
    None
}

/// The refusal for an event that asks for something, when nothing can be
/// given. A restart and a clear with nothing to clear ask for nothing.
fn refuse_requests(event: Option<Event>, why: Refusal) -> Option<Refusal> {
    match event {
        Some(Event::RequestArm | Event::RequestLease { .. } | Event::LeaseUsed { .. }) => Some(why),
        Some(Event::ClearFault { .. }) if matches!(why, Refusal::Faulted) => Some(why),
        Some(Event::Restart { .. } | Event::ClearFault { .. }) | None => None,
    }
}

/// One tick with a fault latched. The first reason is kept. A clear from the
/// supervisor never reaches here: it is refused before any state is looked at.
fn step_faulted(
    reason: FaultReason,
    inputs: &Inputs,
    heartbeat_alive: bool,
    event: Option<Event>,
) -> (Mode, Option<Refusal>) {
    let stay = Mode::Faulted(reason);
    match event {
        Some(Event::ClearFault {
            source: ClearSource::Panel(_),
        }) => {
            if !inputs.panel_button_pressed {
                (stay, Some(Refusal::PanelButtonNotPressed))
            } else if inputs.estop_pressed {
                (stay, Some(Refusal::EmergencyStopPressed))
            } else if !heartbeat_alive {
                (stay, Some(Refusal::HeartbeatUnhealthy))
            } else if inputs.any_rail_sensed() {
                (stay, Some(Refusal::RailSensedLive))
            } else if inputs.arm_switch_on {
                (stay, Some(Refusal::ArmSwitchOn))
            } else {
                // The arm switch is off right now, so it has been seen off.
                (Mode::Disarmed, None)
            }
        }
        _ => (stay, refuse_requests(event, Refusal::Faulted)),
    }
}

/// One tick of the class 3 approval while armed with no hazard present.
fn step_approval(
    approval: Approval,
    inputs: &Inputs,
    event: Option<Event>,
) -> (Approval, Option<Refusal>) {
    let request = match event {
        Some(Event::RequestLease { target, .. }) => Some(target),
        _ => None,
    };
    let used = match event {
        Some(Event::LeaseUsed { target }) => Some(target),
        _ => None,
    };
    let pressed = inputs.panel_button_pressed;
    let now = inputs.now;

    // Taking the key out withdraws any request or lease.
    if !inputs.key_switch_on {
        let refusal = if request.is_some() {
            Some(Refusal::KeyOff)
        } else if used.is_some() {
            Some(Refusal::NoLiveLease)
        } else {
            None
        };
        return (Approval::Idle, refusal);
    }

    // Age out a stale request or an expired lease before anything else.
    let approval = match approval {
        Approval::AwaitRelease { requested_at, .. } | Approval::AwaitPress { requested_at, .. }
            if expired(requested_at, now, REQUEST_TTL_TICKS) =>
        {
            Approval::Idle
        }
        Approval::Live { granted_at, .. } if expired(granted_at, now, LEASE_TTL_TICKS) => {
            Approval::Idle
        }
        other => other,
    };

    let busy = if request.is_some() {
        Some(Refusal::LeaseBusy)
    } else if used.is_some() {
        Some(Refusal::NoLiveLease)
    } else {
        None
    };

    match approval {
        Approval::Idle => match request {
            Some(target) => {
                // A button already held down is not an answer to this request.
                let pending = if pressed {
                    Approval::AwaitRelease {
                        requested_at: now,
                        target,
                    }
                } else {
                    Approval::AwaitPress {
                        requested_at: now,
                        target,
                    }
                };
                (pending, None)
            }
            None => (Approval::Idle, busy),
        },
        Approval::AwaitRelease {
            requested_at,
            target,
        } if !pressed => (
            Approval::AwaitPress {
                requested_at,
                target,
            },
            busy,
        ),
        Approval::AwaitPress { target, .. } if pressed => (
            Approval::Live {
                granted_at: now,
                target,
            },
            busy,
        ),
        Approval::AwaitRelease { .. } | Approval::AwaitPress { .. } => (approval, busy),
        Approval::Live { target, .. } => match used {
            Some(named) if named == target => (Approval::Idle, None),
            // A use for anything else is wrong bookkeeping at best. Refuse it
            // and end the lease, so rail B goes off.
            Some(_) => (Approval::Idle, Some(Refusal::LeaseMismatch)),
            None => (approval, busy),
        },
    }
}

/// True once `ttl` ticks have passed since `since`, or if the tick went
/// backwards.
fn expired(since: Tick, now: Tick, ttl: Tick) -> bool {
    now.checked_sub(since).is_none_or(|age| age >= ttl)
}
