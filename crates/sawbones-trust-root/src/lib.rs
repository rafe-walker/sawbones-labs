//! # sawbones-trust-root: PROVISIONAL
//!
//! **Provisional, like the interlock crate it drives. Every type here may
//! change.**
//!
//! The main loop of the trust root, the RP2350 interlock controller, written
//! against the [`Board`] trait instead of real pins. The firmware image
//! (`firmware/trust-root`) implements [`Board`] for the chip and calls
//! [`TrustRoot::tick`] forever; the host tests implement it with a simulated
//! board and check the same loop. This is the firmware's half of the contract
//! that `sawbones-interlock` states but cannot enforce itself:
//!
//! - **Boot.** Every output is driven off when the loop is made. The first
//!   tick reads the persisted [`FaultRecord`] (a record that fails its check
//!   value is passed on as `None`), takes a nonce from the hardware random
//!   number generator and makes its one event [`Event::Restart`] with both.
//!   A board with no working generator gives no nonce (`None`), which
//!   leaves the heartbeat not started, so nothing can arm.
//! - **Outputs come only from [`step`].** Each tick samples the pins, calls
//!   [`step`] once, and drives the rails only from the [`Outputs`] it
//!   returned.
//! - **Cut first, write the record before anything turns on.** Outputs that
//!   go off are driven off before anything else, and the record is written
//!   after. Before any output that was off is driven on, the new record is
//!   written and read back; if that fails, the output stays off this tick.
//!   The interlock's own feedback check then latches a fault once the rail
//!   has been commanded on for its settle window without being sensed live.
//! - **The charge pump is fed only here.** [`Board::toggle_charge_pump`] is
//!   called from one place, once per tick, after [`step`] has returned a rail
//!   on and that rail has been driven on. Never from a timer, PWM, PIO, DMA or
//!   interrupt: `ci/check_charge_pump.py` greps the trust-root sources for
//!   them and for a second call site.
//! - **A panel token is made only from the panel pin.** A clear request from
//!   the link becomes [`ClearSource::Supervisor`], which the interlock always
//!   refuses. [`PanelToken::from_panel_button`] is called only on a press of
//!   the panel button sensed by [`Board::sample`] while a fault is latched.
//!
//! The link protocol to the supervisor, the panel display and a durable
//! record store are not here yet (#144).

#![no_std]
#![forbid(unsafe_code)]

pub mod record;

use core::num::NonZeroU64;

pub use record::{RecordWords, RECORD_WORDS};
use sawbones_interlock::{
    step, Approval, ClearSource, Event, FaultRecord, HazardClass, HeartbeatReply, Inputs,
    LeaseTarget, Mode, Outputs, PanelToken, Port, State, Tick, RAIL_B_PORTS,
};

/// The pins the trust root samples on each tick, already turned into what
/// they mean (the board knows which level is pressed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pins {
    /// The emergency stop is pressed, or its loop is broken.
    pub estop_pressed: bool,
    /// The arm switch is on.
    pub arm_switch_on: bool,
    /// The key switch is on.
    pub key_switch_on: bool,
    /// The trusted panel's button is held down.
    pub panel_button_pressed: bool,
    /// Rail A is sensed live.
    pub rail_a_sensed: bool,
    /// Rail B is sensed live, per port.
    pub rail_b_sensed: [bool; RAIL_B_PORTS],
    /// The firmware-update jumper is fitted.
    pub firmware_jumper_fitted: bool,
}

/// A request from the supervisor over the link. There is deliberately no way
/// to carry a [`PanelToken`] here: a clear from the link is always
/// [`ClearSource::Supervisor`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkRequest {
    /// [`Event::RequestArm`].
    RequestArm,
    /// [`Event::RequestLease`].
    RequestLease {
        /// The hazard class of the action.
        class: HazardClass,
        /// The port and action the lease is for.
        target: LeaseTarget,
    },
    /// [`Event::LeaseUsed`].
    LeaseUsed {
        /// The port and action that were carried out.
        target: LeaseTarget,
    },
    /// A clear asked for by the supervisor. Always refused by the interlock.
    ClearFault,
}

impl LinkRequest {
    /// The interlock event this request becomes.
    #[must_use]
    pub const fn into_event(self) -> Event {
        match self {
            LinkRequest::RequestArm => Event::RequestArm,
            LinkRequest::RequestLease { class, target } => Event::RequestLease { class, target },
            LinkRequest::LeaseUsed { target } => Event::LeaseUsed { target },
            LinkRequest::ClearFault => Event::ClearFault {
                source: ClearSource::Supervisor,
            },
        }
    }
}

/// The record store could not take the write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordWriteFailed;

/// The hardware the loop drives. Every method returns promptly: nothing here
/// may block for longer than a fraction of a tick.
pub trait Board {
    /// Drives the rail enables: rail A, and rail B for at most one port. Every
    /// other enable is off. The levels are set when this returns.
    fn drive_rails(&mut self, rail_a: bool, rail_b: Option<Port>);
    /// Inverts the charge-pump pin once, by a plain software write. Called by
    /// the loop only; see the crate docs.
    fn toggle_charge_pump(&mut self);
    /// Reads the stored fault record, or `None` if there is none.
    fn read_record(&mut self) -> Option<RecordWords>;
    /// Stores the fault record. Returns `Ok` only once the write is durable.
    ///
    /// # Errors
    ///
    /// [`RecordWriteFailed`] if the store did not take the write.
    fn write_record(&mut self, words: &RecordWords) -> Result<(), RecordWriteFailed>;
    /// A fresh nonce from the hardware random number generator, or `None` if
    /// it failed its health checks or is not available.
    fn hardware_nonce(&mut self) -> Option<NonZeroU64>;
    /// The current tick. Monotonic.
    fn now(&mut self) -> Tick;
    /// Samples the pins.
    fn sample(&mut self) -> Pins;
    /// The supervisor's heartbeat reply received since the last tick, if any.
    /// At most one per tick; the link driver keeps the rest for later ticks.
    fn heartbeat_reply(&mut self) -> Option<HeartbeatReply>;
    /// The next request from the supervisor, if any. Called only on a tick
    /// with no other event, so the link driver keeps it until then.
    fn link_request(&mut self) -> Option<LinkRequest>;
    /// Hands the tick's result to the link and the panel.
    fn report(&mut self, report: &TickReport);
    /// Feeds the hardware watchdog. Called once at the end of every tick.
    fn feed_watchdog(&mut self);
}

/// What happened to the fault record on one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordWrite {
    /// No write was needed.
    None,
    /// This record was written and read back intact.
    Written(FaultRecord),
    /// The write of this record failed or did not read back. Anything that
    /// was to turn on stayed off.
    Failed(FaultRecord),
}

/// Where the trust root's event on a tick came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSource {
    /// No event.
    None,
    /// The boot [`Event::Restart`].
    Restart,
    /// A press of the panel button while a fault was latched.
    Panel,
    /// A request from the supervisor.
    Link,
}

/// The result of one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickReport {
    /// What [`step`] returned.
    pub outputs: Outputs,
    /// Rail A as driven at the end of the tick.
    pub rail_a_driven: bool,
    /// Rail B's port as driven at the end of the tick.
    pub rail_b_driven: Option<Port>,
    /// The charge pump was toggled on this tick.
    pub pump_toggled: bool,
    /// What happened to the fault record.
    pub record: RecordWrite,
    /// Where this tick's event came from.
    pub event: EventSource,
}

/// Which rails may be driven on: a rail turns on only if two independent
/// reads agree, so one corrupted or glitched value fails safe (rail off).
///
/// - Read 1 is the [`Outputs`] that [`step`] returned.
/// - Read 2 is the [`State`] mode the same step returned: rail A needs
///   [`Mode::Armed`], and rail B needs a live lease in that mode for the same
///   port.
///
/// A rail is on only if both say on. Either one saying off, or the two
/// disagreeing, gives off. This is a design-level defence against a single
/// flipped bool or skipped instruction (docs/THREAT-MODEL.md, section 4a,
/// #238). It does not stop a determined attacker with the board on a bench.
#[must_use]
pub fn permitted(outputs: &Outputs, state: &State) -> Driven {
    let (armed, lease_port) = match state.mode {
        Mode::Armed {
            approval: Approval::Live { target, .. },
            ..
        } => (true, Some(target.port)),
        Mode::Armed { .. } => (true, None),
        _ => (false, None),
    };
    let rail_a = outputs.rail_a && armed && outputs.fault.is_none();
    let rail_b = match (outputs.rail_b, lease_port) {
        (Some(a), Some(b)) if a == b && rail_a => Some(a),
        _ => None,
    };
    Driven { rail_a, rail_b }
}

/// What the rail enables are driven to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Driven {
    /// Rail A enable.
    pub rail_a: bool,
    /// Rail B's port, if enabled.
    pub rail_b: Option<Port>,
}

impl Driven {
    pub(crate) const OFF: Driven = Driven {
        rail_a: false,
        rail_b: None,
    };

    pub(crate) const fn any_on(self) -> bool {
        self.rail_a || self.rail_b.is_some()
    }

    /// What stays on when moving from `self` to `next`: only what is on in
    /// both.
    pub(crate) fn common(self, next: Driven) -> Driven {
        Driven {
            rail_a: self.rail_a && next.rail_a,
            rail_b: if self.rail_b == next.rail_b {
                next.rail_b
            } else {
                None
            },
        }
    }
}

/// The trust root's main loop around one [`Board`].
#[derive(Debug)]
pub struct TrustRoot<B: Board> {
    board: B,
    state: State,
    driven: Driven,
    booted: bool,
    /// The record last written and read back since boot, if any.
    written: Option<FaultRecord>,
    panel_was_pressed: bool,
}

impl<B: Board> TrustRoot<B> {
    /// Takes the board and drives every output off. Nothing else happens
    /// until the first [`TrustRoot::tick`], which is the restart.
    pub fn new(mut board: B) -> Self {
        board.drive_rails(false, None);
        TrustRoot {
            board,
            state: State::POWER_ON,
            driven: Driven::OFF,
            booted: false,
            written: None,
            panel_was_pressed: false,
        }
    }

    /// The interlock state after the last tick.
    #[must_use]
    pub const fn state(&self) -> State {
        self.state
    }

    /// The board, for inspection.
    #[must_use]
    pub const fn board(&self) -> &B {
        &self.board
    }

    /// The board, for a test to change its inputs.
    pub fn board_mut(&mut self) -> &mut B {
        &mut self.board
    }

    /// One pass of the main loop. The caller paces the ticks.
    pub fn tick(&mut self) -> TickReport {
        let pins = self.board.sample();
        let now = self.board.now();
        let heartbeat = self.board.heartbeat_reply();
        let inputs = Inputs {
            estop_pressed: pins.estop_pressed,
            arm_switch_on: pins.arm_switch_on,
            key_switch_on: pins.key_switch_on,
            heartbeat,
            panel_button_pressed: pins.panel_button_pressed,
            rail_a_sensed: pins.rail_a_sensed,
            rail_b_sensed: pins.rail_b_sensed,
            firmware_jumper_fitted: pins.firmware_jumper_fitted,
            now,
        };

        let panel_press = pins.panel_button_pressed && !self.panel_was_pressed;
        self.panel_was_pressed = pins.panel_button_pressed;

        let (event, source) = if !self.booted {
            self.booted = true;
            let record = self
                .board
                .read_record()
                .and_then(|words| record::decode(&words));
            let nonce = self.board.hardware_nonce();
            (Some(Event::Restart { record, nonce }), EventSource::Restart)
        } else if panel_press && matches!(self.state.mode, Mode::Faulted(_)) {
            let event = Event::ClearFault {
                source: ClearSource::Panel(PanelToken::from_panel_button()),
            };
            (Some(event), EventSource::Panel)
        } else {
            match self.board.link_request() {
                Some(request) => (Some(request.into_event()), EventSource::Link),
                None => (None, EventSource::None),
            }
        };

        let (next, outputs) = step(self.state, inputs, event);
        self.state = next;

        let want = permitted(&outputs, &next);

        // Cut first: whatever goes off goes off now, before any write.
        let keep = self.driven.common(want);
        if keep != self.driven {
            self.drive(keep);
        }

        let record = next.fault_record();
        let record_write = if want != keep {
            // Something turns on: the record must be durable first.
            match self.persist(record) {
                RecordWrite::Written(r) => {
                    self.drive(want);
                    RecordWrite::Written(r)
                }
                other => other,
            }
        } else if self.written != Some(record) {
            self.persist(record)
        } else {
            RecordWrite::None
        };

        // The only call site of the charge pump: once per pass, after step
        // returned a rail on and that rail is driven on.
        let pump_toggled = self.driven.any_on();
        if pump_toggled {
            self.board.toggle_charge_pump();
        }

        let report = TickReport {
            outputs,
            rail_a_driven: self.driven.rail_a,
            rail_b_driven: self.driven.rail_b,
            pump_toggled,
            record: record_write,
            event: source,
        };
        self.board.report(&report);
        self.board.feed_watchdog();
        report
    }

    fn drive(&mut self, driven: Driven) {
        self.board.drive_rails(driven.rail_a, driven.rail_b);
        self.driven = driven;
    }

    /// Writes the record and reads it back. A write that does not read back
    /// as the same record counts as failed.
    fn persist(&mut self, record: FaultRecord) -> RecordWrite {
        let words = record::encode(record);
        let ok = self.board.write_record(&words).is_ok() && self.board.read_record() == Some(words);
        if ok {
            self.written = Some(record);
            RecordWrite::Written(record)
        } else {
            self.written = None;
            RecordWrite::Failed(record)
        }
    }
}
