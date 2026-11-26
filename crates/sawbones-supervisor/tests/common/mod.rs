//! Minimal in-memory fakes: modules that speak the real module protocol, a
//! hub, the real interlock state machine behind a link, and a bus.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::num::NonZeroU64;

use ed25519_dalek::{Signer, SigningKey};
use sawbones_interlock::{
    step, ClearSource, Event, FaultRecord, HazardClass, HeartbeatReply, Inputs, Outputs, State,
};
use sawbones_module_protocol::{
    Capabilities, Challenge, Channel, ClassTable, ConfigKey, HealthReport, HealthStatus,
    IdentityClaim, IdentitySignature, ImageDigest, ManifestDigest, ManifestHasher, ModuleId,
    ModuleKey, ModuleLink, RefusalCode, Request, Response, MAX_FRAME_LEN,
};
use sawbones_policy::{
    BusFact, BusFacts, DeviceIdentity, FactContent, FactEvent, FactRecord, FactScope, Interface,
    LedgerId, Mastership, Method, PortId, ScopeLabel, ScopeName, SettingKind, SignalId, Validity,
    WriteEntry, WriteMap,
};
use sawbones_reading::Trust;
use sawbones_supervisor::{
    Authorized, BusReply, BusTransport, ChallengeSource, Config, Enrollment, HubPort,
    InterlockLink, InterlockReport, ModuleTransport, Supervisor, Tick, ToInterlock,
};

/// TEST ONLY. The comms pod's identity seed. Public, in tests only.
pub const POD_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY supervisor-pod";
/// TEST ONLY. The meter's identity seed.
pub const METER_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY supervisor-mtr";
/// TEST ONLY. A key nobody enrolled.
pub const STRANGER_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY supervisor-xxx";

pub const POD: ModuleId = ModuleId(2);
pub const METER: ModuleId = ModuleId(1);
pub const POD_PORT: HubPort = HubPort(0);
pub const METER_PORT: HubPort = HubPort(1);
pub const SPARE_PORT: HubPort = HubPort(3);

pub const MANIFEST: &[u8] = b"name: test module\n";

pub fn manifest_digest() -> ManifestDigest {
    let mut h = ManifestHasher::new();
    h.bytes(MANIFEST);
    h.finish()
}

pub fn key(seed: &[u8; 32]) -> ModuleKey {
    ModuleKey(SigningKey::from_bytes(seed).verifying_key().to_bytes())
}

pub fn bus_port() -> PortId {
    PortId::parse("rs485-1").unwrap()
}

pub fn signal() -> SignalId {
    SignalId::parse("test_setpoint").unwrap()
}

pub fn quiet() -> Capabilities {
    Capabilities {
        can_emit: false,
        max_class: HazardClass::Class0,
    }
}

/// A module with no config keys and no channels.
struct NoTable;

impl ClassTable for NoTable {
    fn config_class(&self, _: ConfigKey) -> Option<HazardClass> {
        None
    }
    fn channel_class(&self, _: Channel) -> Option<HazardClass> {
        None
    }
}

/// A module running the real `ModuleLink`, signing its identity with a
/// fixed test seed.
pub struct FakeModule {
    pub link: ModuleLink,
    pub seed: [u8; 32],
    pub id: ModuleId,
    pub capabilities: Capabilities,
    pub firmware: ImageDigest,
    pub manifest: ManifestDigest,
    /// Answers nothing while false.
    pub responsive: bool,
    next_session: u64,
}

impl FakeModule {
    pub fn new(seed: [u8; 32], id: ModuleId) -> Self {
        FakeModule {
            link: ModuleLink::new(id),
            seed,
            id,
            capabilities: quiet(),
            firmware: ImageDigest([7; 32]),
            manifest: manifest_digest(),
            responsive: true,
            next_session: u64::from(id.0) << 32,
        }
    }

    /// Takes one frame from the host and returns the reply, if any.
    pub fn handle(&mut self, frame: &[u8]) -> Option<Vec<u8>> {
        if !self.responsive {
            return None;
        }
        let next = &mut self.next_session;
        let accepted = match self.link.accept(
            frame,
            || {
                *next += 1;
                *next
            },
            &NoTable,
        ) {
            Ok(a) => a,
            Err(sawbones_module_protocol::Rejection::Refuse { reply_to, code }) => {
                return self.reply(reply_to, &Response::Refused(code));
            }
            Err(_) => return None,
        };
        let response = match accepted.request {
            Request::Identify { challenge } => Response::Identity(self.sign(&challenge)),
            Request::Health => Response::Health(HealthReport {
                status: HealthStatus::Ok,
                uptime_ticks: 1,
                fault_code: 0,
                bad_frames: 0,
                refused_frames: 0,
            }),
            Request::SafeState => Response::SafeStateEntered,
            _ => Response::Refused(RefusalCode::UnknownKey),
        };
        self.reply(accepted.seq, &response)
    }

    fn reply(&mut self, reply_to: u32, response: &Response<'_>) -> Option<Vec<u8>> {
        let mut buf = [0u8; MAX_FRAME_LEN];
        let len = self.link.reply(reply_to, response, &mut buf).ok()?;
        Some(buf[..len].to_vec())
    }

    fn sign(&self, challenge: &Challenge) -> IdentityClaim {
        let session: NonZeroU64 = self.link.session().unwrap();
        let mut claim = IdentityClaim {
            module: self.id,
            capabilities: self.capabilities,
            manifest_digest: self.manifest,
            firmware_digest: self.firmware,
            signature: IdentitySignature([0; 64]),
        };
        claim.signature = IdentitySignature(
            SigningKey::from_bytes(&self.seed)
                .sign(&claim.signed_bytes(challenge, session))
                .to_bytes(),
        );
        claim
    }
}

/// A hub: one fake module per port. A module answers in the same call.
#[derive(Default)]
pub struct FakeHub {
    pub modules: BTreeMap<HubPort, FakeModule>,
    inbox: BTreeMap<HubPort, VecDeque<Vec<u8>>>,
    /// Every frame the host sent, by port.
    pub sent: BTreeMap<HubPort, Vec<Vec<u8>>>,
}

impl FakeHub {
    pub fn plug(&mut self, port: HubPort, module: FakeModule) {
        self.modules.insert(port, module);
    }

    pub fn unplug(&mut self, port: HubPort) -> Option<FakeModule> {
        self.inbox.remove(&port);
        self.modules.remove(&port)
    }

    pub fn module(&mut self, port: HubPort) -> &mut FakeModule {
        self.modules.get_mut(&port).unwrap()
    }
}

impl ModuleTransport for FakeHub {
    fn present(&self) -> Vec<HubPort> {
        self.modules.keys().copied().collect()
    }

    fn send(&mut self, port: HubPort, frame: &[u8]) {
        self.sent.entry(port).or_default().push(frame.to_vec());
        if let Some(m) = self.modules.get_mut(&port) {
            if let Some(reply) = m.handle(frame) {
                self.inbox.entry(port).or_default().push_back(reply);
            }
        }
    }

    fn receive(&mut self, port: HubPort) -> Option<Vec<u8>> {
        self.inbox.get_mut(&port)?.pop_front()
    }
}

/// The real interlock state machine behind a link. Feedback follows the
/// outputs, so no output mismatch latches.
pub struct FakeInterlock {
    pub state: State,
    pub outputs: Outputs,
    pub arm_switch_on: bool,
    pub key_switch_on: bool,
    pub panel_button_pressed: bool,
    pending_reply: Option<HeartbeatReply>,
    events: VecDeque<Event>,
    /// Every request the supervisor sent.
    pub requests: Vec<ToInterlock>,
    started: bool,
}

impl Default for FakeInterlock {
    fn default() -> Self {
        FakeInterlock {
            state: State::POWER_ON,
            outputs: State::POWER_ON.outputs(),
            arm_switch_on: false,
            key_switch_on: false,
            panel_button_pressed: false,
            pending_reply: None,
            events: VecDeque::new(),
            requests: Vec::new(),
            started: false,
        }
    }
}

impl FakeInterlock {
    /// One interlock tick. The first is a clean restart with a fixed nonce.
    pub fn tick(&mut self, now: Tick) {
        let event = if self.started {
            self.events.pop_front()
        } else {
            self.started = true;
            Some(Event::Restart {
                record: Some(FaultRecord::Clear),
                nonce: 0x5eed,
            })
        };
        let inputs = Inputs {
            estop_pressed: false,
            arm_switch_on: self.arm_switch_on,
            key_switch_on: self.key_switch_on,
            heartbeat: self.pending_reply.take(),
            panel_button_pressed: self.panel_button_pressed,
            rail_a_sensed: self.outputs.rail_a,
            rail_b_sensed: core::array::from_fn(|p| {
                self.outputs.rail_b.map(|b| usize::from(b.0)) == Some(p)
            }),
            firmware_jumper_fitted: false,
            now,
        };
        let (state, outputs) = step(self.state, inputs, event);
        self.state = state;
        self.outputs = outputs;
    }

    /// A clear from the panel, for tests that need one.
    pub fn panel_clear(&mut self) {
        self.events.push_back(Event::ClearFault {
            source: ClearSource::Panel(sawbones_interlock::PanelToken::from_panel_button()),
        });
    }
}

impl InterlockLink for FakeInterlock {
    fn challenge(&self) -> Option<sawbones_interlock::HeartbeatChallenge> {
        self.outputs.challenge
    }

    fn reply(&mut self, reply: HeartbeatReply) {
        self.pending_reply = Some(reply);
    }

    fn send(&mut self, request: ToInterlock) {
        self.requests.push(request);
        self.events.push_back(match request {
            ToInterlock::RequestArm => Event::RequestArm,
            ToInterlock::RequestLease { target } => Event::RequestLease {
                class: HazardClass::Class3,
                target,
            },
            ToInterlock::LeaseUsed { target } => Event::LeaseUsed { target },
        });
    }

    fn report(&self) -> Option<InterlockReport> {
        Some(InterlockReport {
            rules_id: sawbones_interlock::rules_id(),
            rail_a: self.outputs.rail_a,
            rail_b: self.outputs.rail_b,
            faulted: self.outputs.fault.is_some(),
        })
    }
}

/// A bus that records what it was asked to do and replies as told.
pub struct FakeBus {
    pub reply: BusReply,
    pub performed: Vec<(sawbones_policy::EffectRequest, u8)>,
}

impl Default for FakeBus {
    fn default() -> Self {
        FakeBus {
            reply: BusReply::Done,
            performed: Vec::new(),
        }
    }
}

impl BusTransport for FakeBus {
    fn perform(&mut self, action: &Authorized) -> BusReply {
        self.performed
            .push((action.request().clone(), action.bench_port()));
        self.reply
    }
}

/// TEST ONLY. Counting challenges: predictable, which is fine in a test.
pub struct Counter(pub u8);

impl ChallengeSource for Counter {
    fn challenge(&mut self) -> Challenge {
        self.0 = self.0.wrapping_add(1);
        Challenge([self.0; 16])
    }
}

pub fn pod_enrollment() -> Enrollment {
    Enrollment {
        module: POD,
        hub_port: POD_PORT,
        key: key(&POD_SEED),
        manifest: manifest_digest(),
        capabilities: Capabilities {
            can_emit: true,
            max_class: HazardClass::Class3,
        },
        instrument: None,
        carries: vec![bus_port()],
    }
}

pub fn meter_enrollment() -> Enrollment {
    Enrollment {
        module: METER,
        hub_port: METER_PORT,
        key: key(&METER_SEED),
        manifest: manifest_digest(),
        capabilities: quiet(),
        instrument: Some(ScopeName::parse("dc-meter").unwrap()),
        carries: Vec::new(),
    }
}

pub fn write_entry(class: HazardClass) -> WriteEntry {
    WriteEntry {
        port: bus_port(),
        unit: 1,
        signal: signal(),
        register: 0x0030,
        width: 1,
        class,
        kind: SettingKind::Config,
        energy_relevant: false,
        trust: Trust::OperatorAttested,
        range: None,
    }
}

pub fn bus_fact() -> BusFact {
    BusFact {
        port: bus_port(),
        bench_port: 0,
        mastership: Mastership::BenchOnly,
        trust: Trust::OperatorAttested,
    }
}

pub fn supervisor(class: HazardClass) -> Supervisor {
    Supervisor::new(
        Config {
            ledger: LedgerId(1),
            enrollments: vec![pod_enrollment(), meter_enrollment()],
            writes: WriteMap::new([write_entry(class)]).unwrap(),
            buses: BusFacts::new([bus_fact()]).unwrap(),
        },
        Box::new(Counter(0)),
    )
    .unwrap()
}

pub fn device() -> DeviceIdentity {
    DeviceIdentity {
        model: ScopeName::parse("inverter-a").unwrap(),
        serial: ScopeLabel::parse("SN0001").unwrap(),
        firmware: ScopeLabel::parse("V1.0").unwrap(),
    }
}

/// Identifies the device on the pod's bus port and records the write entry
/// and the mastership fact, at `now`.
pub fn record_facts(s: &mut Supervisor, class: HazardClass, now: Tick) {
    s.apply_fact_event(
        FactEvent::DeviceIdentified {
            port: bus_port(),
            unit: 1,
            identity: device(),
        },
        now,
    )
    .unwrap();
    let scope = |unit, device| FactScope {
        device,
        interface: Interface {
            port: bus_port(),
            bench_port: 0,
            unit,
        },
        conditions: BTreeSet::new(),
        method: Method::OperatorAttested,
    };
    s.record_fact(
        FactRecord {
            content: FactContent::WriteEntry(write_entry(class)),
            scope: scope(Some(1), Some(device())),
            earned_at: now,
            validity: Validity::UntilInvalidated,
        },
        now,
    )
    .unwrap();
    s.record_fact(
        FactRecord {
            content: FactContent::Mastership(bus_fact()),
            scope: scope(None, None),
            earned_at: now,
            validity: Validity::For { ticks: 100_000 },
        },
        now,
    )
    .unwrap();
}

/// A clean connection plan: the brain, a hub, and an isolated meter.
pub fn clean_plan() -> sawbones_isolation::IsolationMap {
    use sawbones_isolation::{Edge, MapSpec, Node, Role};
    MapSpec::new()
        .node(Node::new("pi", Role::Brain))
        .node(Node::new("hub", Role::Other))
        .node(Node::new("meter", Role::MeasuringModule).must_float())
        .edge(Edge::bonded("pi-usb", "pi", "hub"))
        .edge(Edge::isolated("usb-isolator", "hub", "meter", 1000))
        .build()
        .unwrap()
}

/// The same plan with a ground clip from the hub to the meter: a Stop.
pub fn clipped_plan() -> sawbones_isolation::IsolationMap {
    clean_plan()
        .with_edge(sawbones_isolation::Edge::bonded("clip", "hub", "meter"))
        .unwrap()
}

/// A bench: the supervisor, a hub with the pod and the meter plugged in,
/// the real interlock and a bus.
pub struct Bench {
    pub s: Supervisor,
    pub hub: FakeHub,
    pub interlock: FakeInterlock,
    pub bus: FakeBus,
    pub now: Tick,
}

impl Bench {
    pub fn new(class: HazardClass) -> Self {
        let mut hub = FakeHub::default();
        hub.plug(POD_PORT, FakeModule::new(POD_SEED, POD));
        hub.plug(METER_PORT, FakeModule::new(METER_SEED, METER));
        Bench {
            s: supervisor(class),
            hub,
            interlock: FakeInterlock::default(),
            bus: FakeBus::default(),
            now: 0,
        }
    }

    /// One tick: the interlock steps, then the supervisor's main loop.
    pub fn tick(&mut self) -> Vec<sawbones_supervisor::Happened> {
        self.interlock.tick(self.now);
        let out = self.s.step(self.now, &mut self.hub, &mut self.interlock);
        self.now += 1;
        out
    }

    /// Ticks the interlock only: the supervisor's loop has stopped.
    pub fn tick_hung(&mut self) {
        self.interlock.tick(self.now);
        self.now += 1;
    }

    pub fn run(&mut self, ticks: u64) -> Vec<sawbones_supervisor::Happened> {
        let mut all = Vec::new();
        for _ in 0..ticks {
            all.extend(self.tick());
        }
        all
    }

    /// Brings rail A up: the heartbeat answered, the arm switch off then on,
    /// and a clean plan armed.
    pub fn arm(&mut self) {
        self.run(3);
        self.s.arm(&clean_plan(), &mut self.interlock).unwrap();
        self.interlock.arm_switch_on = true;
        self.run(2);
        assert!(self.interlock.outputs.rail_a, "rail A did not come up");
    }
}
