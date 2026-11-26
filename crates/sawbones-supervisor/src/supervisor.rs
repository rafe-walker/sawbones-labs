//! The supervisor's state and its main loop.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use sawbones_interlock::{HeartbeatChallenge, Tick};
use sawbones_isolation::Report;
use sawbones_module_protocol::{
    Challenge, HostLink, IdentityClaim, ImageDigest, ModuleId, Request, Response, MAX_FRAME_LEN,
};
use sawbones_policy::{
    Applied, BusFacts, ChainHead, EventRefusal, FactEvent, FactLedger, FactRecord, FactVersion,
    LedgerId, PortId, RecordError, ScopeLabel, WriteMap,
};

use crate::io::{ChallengeSource, InterlockLink, ModuleTransport};
use crate::registry::{
    ConfigError, Enrollment, Happened, HubPort, ModuleState, PortRefusal, PortState,
    QuarantineReason,
};
use crate::write::PendingWrite;

/// How many ticks a module has to answer a request before it is lost. A
/// placeholder, like the interlock's constants: the real figure follows
/// from the tick period and the link.
pub const LINK_TIMEOUT_TICKS: Tick = 5;

/// How many ticks apart the supervisor polls a verified module's health, and
/// how long it waits before identifying a lost device again. A placeholder.
pub const POLL_PERIOD_TICKS: Tick = 10;

// A module must have time to answer one poll before the next is due.
const _: () = assert!(LINK_TIMEOUT_TICKS >= 1 && LINK_TIMEOUT_TICKS < POLL_PERIOD_TICKS);

/// What the supervisor starts from.
#[derive(Debug)]
pub struct Config {
    /// The fact ledger's id, for example a boot nonce.
    pub ledger: LedgerId,
    /// Every enrolled module.
    pub enrollments: Vec<Enrollment>,
    /// The write-map facts.
    pub writes: WriteMap,
    /// The bus facts.
    pub buses: BusFacts,
}

#[derive(Debug)]
struct Slot {
    link: HostLink,
    state: PortState,
    challenge: Option<Challenge>,
    /// The tick by which the outstanding request must be answered.
    deadline: Option<Tick>,
    /// The tick of the next health poll, or of the next identify after a
    /// loss.
    next_poll: Tick,
}

impl Slot {
    fn new(now: Tick) -> Self {
        Slot {
            link: HostLink::new(),
            state: PortState::Opening,
            challenge: None,
            deadline: None,
            next_poll: now,
        }
    }
}

/// A connection plan that passed the isolation check, held from
/// [`Supervisor::arm`] until [`Supervisor::disarm`] or the trust root
/// reports a fault or rail A going off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmedPlan {
    /// The checker's report: warnings and notes, no violations.
    pub report: Report,
    rail_a_seen: bool,
}

/// The host-side supervisor: the module registry, link deadlines, the
/// heartbeat, and the one write path. See the crate docs.
pub struct Supervisor {
    pub(crate) enrollments: BTreeMap<ModuleId, Enrollment>,
    quarantine: BTreeMap<ModuleId, QuarantineReason>,
    firmware: BTreeMap<ModuleId, ImageDigest>,
    slots: BTreeMap<HubPort, Slot>,
    challenges: Box<dyn ChallengeSource>,
    answered: Option<HeartbeatChallenge>,
    pub(crate) writes: WriteMap,
    pub(crate) buses: BusFacts,
    pub(crate) ledger: FactLedger,
    pub(crate) chain_head: ChainHead,
    pub(crate) armed: Option<ArmedPlan>,
    pub(crate) pending: BTreeMap<PortId, PendingWrite>,
}

impl std::fmt::Debug for Supervisor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Supervisor")
            .field("enrollments", &self.enrollments.len())
            .field("quarantine", &self.quarantine)
            .field("slots", &self.slots.len())
            .field("armed", &self.armed.is_some())
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

/// The firmware digest as a ledger label: 64 lower-case hex digits.
pub(crate) fn firmware_label(digest: &ImageDigest) -> ScopeLabel {
    let mut hex = String::with_capacity(64);
    for b in digest.0 {
        let _ = write!(hex, "{b:02x}");
    }
    // 64 hex digits always pass the label rule.
    ScopeLabel::parse(&hex).unwrap_or_else(|_| unreachable!("hex is a valid label"))
}

impl Supervisor {
    /// A supervisor with nothing seen yet.
    ///
    /// # Errors
    ///
    /// [`ConfigError`] for two enrollments of one id, two modules on one hub
    /// port, or one bus port carried by two modules.
    pub fn new(config: Config, challenges: Box<dyn ChallengeSource>) -> Result<Self, ConfigError> {
        let mut enrollments = BTreeMap::new();
        let mut hub_ports = BTreeSet::new();
        let mut carried = BTreeSet::new();
        for e in config.enrollments {
            if !hub_ports.insert(e.hub_port) {
                return Err(ConfigError::DuplicateHubPort(e.hub_port));
            }
            for p in &e.carries {
                if !carried.insert(p.clone()) {
                    return Err(ConfigError::SharedBusPort(p.clone()));
                }
            }
            if enrollments.contains_key(&e.module) {
                return Err(ConfigError::DuplicateModule(e.module));
            }
            enrollments.insert(e.module, e);
        }
        Ok(Supervisor {
            enrollments,
            quarantine: BTreeMap::new(),
            firmware: BTreeMap::new(),
            slots: BTreeMap::new(),
            challenges,
            answered: None,
            writes: config.writes,
            buses: config.buses,
            ledger: FactLedger::new(config.ledger),
            chain_head: ChainHead([0; 32]),
            armed: None,
            pending: BTreeMap::new(),
        })
    }

    /// The fact ledger.
    pub fn ledger(&self) -> &FactLedger {
        &self.ledger
    }

    /// Records a fact in the ledger.
    ///
    /// # Errors
    ///
    /// The ledger's [`RecordError`].
    pub fn record_fact(
        &mut self,
        record: FactRecord,
        now: Tick,
    ) -> Result<FactVersion, RecordError> {
        self.ledger.record(record, now)
    }

    /// Applies an event to the ledger.
    ///
    /// # Errors
    ///
    /// The ledger's [`EventRefusal`], only for an event that raises trust.
    pub fn apply_fact_event(
        &mut self,
        event: FactEvent,
        now: Tick,
    ) -> Result<Applied, EventRefusal> {
        self.ledger.apply(event, now)
    }

    /// Sets the store's chain head. A token is good only against the head
    /// it was made with.
    pub fn set_chain_head(&mut self, head: ChainHead) {
        self.chain_head = head;
    }

    /// Where an enrolled module stands, or `None` if it is not enrolled.
    pub fn module_state(&self, module: ModuleId) -> Option<ModuleState> {
        let enrollment = self.enrollments.get(&module)?;
        if let Some(reason) = self.quarantine.get(&module) {
            return Some(ModuleState::Quarantined(*reason));
        }
        if self
            .slots
            .values()
            .any(|s| s.state == PortState::Verified(module))
        {
            return Some(ModuleState::Available);
        }
        Some(
            match self.slots.get(&enrollment.hub_port).map(|s| s.state) {
                None => ModuleState::Absent,
                Some(PortState::Opening) => ModuleState::Discovered,
                Some(PortState::Verified(_) | PortState::Lost) => ModuleState::Lost,
                Some(PortState::Refused(why)) => ModuleState::Refused(why),
            },
        )
    }

    /// What a hub port holds, if a device is on it.
    pub fn port_state(&self, port: HubPort) -> Option<PortState> {
        self.slots.get(&port).map(|s| s.state)
    }

    /// Re-enrolls a module: a person's act, after a quarantine. It replaces
    /// the enrollment and ends the quarantine; a device refused for the
    /// quarantine is identified again on the next poll. Its facts stay
    /// invalidated.
    ///
    /// # Errors
    ///
    /// [`ConfigError`] if the new enrollment takes another module's hub port
    /// or bus port.
    pub fn re_enroll(&mut self, enrollment: Enrollment) -> Result<(), ConfigError> {
        for other in self.enrollments.values() {
            if other.module == enrollment.module {
                continue;
            }
            if other.hub_port == enrollment.hub_port {
                return Err(ConfigError::DuplicateHubPort(enrollment.hub_port));
            }
            if let Some(p) = other
                .carries
                .iter()
                .find(|p| enrollment.carries.contains(p))
            {
                return Err(ConfigError::SharedBusPort(p.clone()));
            }
        }
        let module = enrollment.module;
        self.quarantine.remove(&module);
        self.firmware.remove(&module);
        self.enrollments.insert(module, enrollment);
        for slot in self.slots.values_mut() {
            if slot.state == PortState::Refused(PortRefusal::Quarantined(module)) {
                slot.state = PortState::Lost;
            }
        }
        Ok(())
    }

    /// One pass of the main loop at tick `now`.
    ///
    /// In order: answer the trust root's heartbeat challenge (this is the
    /// only place it is answered, so a stalled loop stops the heartbeat and
    /// the trust root drops the rails); watch the trust root's report; find
    /// devices plugged in or out; take the frames that arrived; then mark
    /// lost every module past its deadline, invalidating its facts through
    /// the ledger, and poll the rest.
    ///
    /// `now` must not go backwards; the ledger treats a step back as a clock
    /// discontinuity and invalidates every fact.
    pub fn step(
        &mut self,
        now: Tick,
        hub: &mut impl ModuleTransport,
        interlock: &mut impl InterlockLink,
    ) -> Vec<Happened> {
        let mut out = Vec::new();
        self.answer_heartbeat(interlock, &mut out);
        self.watch_interlock(interlock, &mut out);
        self.discover(now, hub, &mut out);
        self.take_frames(now, hub, &mut out);
        self.deadlines(now, hub, &mut out);
        out
    }

    /// Answers each challenge once, with the reply this build computes.
    fn answer_heartbeat(&mut self, interlock: &mut impl InterlockLink, out: &mut Vec<Happened>) {
        if let Some(challenge) = interlock.challenge() {
            if self.answered != Some(challenge) {
                interlock.reply(challenge.reply());
                self.answered = Some(challenge);
                out.push(Happened::HeartbeatAnswered);
            }
        }
    }

    fn watch_interlock(&mut self, interlock: &impl InterlockLink, out: &mut Vec<Happened>) {
        let Some(plan) = self.armed.as_mut() else {
            return;
        };
        let Some(report) = interlock.report() else {
            return;
        };
        if report.faulted || (plan.rail_a_seen && !report.rail_a) {
            self.armed = None;
            out.push(Happened::PlanDropped);
        } else if report.rail_a {
            plan.rail_a_seen = true;
        }
    }

    pub(crate) fn plan_armed(&mut self, report: Report) {
        self.armed = Some(ArmedPlan {
            report,
            rail_a_seen: false,
        });
    }

    fn discover(&mut self, now: Tick, hub: &mut impl ModuleTransport, out: &mut Vec<Happened>) {
        let present: BTreeSet<HubPort> = hub.present().into_iter().collect();
        let gone: Vec<HubPort> = self
            .slots
            .keys()
            .filter(|p| !present.contains(p))
            .copied()
            .collect();
        for port in gone {
            if let Some(slot) = self.slots.remove(&port) {
                out.push(Happened::Unplugged(port));
                if let PortState::Verified(module) = slot.state {
                    out.push(Happened::Lost { module, port });
                    self.invalidate(module, now, out);
                }
            }
        }
        for port in present {
            if let std::collections::btree_map::Entry::Vacant(v) = self.slots.entry(port) {
                v.insert(Slot::new(now));
                out.push(Happened::Discovered(port));
                self.open(port, now, hub);
            }
        }
    }

    /// Sends an identify with a fresh challenge.
    fn open(&mut self, port: HubPort, now: Tick, hub: &mut impl ModuleTransport) {
        let challenge = self.challenges.challenge();
        let Some(slot) = self.slots.get_mut(&port) else {
            return;
        };
        let mut buf = [0u8; MAX_FRAME_LEN];
        slot.state = PortState::Opening;
        match slot.link.open(challenge, &mut buf) {
            Ok(len) => {
                slot.challenge = Some(challenge);
                slot.deadline = Some(now.saturating_add(LINK_TIMEOUT_TICKS));
                hub.send(port, &buf[..len]);
            }
            Err(_) => {
                slot.state = PortState::Lost;
                slot.next_poll = now.saturating_add(POLL_PERIOD_TICKS);
            }
        }
    }

    fn take_frames(&mut self, now: Tick, hub: &mut impl ModuleTransport, out: &mut Vec<Happened>) {
        let ports: Vec<HubPort> = self.slots.keys().copied().collect();
        for port in ports {
            while let Some(bytes) = hub.receive(port) {
                self.take_frame(port, &bytes, now, hub, out);
            }
        }
    }

    fn take_frame(
        &mut self,
        port: HubPort,
        bytes: &[u8],
        now: Tick,
        hub: &mut impl ModuleTransport,
        out: &mut Vec<Happened>,
    ) {
        let Some(slot) = self.slots.get_mut(&port) else {
            return;
        };
        if matches!(slot.state, PortState::Refused(_) | PortState::Lost) {
            // Not listening: a late reply after a loss is not taken either.
            return;
        }
        let response = match slot.link.receive(bytes) {
            Ok(r) => r.into_claim(),
            Err(error) => {
                out.push(Happened::FrameDropped { port, error });
                return;
            }
        };
        slot.deadline = None;
        match (slot.state, response) {
            (PortState::Opening, Response::Identity(claim)) => {
                self.identify(port, claim, now, hub, out);
            }
            (PortState::Opening, Response::Refused(code)) => {
                let why = PortRefusal::IdentifyRefused(code);
                slot.state = PortState::Refused(why);
                out.push(Happened::Refused { port, why });
            }
            (PortState::Verified(_), _) => {
                // Any answer to a poll keeps it alive. Its content is a claim
                // and is not used yet.
                slot.next_poll = now.saturating_add(POLL_PERIOD_TICKS);
            }
            _ => {}
        }
    }

    /// Judges a module's identity: verified against its enrollment first,
    /// so a device that cannot sign can only get itself refused; then bound
    /// to its hub port, unique, within its enrolled capabilities and with
    /// its pinned manifest, or quarantined.
    fn identify(
        &mut self,
        port: HubPort,
        claim: IdentityClaim,
        now: Tick,
        hub: &mut impl ModuleTransport,
        out: &mut Vec<Happened>,
    ) {
        let module = claim.module;
        let Some(slot) = self.slots.get_mut(&port) else {
            return;
        };
        let (Some(session), Some(challenge)) = (slot.link.session(), slot.challenge.take()) else {
            return;
        };
        let refuse = |slot: &mut Slot, why: PortRefusal, out: &mut Vec<Happened>| {
            slot.state = PortState::Refused(why);
            out.push(Happened::Refused { port, why });
        };
        if self.quarantine.contains_key(&module) {
            refuse(slot, PortRefusal::Quarantined(module), out);
            return;
        }
        let Some(enrollment) = self.enrollments.get(&module) else {
            refuse(slot, PortRefusal::Unenrolled(module), out);
            return;
        };
        let identity = match claim.verify(module, &enrollment.key, &challenge, session) {
            Ok(identity) => identity,
            Err(e) => {
                refuse(slot, PortRefusal::BadIdentity(e), out);
                return;
            }
        };
        let claimed = identity.capabilities();
        let enrolled = enrollment.capabilities;
        let first = self
            .slots
            .iter()
            .find(|(p, s)| **p != port && s.state == PortState::Verified(module))
            .map(|(p, _)| *p);
        let reason = if let Some(first) = first {
            Some(QuarantineReason::DuplicateId {
                first,
                second: port,
            })
        } else if enrollment.hub_port != port {
            Some(QuarantineReason::WrongHubPort {
                enrolled: enrollment.hub_port,
                seen: port,
            })
        } else if (claimed.can_emit && !enrolled.can_emit)
            || sawbones_policy::class_rank(claimed.max_class)
                > sawbones_policy::class_rank(enrolled.max_class)
        {
            Some(QuarantineReason::CapabilityExceeded { enrolled, claimed })
        } else if identity.manifest_digest() != enrollment.manifest {
            Some(QuarantineReason::ManifestMismatch)
        } else {
            None
        };
        if let Some(reason) = reason {
            self.quarantine_module(module, reason, port, now, hub, out);
            return;
        }
        let instrument = enrollment.instrument.clone();
        if let Some(slot) = self.slots.get_mut(&port) {
            slot.state = PortState::Verified(module);
            slot.next_poll = now.saturating_add(POLL_PERIOD_TICKS);
        }
        out.push(Happened::Available { module, port });
        // A new firmware digest, or the first one seen, means the module is
        // unverified as an instrument until it passes the known-answer check
        // (#178): every fact it measured is invalidated.
        let firmware = identity.firmware_digest();
        if self.firmware.insert(module, firmware) != Some(firmware) {
            if let Some(name) = instrument {
                let event = FactEvent::ModuleReflashed {
                    module: name,
                    firmware: firmware_label(&firmware),
                };
                self.apply_invalidation(event, now, out);
            }
        }
    }

    /// Quarantines `module`: every port verified as it, and `port`, is
    /// refused and told to go to its safe state, and its facts are
    /// invalidated.
    fn quarantine_module(
        &mut self,
        module: ModuleId,
        reason: QuarantineReason,
        port: HubPort,
        now: Tick,
        hub: &mut impl ModuleTransport,
        out: &mut Vec<Happened>,
    ) {
        self.quarantine.entry(module).or_insert(reason);
        out.push(Happened::Quarantined { module, reason });
        let why = PortRefusal::Quarantined(module);
        for (p, slot) in &mut self.slots {
            if *p == port || slot.state == PortState::Verified(module) {
                slot.state = PortState::Refused(why);
                slot.deadline = None;
                slot.link.abandon();
                let mut buf = [0u8; MAX_FRAME_LEN];
                if let Ok(len) = slot.link.send(&Request::SafeState, &mut buf) {
                    hub.send(*p, &buf[..len]);
                }
                out.push(Happened::Refused { port: *p, why });
            }
        }
        self.invalidate(module, now, out);
    }

    /// Invalidates, through the ledger, every fact on the bus ports the
    /// module carries, and every fact it measured.
    fn invalidate(&mut self, module: ModuleId, now: Tick, out: &mut Vec<Happened>) {
        let Some(enrollment) = self.enrollments.get(&module) else {
            return;
        };
        let mut events: Vec<FactEvent> = enrollment
            .carries
            .iter()
            .map(|port| FactEvent::PortRewired { port: port.clone() })
            .collect();
        if let (Some(name), Some(firmware)) =
            (enrollment.instrument.clone(), self.firmware.get(&module))
        {
            // The ledger has no event for a lost instrument. A reflash is the
            // nearest: it invalidates what the module measured and ends its
            // qualification, and that of every module qualified against it.
            // Nothing watched it while it was gone.
            events.push(FactEvent::ModuleReflashed {
                module: name,
                firmware: firmware_label(firmware),
            });
        }
        for event in events {
            self.apply_invalidation(event, now, out);
        }
    }

    fn apply_invalidation(&mut self, event: FactEvent, now: Tick, out: &mut Vec<Happened>) {
        // An invalidating event is always applied.
        if let Ok(applied) = self.ledger.apply(event, now) {
            if !applied.invalidated.is_empty() {
                out.push(Happened::FactsInvalidated(applied.invalidated));
            }
        }
    }

    fn deadlines(&mut self, now: Tick, hub: &mut impl ModuleTransport, out: &mut Vec<Happened>) {
        let ports: Vec<HubPort> = self.slots.keys().copied().collect();
        for port in ports {
            let Some(slot) = self.slots.get_mut(&port) else {
                continue;
            };
            if let Some(deadline) = slot.deadline {
                if now < deadline {
                    continue;
                }
                slot.link.abandon();
                slot.deadline = None;
                slot.next_poll = now.saturating_add(POLL_PERIOD_TICKS);
                let was = slot.state;
                slot.state = PortState::Lost;
                match was {
                    PortState::Verified(module) => {
                        out.push(Happened::Lost { module, port });
                        self.invalidate(module, now, out);
                    }
                    _ => out.push(Happened::IdentifyTimedOut(port)),
                }
                continue;
            }
            if now < slot.next_poll {
                continue;
            }
            match slot.state {
                PortState::Verified(_) => {
                    let mut buf = [0u8; MAX_FRAME_LEN];
                    match slot.link.send(&Request::Health, &mut buf) {
                        Ok(len) => {
                            slot.deadline = Some(now.saturating_add(LINK_TIMEOUT_TICKS));
                            hub.send(port, &buf[..len]);
                        }
                        // Sequence numbers used up: start a new session.
                        Err(_) => self.open(port, now, hub),
                    }
                }
                PortState::Lost => self.open(port, now, hub),
                PortState::Opening | PortState::Refused(_) => {}
            }
        }
    }

    /// The enrolled module that carries a bus port, and whether it is
    /// available.
    pub(crate) fn carrier(&self, port: &PortId) -> Option<(ModuleId, bool)> {
        let module = self
            .enrollments
            .values()
            .find(|e| e.carries.contains(port))?
            .module;
        Some((
            module,
            self.module_state(module) == Some(ModuleState::Available),
        ))
    }
}
