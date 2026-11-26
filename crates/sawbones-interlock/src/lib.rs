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
//! - **Latched fault.** A fault clears only on [`Event::ClearFault`] while the
//!   emergency stop is released, the heartbeat is healthy and the arm switch
//!   is off. A restart does not clear it.
//! - **Rail A (hazard class 2)** is on while armed: no fault, the arm switch
//!   on, and a valid arm sequence behind it.
//! - **Rail B (hazard class 3)** also needs the key switch on and a live
//!   lease. A lease is granted only by a fresh press of the panel button while
//!   a request is pending. It ends after [`LEASE_TTL_TICKS`], on
//!   [`Event::LeaseUsed`], or when the key or the arm switch goes off.
//! - **Hazard class 4** has no rail. A request for it is always refused.
//!
//! No event can turn a rail on by itself. Rail A comes on with the arm switch
//! and rail B with the panel button, both of which are a person's hand.
//!
//! What is deliberately not here yet: leases bound to a target and a parameter
//! set, output feedback (a stuck contactor), the firmware-update jumper, the
//! heartbeat's own timing, and any I/O. The caller samples the pins, decides
//! whether the heartbeat is healthy, and supplies the tick.

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
    /// The current tick. It must not go backwards; if it does, any lease or
    /// pending request is dropped.
    pub now: Tick,
}

/// Something that happened on this tick, besides the sampled inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Event {
    /// The controller restarted: boot, reset, brownout or crash.
    Restart,
    /// The supervisor asks for rail A. This never arms anything, because only
    /// the arm switch can. It is refused with [`Refusal::NotArmed`] unless
    /// rail A is on.
    RequestArm,
    /// The supervisor asks for a lease to carry out one action of this class.
    RequestLease {
        /// The hazard class of the action.
        class: HazardClass,
    },
    /// The leased action has been carried out. The lease is spent.
    LeaseUsed,
    /// A person asks for the latched fault to be cleared.
    ClearFault,
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
    },
    /// A request is pending and the next press of the panel button grants it.
    AwaitPress {
        /// The tick the request arrived on.
        requested_at: Tick,
    },
    /// A lease is live. Rail B is on.
    Live {
        /// The tick the panel button granted the lease on.
        granted_at: Tick,
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
    /// Rail A is on. Rail B follows the approval inside.
    Armed(Approval),
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
            State::AwaitArmOff => (false, false, PanelState::AwaitingArmOff, None),
            State::Disarmed => (false, false, PanelState::Disarmed, None),
            State::Armed(Approval::Idle) => (true, false, PanelState::Armed, None),
            State::Armed(Approval::AwaitRelease { .. } | Approval::AwaitPress { .. }) => {
                (true, false, PanelState::ApprovalRequested, None)
            }
            State::Armed(Approval::Live { .. }) => (true, true, PanelState::LeaseLive, None),
            State::Faulted(reason) => (false, false, PanelState::Fault, Some(reason)),
        };
        Outputs {
            rail_a,
            rail_b,
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
    /// A fault is latched, or was latched on this tick.
    Faulted,
    /// Rail A is off. Only the arm switch can turn it on.
    NotArmed,
    /// The key switch is off.
    KeyOff,
    /// A request is already pending or a lease is already live.
    LeaseBusy,
    /// There is no live lease to spend.
    NoLiveLease,
    /// The fault cannot be cleared while the emergency stop is pressed.
    EmergencyStopPressed,
    /// The fault cannot be cleared while the heartbeat is unhealthy.
    HeartbeatUnhealthy,
    /// The fault cannot be cleared while the arm switch is on.
    ArmSwitchOn,
}

/// What the interlock drives on one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Outputs {
    /// Enable for armed rail A (hazard class 2).
    pub rail_a: bool,
    /// Enable for keyed rail B (hazard class 3).
    pub rail_b: bool,
    /// What the trusted panel shows.
    pub panel: PanelState,
    /// The latched fault, if there is one.
    pub fault: Option<FaultReason>,
    /// Why this tick's event was refused, if it was.
    pub refusal: Option<Refusal>,
}

/// Advances the interlock by one tick.
///
/// Deterministic: the result depends only on the arguments. The order within
/// a tick is restart first, then emergency stop and heartbeat, then
/// everything else, so nothing an event asks for can outrank a hazard.
#[must_use]
pub fn step(state: State, inputs: Inputs, event: Option<Event>) -> (State, Outputs) {
    // Refusals that hold in every state. The event then plays no further part.
    let by_class = match event {
        Some(Event::RequestLease { class }) => match class {
            HazardClass::Class3 => None,
            HazardClass::Class4 => Some(Refusal::ClassNeverPermitted),
            HazardClass::Class0 | HazardClass::Class1 | HazardClass::Class2 => {
                Some(Refusal::NoLeaseForClass)
            }
        },
        _ => None,
    };
    let event = if by_class.is_some() { None } else { event };

    // A restart drops everything except a latched fault.
    let state = match (state, event) {
        (State::Faulted(_), _) => state,
        (_, Some(Event::Restart)) => State::AwaitArmOff,
        _ => state,
    };

    let hazard = if inputs.estop_pressed {
        Some(FaultReason::EmergencyStop)
    } else if !inputs.heartbeat_healthy {
        Some(FaultReason::HeartbeatLost)
    } else {
        None
    };

    let (next, refusal) = match (state, hazard) {
        (State::Faulted(reason), _) => step_faulted(reason, inputs, event),
        (_, Some(reason)) => (
            State::Faulted(reason),
            refuse_requests(event, Refusal::Faulted),
        ),
        (State::AwaitArmOff, None) => {
            let next = if inputs.arm_switch_on {
                State::AwaitArmOff
            } else {
                State::Disarmed
            };
            (next, refuse_requests(event, Refusal::NotArmed))
        }
        (State::Disarmed, None) => {
            if inputs.arm_switch_on {
                // Armed on this tick: a request for rail A is now satisfied,
                // but a lease has to be asked for again once armed.
                let refusal = match event {
                    Some(Event::RequestArm) => None,
                    _ => refuse_requests(event, Refusal::NotArmed),
                };
                (State::Armed(Approval::Idle), refusal)
            } else {
                (State::Disarmed, refuse_requests(event, Refusal::NotArmed))
            }
        }
        (State::Armed(approval), None) => {
            if inputs.arm_switch_on {
                let (approval, refusal) = step_approval(approval, inputs, event);
                (State::Armed(approval), refusal)
            } else {
                (State::Disarmed, refuse_requests(event, Refusal::NotArmed))
            }
        }
    };

    let outputs = Outputs {
        refusal: by_class.or(refusal),
        ..next.outputs()
    };
    (next, outputs)
}

/// The refusal for an event that asks for something, when nothing can be
/// given. A restart and a clear with nothing to clear ask for nothing.
fn refuse_requests(event: Option<Event>, why: Refusal) -> Option<Refusal> {
    match event {
        Some(Event::RequestArm | Event::RequestLease { .. } | Event::LeaseUsed) => Some(why),
        Some(Event::ClearFault) if matches!(why, Refusal::Faulted) => Some(why),
        Some(Event::Restart | Event::ClearFault) | None => None,
    }
}

/// One tick with a fault latched. The first reason is kept.
fn step_faulted(
    reason: FaultReason,
    inputs: Inputs,
    event: Option<Event>,
) -> (State, Option<Refusal>) {
    let stay = State::Faulted(reason);
    match event {
        Some(Event::ClearFault) => {
            if inputs.estop_pressed {
                (stay, Some(Refusal::EmergencyStopPressed))
            } else if !inputs.heartbeat_healthy {
                (stay, Some(Refusal::HeartbeatUnhealthy))
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
    inputs: Inputs,
    event: Option<Event>,
) -> (Approval, Option<Refusal>) {
    let request = matches!(event, Some(Event::RequestLease { .. }));
    let used = matches!(event, Some(Event::LeaseUsed));
    let pressed = inputs.panel_button_pressed;
    let now = inputs.now;

    // Taking the key out withdraws any request or lease.
    if !inputs.key_switch_on {
        let refusal = if request {
            Some(Refusal::KeyOff)
        } else if used {
            Some(Refusal::NoLiveLease)
        } else {
            None
        };
        return (Approval::Idle, refusal);
    }

    // Age out a stale request or an expired lease before anything else.
    let approval = match approval {
        Approval::AwaitRelease { requested_at } | Approval::AwaitPress { requested_at }
            if expired(requested_at, now, REQUEST_TTL_TICKS) =>
        {
            Approval::Idle
        }
        Approval::Live { granted_at } if expired(granted_at, now, LEASE_TTL_TICKS) => {
            Approval::Idle
        }
        other => other,
    };

    let busy = if request {
        Some(Refusal::LeaseBusy)
    } else if used {
        Some(Refusal::NoLiveLease)
    } else {
        None
    };

    match approval {
        Approval::Idle if request => {
            // A button already held down is not an answer to this request.
            let pending = if pressed {
                Approval::AwaitRelease { requested_at: now }
            } else {
                Approval::AwaitPress { requested_at: now }
            };
            (pending, None)
        }
        Approval::Idle => (Approval::Idle, busy),
        Approval::AwaitRelease { requested_at } if !pressed => {
            (Approval::AwaitPress { requested_at }, busy)
        }
        Approval::AwaitPress { .. } if pressed => (Approval::Live { granted_at: now }, busy),
        Approval::AwaitRelease { .. } | Approval::AwaitPress { .. } => (approval, busy),
        Approval::Live { .. } if used => (Approval::Idle, None),
        Approval::Live { .. } => (approval, busy),
    }
}

/// True once `ttl` ticks have passed since `since`, or if the tick went
/// backwards.
fn expired(since: Tick, now: Tick, ttl: Tick) -> bool {
    now.checked_sub(since).is_none_or(|age| age >= ttl)
}
