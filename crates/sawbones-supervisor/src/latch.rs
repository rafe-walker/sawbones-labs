//! The taint latch (#210): once outside content has entered the agent's
//! context, actions of hazard class 2 and higher wait for a fresh approval.
//!
//! # The turn contract for the MCP layer
//!
//! There is no MCP server yet. The latch models the boundary it will call:
//!
//! 1. [`Supervisor::begin_turn`] when the agent's turn starts: a client
//!    request arrives, before any tool result of that turn reaches the
//!    model. It returns the [`TurnId`] every action of the turn names. It
//!    ends any turn still open.
//! 2. [`Supervisor::admit`] (or [`Supervisor::taint`] for kinds of content
//!    with no type here yet) for **every** piece of content that is not
//!    host-authored, at the moment it is put into a tool result, **before**
//!    the result is returned to the model. Host-authored content is text
//!    this project's own code wrote: tool definitions, enum codes, typed
//!    refusals, host-written explanations. Everything else is outside
//!    content, whatever it looks like. The one outside input that does not
//!    trip the latch is a [`MeasuredNumber`]: the bare number of a reading
//!    at [`Trust::Measured`], without its names, unit text or provenance.
//! 3. Every action goes through [`Supervisor::preflight`], then
//!    [`Supervisor::latch_check`] naming the open turn, then
//!    [`Supervisor::invoke`], which takes the [`LatchPass`] by value and
//!    checks the latch again at the moment of action.
//! 4. [`Supervisor::end_turn`] when the turn's final answer has gone out.
//!
//! Reporting a taint is never refused: with no turn open, or with a turn
//! that has ended, it still trips the latch.
//!
//! # What trips it
//!
//! Any [`TurnInput`] other than [`TurnInput::Measured`], and any
//! [`Origin`] passed to [`Supervisor::taint`]: text from a device, module,
//! broker or bus (names, topics, payloads, descriptors, error strings);
//! plugin and decoder output (observations have no tier, and their names
//! and provenance are outside text); files; pages; images; transcribed
//! speech; hand-off notes and store cards an agent wrote.
//!
//! The latch also **starts tripped** ([`TaintSource::SupervisorStart`]),
//! as the interlock starts with its fault state unknown: a supervisor that
//! has just started cannot know what the agent's context already holds, and
//! a restart must not be a way to clear it.
//!
//! # What it blocks, and what clears it
//!
//! While tripped, classes 0 and 1 pass (they need nothing anyway) and
//! everything else is refused with [`LatchRefusal::Tainted`], which names
//! the latest taint's source and turn and the approval that would lift it:
//!
//! - **Class 2: an arm cycle after the taint** ([`FreshApproval::ArmCycle`]).
//!   The supervisor must see, in the trust root's reports read by
//!   [`Supervisor::step`] after the latest taint, rail A off and then rail A
//!   on. Rail A comes on only by the arm switch, and every way from off back
//!   to on needs the switch seen off and turned on again, so this is a
//!   person's hand after the moment the supervisor saw the rail off. It
//!   clears the latch. Arming already dropped any lease, so no approval from
//!   before the taint survives it. A switch flipped off and on between two
//!   reads is missed, and the latch stays tripped: it fails closed.
//! - **Class 3: a lease granted after the taint**
//!   ([`FreshApproval::LeaseAfterTaint`]). The lease must postdate the
//!   taint: the supervisor must see rail B not live for the lease's port in
//!   a report after the latest taint, and then live. A lease granted before
//!   the taint, still live, does not count. A class 3 action spends its
//!   lease, so the next one needs another. It does not clear the latch for
//!   class 2.
//! - **Class 4** is never permitted ([`FreshApproval::NotPermitted`]).
//!
//! A new taint resets both: any approval seen before the latest taint does
//! not count.
//!
//! **Nothing else clears it.** There is no clear method. Turns, sessions,
//! [`Supervisor::arm`] (which only asks), [`Supervisor::disarm`], lease
//! requests, fact events and any content admitted to the turn leave it as
//! it is. A report from a trust root with another rule set is not read.
//!
//! # Why a new turn does not start clear
//!
//! A turn is not a fresh context. The model still holds the text of earlier
//! turns, and clients resume old conversations, so the supervisor cannot
//! tell from a turn or a session boundary that the text is gone. A clear
//! latch stays clear across turns until something trips it, and a tripped
//! latch stays tripped across turns until a fresh approval.
//!
//! # Residual gaps
//!
//! The latch is only as complete as the MCP layer's reports (#103). The
//! interlock report carries no sequence number or trust-root tick, so a
//! report delayed across a taint could make an arm from before the taint
//! look fresh; signed trust-root events (#172) will carry a counter. The
//! supervisor sees rail B by port, not by action: the interlock and the bus
//! guard bind a lease to its action digest.

use std::num::NonZeroU64;

use sawbones_interlock::{Port, RAIL_B_PORTS};
use sawbones_policy::{Decision, EffectRequest, HazardClass, Requirement};
use sawbones_reading::{DecimalValue, Observation, Reading, Trust, WorldText};

use crate::io::{InterlockLink, InterlockReport};
use crate::supervisor::Supervisor;

/// One turn of the agent, as [`Supervisor::begin_turn`] opened it.
///
/// [`Supervisor::begin_turn`]: crate::Supervisor::begin_turn
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TurnId(NonZeroU64);

impl TurnId {
    /// The turn's number, counted from 1 since the supervisor started.
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// Where outside content came from. It holds no text from the content, so a
/// refusal that names it carries none either.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// A device, module, broker or bus: names, topics, payloads, discovery
    /// fields, descriptor strings, error text, and any value a device
    /// reports about itself.
    Device,
    /// A plugin or decoder: its observations and any text the plugin host
    /// kept from it (plugin output is untrusted).
    Plugin,
    /// A file.
    File,
    /// A web page or any other fetched document.
    Page,
    /// An image: a camera frame, a screenshot, OCR or a vision read.
    Image,
    /// Transcribed speech or other audio.
    Speech,
    /// A hand-off note or store card written by an agent.
    Note,
}

/// Why the latch is tripped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaintSource {
    /// The supervisor started. Nothing says what the agent's context already
    /// holds.
    SupervisorStart,
    /// Outside content entered the agent's context.
    Input(Origin),
}

/// One taint, as the latch recorded it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Taint {
    /// Where it came from.
    pub source: TaintSource,
    /// The turn open when it was reported, if one was.
    pub turn: Option<TurnId>,
}

/// The number of a measured reading and nothing else: no source name, no
/// quantity name, no unit text, no provenance. The one input the MCP layer
/// may give the model without tripping the latch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasuredNumber {
    value: DecimalValue,
}

impl MeasuredNumber {
    /// The number of `reading`, if it is at [`Trust::Measured`]: measured by
    /// an instrument this project runs. `None` for every other tier.
    pub fn of(reading: &Reading) -> Option<MeasuredNumber> {
        (reading.trust() == Trust::Measured).then(|| MeasuredNumber {
            value: reading.value().clone(),
        })
    }

    /// The number, as a validated plain decimal.
    pub fn value(&self) -> &DecimalValue {
        &self.value
    }
}

/// Something the MCP layer is about to put into the agent's context.
#[derive(Debug, Clone, Copy)]
pub enum TurnInput<'a> {
    /// A string from outside this project's code. Trips the latch.
    Text {
        /// The text.
        text: &'a WorldText,
        /// Where it came from.
        origin: Origin,
    },
    /// An observation from a decoder or plugin. It has no tier, and its
    /// names and provenance are outside text. Trips the latch.
    Observation {
        /// The observation.
        observation: &'a Observation,
        /// Where it came from.
        origin: Origin,
    },
    /// A whole reading: its names, unit, provenance (topic, raw payload) and
    /// tier. Trips the latch, whatever its tier.
    Reading {
        /// The reading.
        reading: &'a Reading,
        /// Where it came from.
        origin: Origin,
    },
    /// The bare number of a measured reading. Does not trip the latch.
    Measured(&'a MeasuredNumber),
}

impl TurnInput<'_> {
    /// The origin this input trips the latch with, or `None` for a
    /// [`MeasuredNumber`].
    pub const fn taints(&self) -> Option<Origin> {
        match *self {
            TurnInput::Text { origin, .. }
            | TurnInput::Observation { origin, .. }
            | TurnInput::Reading { origin, .. } => Some(origin),
            TurnInput::Measured(_) => None,
        }
    }
}

/// The approval that lifts a [`LatchRefusal::Tainted`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FreshApproval {
    /// Class 2: the arm switch off and on again, seen after the latest
    /// taint.
    ArmCycle,
    /// Class 3: a lease for this rail B port granted after the latest
    /// taint, by a panel press.
    LeaseAfterTaint {
        /// The rail B port the action's lease is for.
        port: Port,
    },
    /// Class 4: nothing.
    NotPermitted,
}

/// Why the latch refused an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LatchRefusal {
    /// No turn is open: an action is only taken inside a turn, after every
    /// input of that turn has been reported.
    NoOpenTurn,
    /// The turn named is not the open one.
    NotTheOpenTurn {
        /// The turn named.
        given: TurnId,
        /// The turn open now.
        open: TurnId,
    },
    /// The pass was made for another request.
    OtherRequest,
    /// Outside content entered the agent's context and no fresh approval
    /// has been seen since.
    Tainted {
        /// The action's hazard class.
        class: HazardClass,
        /// The latest taint, which the approval must postdate.
        taint: Taint,
        /// How many taints since the latch was last clear, the latest
        /// included.
        taints: u32,
        /// What would lift the refusal.
        needs: FreshApproval,
    },
}

/// What a [`LatchPass`] was granted on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Grounds {
    /// The latch was clear.
    Clear,
    /// The latch was tripped and the action is class 0 or 1.
    BelowClass2,
    /// The latch was tripped and the class 3 action has a lease granted
    /// after the latest taint.
    LeaseAfterTaint {
        /// The rail B port.
        port: Port,
    },
}

/// The proof that [`Supervisor::latch_check`] let one request through in one
/// open turn. Private fields, not `Clone`, no serde: only `latch_check`
/// makes one, and [`Supervisor::invoke`] consumes it and checks the latch
/// again at the moment of action.
///
/// ```compile_fail
/// use sawbones_policy::{EffectRequest, PreflightToken};
/// use sawbones_supervisor::{BusTransport, Grounds, InterlockLink, LatchPass, Supervisor};
/// fn skip(
///     s: &mut Supervisor,
///     r: &EffectRequest,
///     token: PreflightToken,
///     bus: &mut impl BusTransport,
///     interlock: &mut impl InterlockLink,
/// ) {
///     // A pass cannot be written down outside this crate. Every field is
///     // named, so the only reason this fails is that the fields are private.
///     let turn = s.begin_turn();
///     let pass = LatchPass { turn, request: Vec::new(), grounds: Grounds::Clear };
///     let _ = s.invoke(r, token, pass, 0, bus, interlock);
/// }
/// ```
///
/// ```compile_fail
/// use sawbones_supervisor::LatchPass;
/// // One pass, one action: it cannot be copied to spend twice.
/// fn twice(pass: LatchPass) -> (LatchPass, LatchPass) {
///     (pass.clone(), pass)
/// }
/// ```
///
/// [`Supervisor::latch_check`]: crate::Supervisor::latch_check
/// [`Supervisor::invoke`]: crate::Supervisor::invoke
#[derive(Debug)]
#[must_use = "a latch pass is presented to invoke"]
pub struct LatchPass {
    pub(crate) turn: TurnId,
    pub(crate) request: Vec<u8>,
    pub(crate) grounds: Grounds,
}

impl LatchPass {
    /// The turn it was granted in.
    pub const fn turn(&self) -> TurnId {
        self.turn
    }

    /// What it was granted on.
    pub const fn grounds(&self) -> Grounds {
        self.grounds
    }
}

/// What the latch has seen since the latest taint.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Tripped {
    latest: Taint,
    taints: u32,
    /// Rail A reported off since the latest taint.
    rail_a_seen_off: bool,
    /// Rail B reported not live for each port since the latest taint.
    rail_b_seen_off: [bool; RAIL_B_PORTS],
    /// The port whose rail B was seen go live after being seen off, since
    /// the latest taint, with its lease not yet spent.
    fresh_lease: Option<Port>,
}

/// The latch and the turn bookkeeping. Lives in the [`crate::Supervisor`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Latch {
    open: Option<TurnId>,
    next: NonZeroU64,
    tripped: Option<Tripped>,
}

impl Latch {
    /// Tripped, with [`TaintSource::SupervisorStart`].
    pub(crate) fn at_start() -> Self {
        let mut latch = Latch {
            open: None,
            next: NonZeroU64::MIN,
            tripped: None,
        };
        latch.trip(TaintSource::SupervisorStart);
        latch
    }

    pub(crate) fn begin_turn(&mut self) -> TurnId {
        let turn = TurnId(self.next);
        self.next = self.next.saturating_add(1);
        self.open = Some(turn);
        turn
    }

    pub(crate) fn end_turn(&mut self, turn: TurnId) {
        if self.open == Some(turn) {
            self.open = None;
        }
    }

    pub(crate) const fn open_turn(&self) -> Option<TurnId> {
        self.open
    }

    pub(crate) fn tainted_by(&self) -> Option<Taint> {
        self.tripped.as_ref().map(|t| t.latest)
    }

    /// Trips the latch, or trips it again: every approval seen before this
    /// taint stops counting.
    pub(crate) fn trip(&mut self, source: TaintSource) {
        let taints = self
            .tripped
            .as_ref()
            .map_or(0, |t| t.taints)
            .saturating_add(1);
        self.tripped = Some(Tripped {
            latest: Taint {
                source,
                turn: self.open,
            },
            taints,
            rail_a_seen_off: false,
            rail_b_seen_off: [false; RAIL_B_PORTS],
            fresh_lease: None,
        });
    }

    /// Takes in one trust-root report, read by the main loop. The only way
    /// the latch learns of an approval.
    pub(crate) fn observe(&mut self, report: Option<InterlockReport>, rules_id: u64) {
        let (Some(tripped), Some(report)) = (self.tripped.as_mut(), report) else {
            return;
        };
        if report.rules_id != rules_id {
            return;
        }
        if !report.rail_a {
            tripped.rail_a_seen_off = true;
        } else if tripped.rail_a_seen_off {
            // Off, then on: the arm switch, after the latest taint.
            self.tripped = None;
            return;
        }
        for (index, seen_off) in tripped.rail_b_seen_off.iter_mut().enumerate() {
            let live = report
                .rail_b
                .is_some_and(|p| usize::from(p.0) == index && report.rail_a);
            if !live {
                *seen_off = true;
                if tripped
                    .fresh_lease
                    .is_some_and(|p| usize::from(p.0) == index)
                {
                    tripped.fresh_lease = None;
                }
            } else if *seen_off {
                tripped.fresh_lease = report.rail_b;
            }
        }
    }

    /// What the latch allows for an action of `class` whose lease, for class
    /// 3, is on `lease_port`.
    fn allows(
        &self,
        class: HazardClass,
        lease_port: Option<Port>,
    ) -> Result<Grounds, LatchRefusal> {
        let Some(tripped) = &self.tripped else {
            return Ok(Grounds::Clear);
        };
        let refuse = |needs| {
            Err(LatchRefusal::Tainted {
                class,
                taint: tripped.latest,
                taints: tripped.taints,
                needs,
            })
        };
        match (class, lease_port) {
            (HazardClass::Class0 | HazardClass::Class1, _) => Ok(Grounds::BelowClass2),
            (HazardClass::Class2, _) => refuse(FreshApproval::ArmCycle),
            (HazardClass::Class3, Some(port)) if tripped.fresh_lease == Some(port) => {
                Ok(Grounds::LeaseAfterTaint { port })
            }
            (HazardClass::Class3, Some(port)) => refuse(FreshApproval::LeaseAfterTaint { port }),
            // A class 3 decision always names its lease; refuse rather than
            // assume.
            (HazardClass::Class3, None) | (HazardClass::Class4, _) => {
                refuse(FreshApproval::NotPermitted)
            }
        }
    }

    /// The latch's answer for a decision.
    pub(crate) fn allows_decision(&self, decision: &Decision) -> Result<Grounds, LatchRefusal> {
        let lease_port = match decision.requirement() {
            Requirement::RailBLease { .. } => decision.requirement().lease_target().map(|t| t.port),
            _ => None,
        };
        self.allows(decision.class(), lease_port)
    }

    /// The check [`crate::Supervisor::latch_check`] runs.
    pub(crate) fn check(
        &self,
        turn: TurnId,
        request: &EffectRequest,
        decision: &Decision,
    ) -> Result<LatchPass, LatchRefusal> {
        self.check_turn(turn)?;
        let grounds = self.allows_decision(decision)?;
        Ok(LatchPass {
            turn,
            request: request.canonical_bytes(),
            grounds,
        })
    }

    /// The check at the moment of action: the pass is for this request and
    /// the open turn, and the latch allows the confirmed decision now.
    pub(crate) fn recheck(
        &self,
        pass: &LatchPass,
        request: &EffectRequest,
        decision: &Decision,
    ) -> Result<Grounds, LatchRefusal> {
        self.check_turn(pass.turn)?;
        if pass.request != request.canonical_bytes() {
            return Err(LatchRefusal::OtherRequest);
        }
        self.allows_decision(decision)
    }

    fn check_turn(&self, turn: TurnId) -> Result<(), LatchRefusal> {
        match self.open {
            None => Err(LatchRefusal::NoOpenTurn),
            Some(open) if open != turn => Err(LatchRefusal::NotTheOpenTurn { given: turn, open }),
            Some(_) => Ok(()),
        }
    }

    /// A class 3 action went out on a lease granted after the taint: the
    /// lease is spent, and the next class 3 action needs another.
    pub(crate) fn spend(&mut self, grounds: Grounds) {
        let (Grounds::LeaseAfterTaint { port }, Some(tripped)) = (grounds, self.tripped.as_mut())
        else {
            return;
        };
        if tripped.fresh_lease == Some(port) {
            tripped.fresh_lease = None;
        }
        if let Some(seen_off) = tripped.rail_b_seen_off.get_mut(usize::from(port.0)) {
            *seen_off = false;
        }
    }
}

impl Supervisor {
    /// Opens the agent's next turn and returns its id. Any turn still open
    /// ends. The latch is not changed: a new turn inherits it (see the
    /// module docs on why a turn is not a fresh context).
    pub fn begin_turn(&mut self) -> TurnId {
        self.latch.begin_turn()
    }

    /// Ends `turn`, if it is the open one. The latch is not changed.
    pub fn end_turn(&mut self, turn: TurnId) {
        self.latch.end_turn(turn);
    }

    /// The open turn, if any.
    pub const fn open_turn(&self) -> Option<TurnId> {
        self.latch.open_turn()
    }

    /// The latest taint, if the latch is tripped.
    pub fn tainted_by(&self) -> Option<Taint> {
        self.latch.tainted_by()
    }

    /// Reports content the MCP layer is about to put into the agent's
    /// context. Every input but a [`MeasuredNumber`] trips the latch; the
    /// origin it tripped with is returned. Never refused.
    pub fn admit(&mut self, input: TurnInput<'_>) -> Option<Origin> {
        let origin = input.taints()?;
        self.taint(origin);
        Some(origin)
    }

    /// Trips the latch for outside content of `origin`, for kinds of
    /// content with no type here yet (files, pages, images, speech, notes).
    /// Never refused, with or without an open turn. Every approval seen
    /// before it stops counting.
    pub fn taint(&mut self, origin: Origin) {
        self.latch.trip(TaintSource::Input(origin));
    }

    /// The latch check, step two of the write path: between
    /// [`Supervisor::preflight`] and [`Supervisor::invoke`]. `turn` must be
    /// the open turn. While the latch is tripped, a class 2 action is
    /// refused until an arm cycle is seen after the latest taint, and a
    /// class 3 action until a lease for its port is seen granted after it;
    /// classes 0 and 1 pass. The pass is bound to the turn and the request,
    /// and [`Supervisor::invoke`] checks the latch again.
    ///
    /// # Errors
    ///
    /// [`LatchRefusal`], naming the latest taint.
    pub fn latch_check(
        &self,
        turn: TurnId,
        request: &EffectRequest,
        token: &sawbones_policy::PreflightToken,
    ) -> Result<LatchPass, LatchRefusal> {
        self.latch.check(turn, request, token.decision())
    }

    /// The main loop's report to the latch.
    pub(crate) fn observe_latch(&mut self, interlock: &impl InterlockLink) {
        self.latch
            .observe(interlock.report(), sawbones_interlock::rules_id());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULES: u64 = 7;

    fn report(rail_a: bool, rail_b: Option<u8>) -> Option<InterlockReport> {
        Some(InterlockReport {
            rules_id: RULES,
            rail_a,
            rail_b: rail_b.map(Port),
            faulted: false,
        })
    }

    fn clear() -> Latch {
        let mut latch = Latch::at_start();
        latch.observe(report(false, None), RULES);
        latch.observe(report(true, None), RULES);
        assert_eq!(latch.tainted_by(), None);
        latch
    }

    const ALL: [HazardClass; 5] = [
        HazardClass::Class0,
        HazardClass::Class1,
        HazardClass::Class2,
        HazardClass::Class3,
        HazardClass::Class4,
    ];

    #[test]
    fn a_clear_latch_allows_every_class() {
        let latch = clear();
        for class in ALL {
            assert_eq!(latch.allows(class, Some(Port(0))), Ok(Grounds::Clear));
        }
    }

    #[test]
    fn a_tripped_latch_allows_class_0_and_1_only() {
        let mut latch = clear();
        latch.trip(TaintSource::Input(Origin::Device));
        for class in [HazardClass::Class0, HazardClass::Class1] {
            assert_eq!(latch.allows(class, None), Ok(Grounds::BelowClass2));
        }
        let needs = |class, port| match latch.allows(class, port) {
            Err(LatchRefusal::Tainted { needs, .. }) => needs,
            other => panic!("{other:?}"),
        };
        assert_eq!(needs(HazardClass::Class2, None), FreshApproval::ArmCycle);
        assert_eq!(
            needs(HazardClass::Class3, Some(Port(1))),
            FreshApproval::LeaseAfterTaint { port: Port(1) }
        );
        assert_eq!(
            needs(HazardClass::Class3, None),
            FreshApproval::NotPermitted
        );
        assert_eq!(
            needs(HazardClass::Class4, None),
            FreshApproval::NotPermitted
        );
    }

    #[test]
    fn it_starts_tripped() {
        let latch = Latch::at_start();
        assert_eq!(
            latch.tainted_by(),
            Some(Taint {
                source: TaintSource::SupervisorStart,
                turn: None
            })
        );
    }

    #[test]
    fn a_report_from_other_rules_is_not_read() {
        let mut latch = Latch::at_start();
        latch.observe(report(false, None), RULES + 1);
        latch.observe(report(true, None), RULES);
        assert!(latch.tainted_by().is_some());
        latch.observe(report(true, None), RULES + 1);
        assert!(latch.tainted_by().is_some());
    }

    #[test]
    fn only_off_then_on_after_the_latest_taint_clears() {
        let mut latch = clear();
        latch.trip(TaintSource::Input(Origin::Page));
        latch.observe(report(true, None), RULES);
        assert!(latch.tainted_by().is_some(), "on without off");
        latch.observe(report(false, None), RULES);
        // A second taint between the off and the on.
        latch.trip(TaintSource::Input(Origin::File));
        latch.observe(report(true, None), RULES);
        assert!(latch.tainted_by().is_some(), "the off predates the taint");
        latch.observe(report(false, None), RULES);
        latch.observe(report(true, None), RULES);
        assert_eq!(latch.tainted_by(), None);
    }

    #[test]
    fn a_lease_counts_only_if_seen_off_after_the_taint_and_is_spent_once() {
        let mut latch = clear();
        // A lease live before the taint.
        latch.observe(report(true, Some(2)), RULES);
        latch.trip(TaintSource::Input(Origin::Plugin));
        latch.observe(report(true, Some(2)), RULES);
        assert!(latch.allows(HazardClass::Class3, Some(Port(2))).is_err());
        // It ends, and a new one is granted.
        latch.observe(report(true, None), RULES);
        latch.observe(report(true, Some(2)), RULES);
        let grounds = latch.allows(HazardClass::Class3, Some(Port(2)));
        assert_eq!(grounds, Ok(Grounds::LeaseAfterTaint { port: Port(2) }));
        // Not for another port, and not for class 2.
        assert!(latch.allows(HazardClass::Class3, Some(Port(1))).is_err());
        assert!(latch.allows(HazardClass::Class2, None).is_err());
        latch.spend(Grounds::LeaseAfterTaint { port: Port(2) });
        latch.observe(report(true, Some(2)), RULES);
        assert!(latch.allows(HazardClass::Class3, Some(Port(2))).is_err());
    }

    #[test]
    fn turns_are_numbered_and_checked() {
        let mut latch = clear();
        let decision_free = |l: &Latch, t| l.check_turn(t);
        let first = latch.begin_turn();
        let second = latch.begin_turn();
        assert_eq!((first.get(), second.get()), (1, 2));
        assert_eq!(
            decision_free(&latch, first),
            Err(LatchRefusal::NotTheOpenTurn {
                given: first,
                open: second
            })
        );
        latch.end_turn(first);
        assert_eq!(latch.open_turn(), Some(second));
        latch.end_turn(second);
        assert_eq!(decision_free(&latch, second), Err(LatchRefusal::NoOpenTurn));
    }
}
