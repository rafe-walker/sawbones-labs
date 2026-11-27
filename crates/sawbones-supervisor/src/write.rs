//! The one write path, and arming behind the isolation check.

use std::collections::BTreeMap;

use sawbones_interlock::{LeaseTarget, Tick};
use sawbones_isolation::{build_plan, Connection, Finding, ModuleDeclaration, PlanProblem};
use sawbones_module_protocol::ModuleId;
use sawbones_policy::{
    confirm, preflight, BusWriteAction, Decision, EffectRequest, PortId, PreflightRefusal,
    PreflightToken, Requirement, SignalId, WriteValue,
};

use crate::io::{BusReply, BusTransport, InterlockLink, ToInterlock};
use crate::latch::{LatchPass, LatchRefusal};
use crate::registry::ModuleState;
use crate::supervisor::{ArmedPlan, Supervisor};

/// An action that passed [`confirm`] at the moment of action, the taint
/// latch and every supervisor gate. Private fields and no constructor
/// outside this crate: only [`Supervisor::invoke`] makes one, and only after
/// consuming a [`PreflightToken`] and a [`LatchPass`], so a [`BusTransport`]
/// cannot be driven around the write path.
///
/// ```compile_fail
/// use sawbones_policy::{Decision, EffectRequest};
/// use sawbones_supervisor::Authorized;
/// // No struct literal: the fields are private.
/// fn forge(request: EffectRequest, decision: Decision) -> Authorized {
///     Authorized { request, decision, bench_port: 0 }
/// }
/// ```
#[derive(Debug)]
pub struct Authorized {
    request: EffectRequest,
    decision: Decision,
    bench_port: u8,
}

impl Authorized {
    /// The request, exactly as preflighted and confirmed.
    pub fn request(&self) -> &EffectRequest {
        &self.request
    }

    /// The policy engine's decision.
    pub fn decision(&self) -> &Decision {
        &self.decision
    }

    /// The bench port the request's port is wired to.
    pub fn bench_port(&self) -> u8 {
        self.bench_port
    }

    /// For a class 3 write, the one bus action the lease approves. The bus
    /// guard passes only the frame it encodes to.
    pub fn bus_action(&self) -> Option<&BusWriteAction> {
        match self.decision.requirement() {
            Requirement::RailBLease { action, .. } => Some(action),
            _ => None,
        }
    }
}

/// A write that went out with no reply: its effect is unknown (#100, card
/// bus-007). Its port takes nothing but a read of the same signal until a
/// read back answers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PendingWrite {
    /// The unit written.
    pub unit: u8,
    /// The signal written.
    pub signal: SignalId,
    /// The value sent.
    pub value: WriteValue,
}

/// What an invoked action did.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// The device acknowledged the write, or the listen started.
    Done,
    /// A read's value: the device's claim.
    Read(WriteValue),
    /// The write went out and nothing came back. The port is blocked until
    /// a read of the same signal answers.
    Unknown,
    /// A read went out and nothing came back. Nothing changes.
    NoReply,
    /// A read of the signal a pending write was for answered. The port is
    /// open again.
    ReadBack {
        /// The value the device reports now: its claim.
        value: WriteValue,
        /// It equals the value the write sent.
        applied: bool,
    },
}

/// Why the supervisor refused to arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArmRefusal {
    /// The connection plan has findings of severity Stop.
    Isolation {
        /// Every Stop finding.
        violations: Vec<Finding>,
    },
    /// The plan's map could not be built from the bench config and the
    /// modules' declarations: something is missing, unconfirmed or in
    /// conflict. An unknown terminal is not a safe one.
    Plan {
        /// Every reason found.
        problems: Vec<PlanProblem>,
    },
}

/// Why an action was not carried out. The gates run in this order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvokeRefusal {
    /// A write on this port went out with no reply, and this is not a read
    /// of the same signal.
    OutcomeUnknown {
        /// The write whose effect is unknown.
        pending: PendingWrite,
    },
    /// No enrolled module carries the port.
    NoCarrier,
    /// The module that carries the port is not available.
    CarrierNotAvailable {
        /// The module.
        module: ModuleId,
    },
    /// [`confirm`] refused the token, or the preflight refused the request.
    Preflight(PreflightRefusal),
    /// The taint latch (#210) refused at the moment of action: the pass is
    /// for another turn or request, or outside content entered the agent's
    /// context and no fresh approval has been seen since.
    Latch(LatchRefusal),
    /// Class 2 or 3 with no connection plan checked clean by
    /// [`Supervisor::arm`].
    NotArmed,
    /// The trust root has sent no report.
    InterlockSilent,
    /// The trust root enforces another rule set than this build was made
    /// for.
    RulesMismatch {
        /// What it reported.
        reported: u64,
    },
    /// The trust root reports a latched fault.
    InterlockFaulted,
    /// The trust root does not report rail A on.
    RailANotLive,
    /// The trust root does not report rail B on for the lease's port.
    LeaseNotLive,
    /// [`Supervisor::request_lease`] for a decision that needs no lease.
    NoLeaseNeeded,
}

impl Supervisor {
    /// Builds the map of the connection plan and checks it before arming.
    /// The map is built here, from the bench config, the declarations of the
    /// modules that are available now, and the proposed `connections`
    /// (galvanic only): a caller cannot pass a map of its own. A terminal no
    /// one declared, or declared and the config does not back, refuses the
    /// plan (fail closed). Any finding of severity Stop refuses; warnings and
    /// notes are kept in the [`ArmedPlan`]. On a clean plan it asks the trust
    /// root for rail A, which only the arm switch can grant.
    ///
    /// # Errors
    ///
    /// [`ArmRefusal::Plan`] if the map cannot be built,
    /// [`ArmRefusal::Isolation`] with every Stop finding.
    pub fn arm(
        &mut self,
        connections: &[Connection],
        interlock: &mut impl InterlockLink,
    ) -> Result<(), ArmRefusal> {
        let vouched: BTreeMap<u32, ModuleDeclaration> = self
            .declarations
            .iter()
            .filter(|(m, _)| self.module_state(**m) == Some(ModuleState::Available))
            .map(|(m, d)| (m.0, d.clone()))
            .collect();
        let map = match build_plan(&self.bench, &vouched, connections) {
            Ok(map) => map,
            Err(refusal) => {
                self.armed = None;
                return Err(ArmRefusal::Plan {
                    problems: refusal.problems,
                });
            }
        };
        let report = map.check();
        let violations: Vec<Finding> = report.violations().cloned().collect();
        if !violations.is_empty() {
            self.armed = None;
            return Err(ArmRefusal::Isolation { violations });
        }
        self.plan_armed(report);
        interlock.send(ToInterlock::RequestArm);
        Ok(())
    }

    /// Drops the armed plan. Class 2 and 3 actions are refused until a plan
    /// is checked again.
    pub fn disarm(&mut self) {
        self.armed = None;
    }

    /// The armed plan, if one passed the isolation check.
    pub fn armed_plan(&self) -> Option<&ArmedPlan> {
        self.armed.as_ref()
    }

    /// The write whose effect is unknown on `port`, if there is one.
    pub fn pending_write(&self, port: &PortId) -> Option<&PendingWrite> {
        self.pending.get(port)
    }

    /// Step one of the write path: checks every fact the request relies on,
    /// then classifies it (`sawbones_policy::preflight`). The token is bound
    /// to the request, its action digest, the fact versions, the ledger and
    /// the chain head.
    ///
    /// # Errors
    ///
    /// [`PreflightRefusal`], naming the fact that does not hold.
    pub fn preflight(
        &self,
        request: &EffectRequest,
        now: Tick,
    ) -> Result<PreflightToken, PreflightRefusal> {
        preflight(
            request,
            &self.writes,
            &self.buses,
            &self.ledger,
            self.chain_head,
            now,
        )
    }

    /// Asks the trust root for the lease a class 3 token's decision names.
    /// Only a panel press grants it. Refused while the port has a write of
    /// unknown effect, or with no armed plan.
    ///
    /// # Errors
    ///
    /// [`InvokeRefusal`].
    pub fn request_lease(
        &mut self,
        token: &PreflightToken,
        interlock: &mut impl InterlockLink,
    ) -> Result<LeaseTarget, InvokeRefusal> {
        let Requirement::RailBLease { port, .. } = token.decision().requirement() else {
            return Err(InvokeRefusal::NoLeaseNeeded);
        };
        if let Some(pending) = self.pending.get(port) {
            return Err(InvokeRefusal::OutcomeUnknown {
                pending: pending.clone(),
            });
        }
        if self.armed.is_none() {
            return Err(InvokeRefusal::NotArmed);
        }
        let target = token
            .decision()
            .requirement()
            .lease_target()
            .ok_or(InvokeRefusal::NoLeaseNeeded)?;
        interlock.send(ToInterlock::RequestLease { target });
        Ok(target)
    }

    /// Step three of the write path, and the only way to carry out an
    /// action: [`confirm`] consumes the token at the moment of action, the
    /// taint latch checks the pass again, then the supervisor's gates run,
    /// then the bus transport gets an [`Authorized`] action. There is no way
    /// to call it without a token and a latch pass, no way to make a token
    /// but [`Supervisor::preflight`], and no way to make a pass but
    /// [`Supervisor::latch_check`] (see [`LatchPass`]):
    ///
    /// ```compile_fail
    /// use sawbones_policy::{EffectRequest, PreflightToken};
    /// use sawbones_supervisor::{BusTransport, InterlockLink, LatchPass, Supervisor};
    /// fn skip(
    ///     s: &mut Supervisor,
    ///     r: &EffectRequest,
    ///     pass: LatchPass,
    ///     bus: &mut impl BusTransport,
    ///     interlock: &mut impl InterlockLink,
    /// ) {
    ///     // A token cannot be written down outside the policy crate.
    ///     let token: PreflightToken = PreflightToken { checked_at: 0 };
    ///     let _ = s.invoke(r, token, pass, 0, bus, interlock);
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// use sawbones_policy::{EffectRequest, PreflightToken};
    /// use sawbones_supervisor::{BusTransport, InterlockLink, Supervisor};
    /// fn skip_latch(
    ///     s: &mut Supervisor,
    ///     r: &EffectRequest,
    ///     token: PreflightToken,
    ///     bus: &mut impl BusTransport,
    ///     interlock: &mut impl InterlockLink,
    /// ) {
    ///     // The latch pass is not optional.
    ///     let _ = s.invoke(r, token, 0, bus, interlock);
    /// }
    /// ```
    ///
    /// The gates, in order: a write of unknown effect on the port blocks
    /// everything but a read of its signal; the port's carrier module must
    /// be available; the token must confirm; the pass must be for the open
    /// turn and this request, and the latch must allow the confirmed class
    /// now; class 2 and 3 need an armed
    /// plan and a trust-root report with this build's rule set, no fault and
    /// rail A on; class 3 also needs rail B on for the lease's port. A class
    /// 3 action is reported to the trust root as used whatever the reply.
    /// A write with no reply blocks its port until a read back.
    ///
    /// # Errors
    ///
    /// [`InvokeRefusal`].
    pub fn invoke(
        &mut self,
        request: &EffectRequest,
        token: PreflightToken,
        pass: LatchPass,
        now: Tick,
        bus: &mut impl BusTransport,
        interlock: &mut impl InterlockLink,
    ) -> Result<Outcome, InvokeRefusal> {
        let port = request.port().clone();
        let readback = match (self.pending.get(&port), request) {
            (None, _) => false,
            (Some(p), EffectRequest::BusRead(r)) if r.unit == p.unit && r.signal == p.signal => {
                true
            }
            (Some(p), _) => {
                return Err(InvokeRefusal::OutcomeUnknown { pending: p.clone() });
            }
        };
        match self.carrier(&port) {
            None => return Err(InvokeRefusal::NoCarrier),
            Some((module, false)) => return Err(InvokeRefusal::CarrierNotAvailable { module }),
            Some((_, true)) => {}
        }
        let confirmed = confirm(
            token,
            request,
            &self.writes,
            &self.buses,
            &self.ledger,
            self.chain_head,
            now,
        )
        .map_err(InvokeRefusal::Preflight)?;
        let decision = confirmed.decision().clone();
        let lease = confirmed.lease_target();
        let grounds = self
            .latch
            .recheck(&pass, request, &decision)
            .map_err(InvokeRefusal::Latch)?;
        match decision.requirement() {
            Requirement::Nothing => {}
            Requirement::RailA { .. } => self.rails_live(None, interlock)?,
            Requirement::RailBLease { .. } => match lease {
                Some(target) => self.rails_live(Some(target), interlock)?,
                None => return Err(InvokeRefusal::LeaseNotLive),
            },
            // confirm never passes a refused decision; refuse rather than
            // assume.
            Requirement::Refused { reason } => {
                return Err(InvokeRefusal::Preflight(PreflightRefusal::Classified {
                    reason: *reason,
                }))
            }
        }
        let bench_port = self
            .buses
            .get(&port)
            .map(|b| b.bench_port)
            .ok_or(InvokeRefusal::NoCarrier)?;
        let action = Authorized {
            request: request.clone(),
            decision,
            bench_port,
        };
        let reply = bus.perform(&action);
        if let Some(target) = lease {
            interlock.send(ToInterlock::LeaseUsed { target });
        }
        self.latch.spend(grounds);
        Ok(match (request, reply) {
            (EffectRequest::BusWrite(w), BusReply::NoReply) => {
                self.pending.insert(
                    port,
                    PendingWrite {
                        unit: w.unit,
                        signal: w.signal.clone(),
                        value: w.value,
                    },
                );
                Outcome::Unknown
            }
            (EffectRequest::BusRead(_), BusReply::Value(value)) if readback => {
                let applied = self.pending.remove(&port).map(|p| p.value) == Some(value);
                Outcome::ReadBack { value, applied }
            }
            (_, BusReply::Value(value)) => Outcome::Read(value),
            (_, BusReply::Done) => Outcome::Done,
            (_, BusReply::NoReply) => Outcome::NoReply,
        })
    }

    fn rails_live(
        &self,
        lease: Option<LeaseTarget>,
        interlock: &impl InterlockLink,
    ) -> Result<(), InvokeRefusal> {
        if self.armed.is_none() {
            return Err(InvokeRefusal::NotArmed);
        }
        let report = interlock.report().ok_or(InvokeRefusal::InterlockSilent)?;
        if report.rules_id != sawbones_interlock::rules_id() {
            return Err(InvokeRefusal::RulesMismatch {
                reported: report.rules_id,
            });
        }
        if report.faulted {
            return Err(InvokeRefusal::InterlockFaulted);
        }
        if !report.rail_a {
            return Err(InvokeRefusal::RailANotLive);
        }
        if let Some(target) = lease {
            if report.rail_b != Some(target.port) {
                return Err(InvokeRefusal::LeaseNotLive);
            }
        }
        Ok(())
    }
}
