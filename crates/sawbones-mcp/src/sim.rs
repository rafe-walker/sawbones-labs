//! The simulated bench v0 serves: three simulated modules on a hub, the real
//! interlock state machine behind a link, and the supervisor on top.
//!
//! Simulated time advances only while a call is being served, one tick per
//! main-loop pass, so a run is repeatable. Module keys and identify
//! challenges come from [`OsRandom`]; nothing else is random.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read as _;
use std::num::NonZeroU64;

use ed25519_dalek::SigningKey;
use sawbones_interlock::{
    step, Event, FaultRecord, HeartbeatChallenge, HeartbeatReply, Inputs, Outputs, State,
};
use sawbones_isolation::{BenchConfig, Edge, MacPower, ModuleDeclaration, Node, Role};
use sawbones_module_protocol::{Challenge, ModuleId, ModuleKey};
use sawbones_policy::{BusFacts, LedgerId, ScopeName, WriteMap};
use sawbones_sim_modules::{ModuleKind, SimLink, SimModule};
use sawbones_supervisor::{
    ChallengeSource, Config, Enrollment, HubPort, InterlockLink, InterlockReport, ModuleTransport,
    Supervisor, Tick, ToInterlock,
};

/// Random bytes from the operating system (`/dev/urandom`), for module keys
/// and identify challenges. A counter would do in a test, but a challenge
/// must not be predictable, and the simulated modules' keys must not be
/// fixed in the source.
#[derive(Debug)]
pub struct OsRandom {
    file: File,
}

impl OsRandom {
    /// Opens the source.
    ///
    /// # Errors
    ///
    /// The I/O error, if `/dev/urandom` cannot be opened.
    pub fn open() -> std::io::Result<Self> {
        Ok(OsRandom {
            file: File::open("/dev/urandom")?,
        })
    }

    /// Fills `out`.
    ///
    /// # Panics
    ///
    /// If the source cannot be read: a bench with no randomness must not
    /// start or go on.
    pub fn fill(&mut self, out: &mut [u8]) {
        self.file
            .read_exact(out)
            .unwrap_or_else(|e| panic!("the random source failed: {e}"));
    }

    /// 32 fresh bytes.
    pub fn bytes32(&mut self) -> [u8; 32] {
        let mut out = [0u8; 32];
        self.fill(&mut out);
        out
    }
}

impl ChallengeSource for OsRandom {
    fn challenge(&mut self) -> Challenge {
        let mut out = [0u8; 16];
        self.fill(&mut out);
        Challenge(out)
    }
}

/// One simulated module to put on the bench.
#[derive(Debug, Clone)]
pub struct ModuleSpec {
    /// Its kind.
    pub kind: ModuleKind,
    /// The owner's label for it, from the bench config. Host-written.
    pub label: ScopeName,
    /// A manifest to serve in place of the kind's own, if any. The
    /// enrollment pins whatever it serves, as an install would.
    pub manifest: Option<Vec<Vec<u8>>>,
}

impl ModuleSpec {
    /// A module of `kind` with the kind's own manifest, labelled with the
    /// kind's name.
    ///
    /// # Panics
    ///
    /// Never: every kind's name is a valid label.
    #[must_use]
    pub fn of(kind: ModuleKind) -> Self {
        ModuleSpec {
            kind,
            label: ScopeName::parse(kind.name())
                .unwrap_or_else(|_| unreachable!("kind names are valid labels")),
            manifest: None,
        }
    }
}

/// The hub: one simulated line per hub port.
#[derive(Debug, Default)]
pub struct SimHub {
    /// The lines, by hub port.
    pub links: BTreeMap<HubPort, SimLink>,
    now: Tick,
}

impl ModuleTransport for SimHub {
    fn present(&self) -> Vec<HubPort> {
        self.links.keys().copied().collect()
    }

    fn send(&mut self, port: HubPort, frame: &[u8]) {
        if let Some(link) = self.links.get_mut(&port) {
            link.send_from_host(self.now, frame.to_vec());
        }
    }

    fn receive(&mut self, port: HubPort) -> Option<Vec<u8>> {
        self.links.get_mut(&port)?.next_for_host(self.now)
    }
}

/// The real interlock state machine behind a link. Its feedback follows its
/// outputs. Nobody presses anything unless a test does.
#[derive(Debug)]
pub struct SimInterlock {
    state: State,
    outputs: Outputs,
    /// The arm switch.
    pub arm_switch_on: bool,
    /// The key switch.
    pub key_switch_on: bool,
    /// The panel button.
    pub panel_button_pressed: bool,
    pending_reply: Option<HeartbeatReply>,
    events: Vec<Event>,
    nonce: NonZeroU64,
    started: bool,
}

impl SimInterlock {
    fn new(nonce: NonZeroU64) -> Self {
        SimInterlock {
            state: State::POWER_ON,
            outputs: State::POWER_ON.outputs(),
            arm_switch_on: false,
            key_switch_on: false,
            panel_button_pressed: false,
            pending_reply: None,
            events: Vec::new(),
            nonce,
            started: false,
        }
    }

    /// What it drives now.
    #[must_use]
    pub const fn outputs(&self) -> Outputs {
        self.outputs
    }

    fn tick(&mut self, now: Tick) {
        let event = if self.started {
            (!self.events.is_empty()).then(|| self.events.remove(0))
        } else {
            self.started = true;
            Some(Event::Restart {
                record: Some(FaultRecord::Clear),
                nonce: Some(self.nonce),
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
}

impl InterlockLink for SimInterlock {
    fn challenge(&self) -> Option<HeartbeatChallenge> {
        self.outputs.challenge
    }

    fn reply(&mut self, reply: HeartbeatReply) {
        self.pending_reply = Some(reply);
    }

    fn send(&mut self, request: ToInterlock) {
        // v0 sends nothing: it has no arm, lease or invoke tool. A request
        // is still passed on as the real link would.
        self.events.push(match request {
            ToInterlock::RequestArm => Event::RequestArm,
            ToInterlock::RequestLease { target } => Event::RequestLease {
                class: sawbones_interlock::HazardClass::Class3,
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

/// The simulated bench: supervisor, hub and interlock, and the clock.
pub struct SimBench {
    /// The supervisor.
    pub supervisor: Supervisor,
    /// The hub.
    pub hub: SimHub,
    /// The interlock.
    pub interlock: SimInterlock,
    labels: BTreeMap<ModuleId, ScopeName>,
    now: Tick,
}

impl std::fmt::Debug for SimBench {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimBench")
            .field("supervisor", &self.supervisor)
            .field("now", &self.now)
            .finish_non_exhaustive()
    }
}

/// How many passes [`SimBench::new`] runs so the modules identify.
pub const START_TICKS: Tick = 5;

impl SimBench {
    /// The default bench: a DC meter (module 1, hub port 0), a comms pod
    /// (module 2, hub port 1) and an interlock controller (module 3, hub
    /// port 2).
    ///
    /// # Errors
    ///
    /// The I/O error, if the random source cannot be opened.
    pub fn standard() -> std::io::Result<Self> {
        Ok(Self::new(
            &ModuleKind::ALL.map(ModuleSpec::of),
            Box::new(OsRandom::open()?),
            OsRandom::open()?,
        ))
    }

    /// A bench with `specs` on hub ports 0, 1, 2 and so on, as modules 1,
    /// 2, 3 and so on. Keys come from `keys`, challenges from `challenges`.
    /// Runs [`START_TICKS`] passes so the modules identify.
    ///
    /// # Panics
    ///
    /// If more than 255 modules are given, or the supervisor refuses the
    /// config (it cannot: ports and ids are distinct).
    #[must_use]
    pub fn new(
        specs: &[ModuleSpec],
        challenges: Box<dyn ChallengeSource>,
        mut keys: OsRandom,
    ) -> Self {
        let mut hub = SimHub::default();
        let mut enrollments = Vec::new();
        let mut labels = BTreeMap::new();
        // The owner's bench config: the brain, a hub, and each module behind
        // its own USB isolator, with no terminals wired. The Mac is on its
        // battery.
        let mut bench_config = BenchConfig::new(MacPower::OnBattery)
            .node(Node::new("pi", Role::Brain))
            .node(Node::new("hub", Role::Other))
            .edge(Edge::bonded("pi-usb", "pi", "hub"));
        for (index, spec) in specs.iter().enumerate() {
            let n = u8::try_from(index).unwrap_or_else(|_| panic!("too many modules"));
            let id = ModuleId(u32::from(n) + 1);
            let seed = keys.bytes32();
            let mut module = SimModule::new(spec.kind, id, seed, u64::from(n) + 1);
            if let Some(pages) = &spec.manifest {
                module = module.with_manifest(pages.clone());
            }
            let port = HubPort(n);
            enrollments.push(Enrollment {
                module: id,
                hub_port: port,
                key: ModuleKey(SigningKey::from_bytes(&seed).verifying_key().to_bytes()),
                manifest: module.manifest_digest(),
                capabilities: spec.kind.capabilities(),
                instrument: (spec.kind == ModuleKind::DcMeter).then(|| spec.label.clone()),
                carries: Vec::new(),
            });
            let node = format!("module-{}", id.0);
            bench_config = bench_config
                .edge(Edge::isolated(
                    &format!("usb-isolator-{}", id.0),
                    "hub",
                    &node,
                    1000,
                ))
                .module(id.0, &node, Vec::new());
            labels.insert(id, spec.label.clone());
            hub.links.insert(port, SimLink::new(module, None));
        }
        let nonce = loop {
            let mut b = [0u8; 8];
            keys.fill(&mut b);
            if let Some(n) = NonZeroU64::new(u64::from_le_bytes(b)) {
                break n;
            }
        };
        let modules: Vec<ModuleId> = labels.keys().copied().collect();
        let mut supervisor = Supervisor::new(
            Config {
                ledger: LedgerId(1),
                bench: bench_config,
                enrollments,
                writes: WriteMap::new([]).unwrap_or_else(|_| unreachable!("empty map")),
                buses: BusFacts::new([]).unwrap_or_else(|_| unreachable!("no facts")),
            },
            challenges,
        )
        .unwrap_or_else(|e| panic!("bench config refused: {e:?}"));
        // What each simulated module says about its terminals: none, as the
        // config says. Real modules will carry this in their manifests.
        for module in modules {
            supervisor
                .declare_terminals(module, ModuleDeclaration::new(Vec::new()))
                .unwrap_or_else(|e| panic!("declaration refused: {e:?}"));
        }
        let mut bench = SimBench {
            supervisor,
            hub,
            interlock: SimInterlock::new(nonce),
            labels,
            now: 0,
        };
        bench.advance(START_TICKS);
        bench
    }

    /// The simulated clock.
    #[must_use]
    pub const fn now(&self) -> Tick {
        self.now
    }

    /// The owner's label for `module`, if it is on the bench.
    #[must_use]
    pub fn label(&self, module: ModuleId) -> Option<&ScopeName> {
        self.labels.get(&module)
    }

    /// Runs `ticks` passes: the lines, the interlock, then the supervisor's
    /// main loop.
    pub fn advance(&mut self, ticks: Tick) {
        for _ in 0..ticks {
            let now = self.now;
            self.hub.now = now;
            for link in self.hub.links.values_mut() {
                link.pump(now);
            }
            self.interlock.tick(now);
            let _ = self
                .supervisor
                .step(now, &mut self.hub, &mut self.interlock);
            self.now = now.saturating_add(1);
        }
    }
}
