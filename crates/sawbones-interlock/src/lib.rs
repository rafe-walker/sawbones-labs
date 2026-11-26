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
//! - **Off after restart.** Every output is off at power-on and after
//!   [`Event::Restart`], and stays off until the arm switch has been seen off
//!   and then on. A switch left on does not re-arm.
//! - **Emergency stop and heartbeat.** A pressed emergency stop or a lost
//!   heartbeat turns every output off on that tick and latches a fault.
//! - **Output feedback.** Each rail's sensed state is compared with what the
//!   previous tick commanded. A rail sensed live while commanded off is stuck
//!   on: everything drops and [`FaultReason::OutputMismatch`] latches on that
//!   tick. A rail sensed dead while commanded on latches the same fault once
//!   it has been commanded on for [`OUTPUT_SETTLE_TICKS`].
//! - **Latched fault.** A fault clears only on [`Event::ClearFault`] while the
//!   emergency stop is released, the heartbeat is healthy, no rail is sensed
//!   live and the arm switch is off. A restart does not clear it.
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
//!   interlock enters [`State::FirmwareUpdate`], the only state in which
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
//! What is deliberately not here yet: the heartbeat's own timing, the trusted
//! display's content beyond [`PanelState`], the preflight token that binds a
//! lease to fact versions and the chain head, and any I/O. The caller samples
//! the pins, decides whether the heartbeat is healthy, and supplies the tick.

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

// Every port number must fit in a `Port`.
const _: () = assert!(RAIL_B_PORTS >= 1 && RAIL_B_PORTS <= 256);

/// The version of the rule set. Bump it with any change to what [`step`]
/// does, however small. The test `rules_version_pins_the_transition_table`
/// fails when the transitions change and this is not bumped.
pub const RULES_VERSION: u32 = 2;

/// The id of the rule set this build enforces: a stable 64-bit hash of
/// [`RULES_VERSION`] and every placeholder constant ([`LEASE_TTL_TICKS`],
/// [`REQUEST_TTL_TICKS`], [`OUTPUT_SETTLE_TICKS`], [`RAIL_B_PORTS`]).
///
/// This is what the trust root reports. The supervisor compares it with the
/// id it was built and reviewed against and refuses to plan if they differ,
/// so a plan can never rest on rules the trust root does not enforce.
///
/// The hash is FNV-1a over the little-endian bytes of each value as a `u64`,
/// in the order above. It is an identifier, not a security measure.
#[must_use]
pub const fn rules_id() -> u64 {
    let words = [
        RULES_VERSION as u64,
        LEASE_TTL_TICKS,
        REQUEST_TTL_TICKS,
        OUTPUT_SETTLE_TICKS,
        RAIL_B_PORTS as u64,
    ];
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
/// the policy engine's `ActionDigest`, which is the shared encoder's digest
/// of one bus write action, the same bytes the trust root's approval and the
/// bus guard's lease carry (#203). The interlock only compares it; it never
/// computes one.
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

/// The pins and flags sampled on one tick. Every combination is a valid
/// sample, so these are plain booleans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Inputs {
    /// The emergency stop is pressed.
    pub estop_pressed: bool,
    /// The arm switch is in the on position.
    pub arm_switch_on: bool,
    /// The key switch is in the on position.
    pub key_switch_on: bool,
    /// The supervisor's heartbeat is arriving on time. The watchdog that
    /// decides this lives outside this crate.
    pub heartbeat_healthy: bool,
    /// The trusted panel's button is held down.
    pub panel_button_pressed: bool,
    /// Rail A's feedback: the rail is sensed live.
    pub rail_a_sensed: bool,
    /// Rail B's feedback per port: the rail is sensed live on that port.
    pub rail_b_sensed: [bool; RAIL_B_PORTS],
    /// The physical firmware-update jumper is fitted.
    pub firmware_jumper_fitted: bool,
    /// The current tick. It must not go backwards; if it does, any lease or
    /// pending request is dropped, and a rail sensed dead while commanded on
    /// counts as past its settle window.
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

/// Something that happened on this tick, besides the sampled inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Event {
    /// The controller restarted: boot, reset, brownout or crash. Every output
    /// is off from reset, so a rail sensed live on this tick is stuck on.
    Restart,
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
    /// A person asks for the latched fault to be cleared.
    ClearFault,
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
    /// The supervisor's heartbeat was lost.
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
}

/// Where a class 3 approval stands. It exists only inside
/// [`State::Armed`], so a lease without an armed rail cannot be written down.
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

/// The interlock's whole memory.
///
/// A value that loses its memory starts again at [`State::POWER_ON`]. A caller
/// that keeps this value across a restart must feed [`Event::Restart`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum State {
    /// Restarted. Everything is off until the arm switch is seen off.
    #[default]
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

impl State {
    /// The state at power-on: everything off, waiting to see the arm switch
    /// off.
    pub const POWER_ON: State = State::AwaitArmOff;

    /// The outputs this state drives, with no refusal attached.
    ///
    /// A state returned by [`step`] already accounts for the inputs of that
    /// tick, so these are the outputs for that tick.
    #[must_use]
    pub const fn outputs(self) -> Outputs {
        let (rail_a, rail_b, panel, fault) = match self {
            State::AwaitArmOff => (false, None, PanelState::AwaitingArmOff, None),
            State::Disarmed => (false, None, PanelState::Disarmed, None),
            State::FirmwareUpdate => (false, None, PanelState::FirmwareUpdate, None),
            State::Armed {
                approval: Approval::Idle,
                ..
            } => (true, None, PanelState::Armed, None),
            State::Armed {
                approval: Approval::AwaitRelease { .. } | Approval::AwaitPress { .. },
                ..
            } => (true, None, PanelState::ApprovalRequested, None),
            State::Armed {
                approval: Approval::Live { target, .. },
                ..
            } => (true, Some(target.port), PanelState::LeaseLive, None),
            State::Faulted(reason) => (false, None, PanelState::Fault, Some(reason)),
        };
        Outputs {
            rail_a,
            rail_b,
            firmware_update_allowed: matches!(self, State::FirmwareUpdate),
            panel,
            fault,
            refusal: None,
        }
    }
}

/// What the trusted panel shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PanelState {
    /// Restarted with the arm switch on. Turn it off to begin.
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
    /// The fault cannot be cleared while the heartbeat is unhealthy.
    HeartbeatUnhealthy,
    /// The fault cannot be cleared while a rail is sensed live.
    RailSensedLive,
    /// The fault cannot be cleared while the arm switch is on.
    ArmSwitchOn,
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
/// a tick is restart first, then the hazards (emergency stop, heartbeat,
/// output feedback, firmware jumper while armed, in that order of
/// precedence), then everything else, so nothing an event asks for can
/// outrank a hazard.
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
        _ => None,
    };
    let event = if by_kind.is_some() { None } else { event };
    let restart = matches!(event, Some(Event::Restart));

    // Feedback is judged against what was driven before this tick. After a
    // restart that is nothing: every output is off from reset.
    let driven = if restart { State::POWER_ON } else { state };
    let mismatch = output_mismatch(driven, &inputs);

    // A restart drops everything except a latched fault.
    let state = match state {
        State::Faulted(_) => state,
        _ if restart => State::AwaitArmOff,
        _ => state,
    };

    let hazard = if inputs.estop_pressed {
        Some(FaultReason::EmergencyStop)
    } else if !inputs.heartbeat_healthy {
        Some(FaultReason::HeartbeatLost)
    } else if mismatch.is_some() {
        mismatch
    } else if inputs.firmware_jumper_fitted && matches!(state, State::Armed { .. }) {
        Some(FaultReason::FirmwareJumperWhileArmed)
    } else {
        None
    };

    let (next, refusal) = match (state, hazard) {
        (State::Faulted(reason), _) => step_faulted(reason, &inputs, event),
        (_, Some(reason)) => (
            State::Faulted(reason),
            refuse_requests(event, Refusal::Faulted),
        ),
        (State::AwaitArmOff | State::FirmwareUpdate, None) if inputs.firmware_jumper_fitted => (
            State::FirmwareUpdate,
            refuse_requests(event, Refusal::FirmwareJumperFitted),
        ),
        (State::AwaitArmOff | State::FirmwareUpdate, None) => {
            let next = if inputs.arm_switch_on {
                State::AwaitArmOff
            } else {
                State::Disarmed
            };
            (next, refuse_requests(event, Refusal::NotArmed))
        }
        (State::Disarmed, None) => {
            if inputs.firmware_jumper_fitted {
                (
                    State::FirmwareUpdate,
                    refuse_requests(event, Refusal::FirmwareJumperFitted),
                )
            } else if inputs.arm_switch_on {
                // Armed on this tick: a request for rail A is now satisfied,
                // but a lease has to be asked for again once armed.
                let refusal = match event {
                    Some(Event::RequestArm) => None,
                    _ => refuse_requests(event, Refusal::NotArmed),
                };
                let armed = State::Armed {
                    since: inputs.now,
                    approval: Approval::Idle,
                };
                (armed, refusal)
            } else {
                (State::Disarmed, refuse_requests(event, Refusal::NotArmed))
            }
        }
        (State::Armed { since, approval }, None) => {
            if inputs.arm_switch_on {
                let (approval, refusal) = step_approval(approval, &inputs, event);
                (State::Armed { since, approval }, refusal)
            } else {
                (State::Disarmed, refuse_requests(event, Refusal::NotArmed))
            }
        }
    };

    let outputs = Outputs {
        refusal: by_kind.or(refusal),
        ..next.outputs()
    };
    (next, outputs)
}

/// The first disagreement between the rails `driven` commands and the rails
/// sensed this tick: rail A first, then rail B by port. A rail commanded off
/// and sensed live is stuck on at once; a rail commanded on and sensed dead is
/// a fault only once [`OUTPUT_SETTLE_TICKS`] have passed since it was
/// commanded on.
fn output_mismatch(driven: State, inputs: &Inputs) -> Option<FaultReason> {
    let now = inputs.now;
    let (rail_a_since, rail_b) = match driven {
        State::Armed { since, approval } => {
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
        Some(Event::ClearFault) if matches!(why, Refusal::Faulted) => Some(why),
        Some(Event::Restart | Event::ClearFault) | None => None,
    }
}

/// One tick with a fault latched. The first reason is kept.
fn step_faulted(
    reason: FaultReason,
    inputs: &Inputs,
    event: Option<Event>,
) -> (State, Option<Refusal>) {
    let stay = State::Faulted(reason);
    match event {
        Some(Event::ClearFault) => {
            if inputs.estop_pressed {
                (stay, Some(Refusal::EmergencyStopPressed))
            } else if !inputs.heartbeat_healthy {
                (stay, Some(Refusal::HeartbeatUnhealthy))
            } else if inputs.any_rail_sensed() {
                (stay, Some(Refusal::RailSensedLive))
            } else if inputs.arm_switch_on {
                (stay, Some(Refusal::ArmSwitchOn))
            } else {
                // The arm switch is off right now, so it has been seen off.
                (State::Disarmed, None)
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
