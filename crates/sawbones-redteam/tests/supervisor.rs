//! Supervisor attacks (cards hw-023, hw-011 and bus-007), against the
//! supervisor skeleton (#100) with fake modules speaking the real module
//! protocol and the real interlock state machine. The fakes are the
//! supervisor crate's own, so the two cannot drift.

#[path = "../../sawbones-supervisor/tests/common/mod.rs"]
mod common;

use common::*;
use sawbones_interlock::{FaultReason, HazardClass, HEARTBEAT_PERIOD_TICKS};
use sawbones_policy::{BusRead, BusWrite, EffectRequest, WriteValue};
use sawbones_supervisor::{
    BusReply, Happened, InvokeRefusal, ModuleState, Outcome, QuarantineReason, POLL_PERIOD_TICKS,
};

fn write(value: u16) -> EffectRequest {
    EffectRequest::BusWrite(BusWrite {
        port: bus_port(),
        unit: 1,
        signal: signal(),
        value: WriteValue::U16(value),
    })
}

#[test]
fn redteam_hw_023_a_duplicate_id_is_quarantined() {
    // A copy of the pod's key on a second hub port: neither can be told from
    // the other, so both are refused until a person re-enrolls the module.
    let mut b = Bench::new(HazardClass::Class2);
    b.tick();
    record_facts(&mut b.s, HazardClass::Class2, b.now);
    b.hub.plug(SPARE_PORT, FakeModule::new(POD_SEED, POD));
    let out = b.tick();
    assert!(out.contains(&Happened::Quarantined {
        module: POD,
        reason: QuarantineReason::DuplicateId {
            first: POD_PORT,
            second: SPARE_PORT
        }
    }));
    assert!(matches!(
        b.s.module_state(POD),
        Some(ModuleState::Quarantined(_))
    ));
    // What it carried can no longer be acted on.
    assert!(b.s.preflight(&write(10), b.now).is_err());
    b.hub.unplug(SPARE_PORT);
    b.run(3 * POLL_PERIOD_TICKS);
    assert!(matches!(
        b.s.module_state(POD),
        Some(ModuleState::Quarantined(_))
    ));
}

#[test]
fn redteam_hw_023_a_module_that_changes_hub_port_is_quarantined() {
    // Two genuine modules swapped between ports: both prove their keys, and
    // both are where they were not enrolled.
    let mut b = Bench::new(HazardClass::Class2);
    let pod = b.hub.unplug(POD_PORT).unwrap();
    let meter = b.hub.unplug(METER_PORT).unwrap();
    b.hub.plug(POD_PORT, meter);
    b.hub.plug(METER_PORT, pod);
    b.tick();
    assert_eq!(
        b.s.module_state(POD),
        Some(ModuleState::Quarantined(QuarantineReason::WrongHubPort {
            enrolled: POD_PORT,
            seen: METER_PORT
        }))
    );
    assert_eq!(
        b.s.module_state(METER),
        Some(ModuleState::Quarantined(QuarantineReason::WrongHubPort {
            enrolled: METER_PORT,
            seen: POD_PORT
        }))
    );
}

#[test]
fn redteam_hw_011_a_hung_supervisor_loses_the_rails_within_two_periods() {
    // The supervisor answers challenges from its main loop only: when the
    // loop stops, so does the heartbeat, and the trust root cuts the rails.
    let mut b = Bench::new(HazardClass::Class2);
    b.arm();
    while !b.tick().contains(&Happened::HeartbeatAnswered) {}
    let last_reply = b.now;
    let mut off_at = None;
    for _ in 0..3 * HEARTBEAT_PERIOD_TICKS {
        b.tick_hung();
        if !b.interlock.outputs.rail_a {
            off_at = Some(b.now - 1);
            break;
        }
    }
    let off_at = off_at.expect("rail A stayed on");
    assert!(off_at <= last_reply + 2 * HEARTBEAT_PERIOD_TICKS);
    assert_eq!(b.interlock.outputs.fault, Some(FaultReason::HeartbeatLost));
}

#[test]
fn redteam_bus_007_a_write_with_no_reply_blocks_new_leases_until_a_readback() {
    let mut b = Bench::new(HazardClass::Class3);
    b.arm();
    b.interlock.key_switch_on = true;
    record_facts(&mut b.s, HazardClass::Class3, b.now);
    let token = b.s.preflight(&write(10), b.now).unwrap();
    b.s.request_lease(&token, &mut b.interlock).unwrap();
    b.tick();
    b.interlock.panel_button_pressed = true;
    b.tick();
    b.interlock.panel_button_pressed = false;
    b.bus.reply = BusReply::NoReply;
    let got = b.invoke(&write(10), token);
    assert_eq!(got, Ok(Outcome::Unknown));

    // The agent retries: no new lease, no write.
    let token = b.s.preflight(&write(10), b.now).unwrap();
    assert!(matches!(
        b.s.request_lease(&token, &mut b.interlock),
        Err(InvokeRefusal::OutcomeUnknown { .. })
    ));
    let got = b.invoke(&write(10), token);
    assert!(matches!(got, Err(InvokeRefusal::OutcomeUnknown { .. })));
    assert_eq!(b.bus.performed.len(), 1);

    // A read back that answers settles it.
    let read = EffectRequest::BusRead(BusRead {
        port: bus_port(),
        unit: 1,
        signal: signal(),
    });
    b.bus.reply = BusReply::Value(WriteValue::U16(9));
    let token = b.s.preflight(&read, b.now).unwrap();
    let got = b.invoke(&read, token);
    assert_eq!(
        got,
        Ok(Outcome::ReadBack {
            value: WriteValue::U16(9),
            applied: false
        })
    );
    let token = b.s.preflight(&write(10), b.now).unwrap();
    assert!(b.s.request_lease(&token, &mut b.interlock).is_ok());
}

// ---- The supervisor against sawbones-sim-modules scenarios (#99) ----------

mod sim {
    use std::collections::BTreeMap;

    use ed25519_dalek::SigningKey;
    use sawbones_module_protocol::{decode_request, ModuleId, ModuleKey, Request};
    use sawbones_policy::{BusFacts, LedgerId, ScopeName, WriteMap};
    use sawbones_sim_modules::{Dir, Fault, ModuleKind, Scenario, SimLink, SimModule};
    use sawbones_supervisor::{
        Config, Enrollment, Happened, HubPort, ModuleTransport, Supervisor, Tick,
    };

    use super::common::{bus_fact, bus_port, write_entry, Counter, FakeInterlock};
    use sawbones_interlock::HazardClass;

    /// TEST ONLY. The simulated pod's device seed. Public, in tests only.
    pub const POD_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY redteam-simpod";
    /// TEST ONLY. The simulated meter's device seed.
    pub const METER_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY redteam-simmtr";
    pub const POD: ModuleId = ModuleId(20);
    pub const METER: ModuleId = ModuleId(21);
    pub const POD_PORT: HubPort = HubPort(0);
    pub const METER_PORT: HubPort = HubPort(1);
    pub const SPARE_PORT: HubPort = HubPort(3);

    pub fn pod(rng: u64) -> SimModule {
        SimModule::new(ModuleKind::CommsPod, POD, POD_SEED, rng)
    }

    pub fn meter(rng: u64) -> SimModule {
        SimModule::new(ModuleKind::DcMeter, METER, METER_SEED, rng)
    }

    fn enrollment(
        kind: ModuleKind,
        module: ModuleId,
        port: HubPort,
        seed: &[u8; 32],
    ) -> Enrollment {
        Enrollment {
            module,
            hub_port: port,
            key: ModuleKey(SigningKey::from_bytes(seed).verifying_key().to_bytes()),
            manifest: kind.manifest_digest(),
            capabilities: kind.capabilities(),
            instrument: (kind == ModuleKind::DcMeter)
                .then(|| ScopeName::parse("dc-meter").unwrap()),
            carries: if kind == ModuleKind::CommsPod {
                vec![bus_port()]
            } else {
                Vec::new()
            },
        }
    }

    /// Simulated modules on simulated lines, one per hub port.
    #[derive(Default)]
    pub struct SimHub {
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

    impl SimHub {
        /// Replays the host's first recorded `Identify` to the module on
        /// `port`, as anyone on the line can.
        pub fn replay_identify(&mut self, port: HubPort) {
            let now = self.now;
            let link = self.links.get_mut(&port).unwrap();
            let frame = link
                .recorded(Dir::ToModule)
                .iter()
                .find(|f| matches!(decode_request(f), Ok(fr) if matches!(fr.request, Request::Identify { .. })))
                .cloned()
                .unwrap();
            link.inject_to_module(now, &frame);
        }
    }

    pub struct Bench {
        pub s: Supervisor,
        pub hub: SimHub,
        pub interlock: FakeInterlock,
    }

    impl Bench {
        /// The pod with `scenario` on its line, and a healthy meter.
        pub fn new(pod_line: Option<Scenario>) -> Self {
            let s = Supervisor::new(
                Config {
                    ledger: LedgerId(9),
                    enrollments: vec![
                        enrollment(ModuleKind::CommsPod, POD, POD_PORT, &POD_SEED),
                        enrollment(ModuleKind::DcMeter, METER, METER_PORT, &METER_SEED),
                    ],
                    writes: WriteMap::new([write_entry(HazardClass::Class2)]).unwrap(),
                    buses: BusFacts::new([bus_fact()]).unwrap(),
                },
                Box::new(Counter(0)),
            )
            .unwrap();
            let mut hub = SimHub::default();
            hub.links.insert(POD_PORT, SimLink::new(pod(1), pod_line));
            hub.links.insert(METER_PORT, SimLink::new(meter(2), None));
            Bench {
                s,
                hub,
                interlock: FakeInterlock::default(),
            }
        }

        pub fn now(&self) -> Tick {
            self.hub.now
        }

        pub fn tick(&mut self) -> Vec<Happened> {
            let now = self.hub.now;
            for link in self.hub.links.values_mut() {
                link.pump(now);
            }
            self.interlock.tick(now);
            let out = self.s.step(now, &mut self.hub, &mut self.interlock);
            self.hub.now += 1;
            out
        }

        pub fn run(&mut self, ticks: u64) -> Vec<Happened> {
            (0..ticks).flat_map(|_| self.tick()).collect()
        }
    }

    pub fn scenario(name: &'static str, dir: Dir, from: Tick, fault: Fault) -> Scenario {
        Scenario::new(name, 0x5eed_0100, dir, from, fault)
    }
}

fn sim_quarantined(out: &[Happened]) -> Vec<QuarantineReason> {
    out.iter()
        .filter_map(|h| match h {
            Happened::Quarantined { reason, .. } => Some(*reason),
            _ => None,
        })
        .collect()
}

#[test]
fn redteam_hw_032_a_second_device_under_an_enrolled_id_is_never_trusted() {
    use sawbones_module_protocol::IdentityError;
    use sawbones_sim_modules::{Dir, Fault, ModuleKind, SimLink, SimModule};
    use sawbones_supervisor::{PortRefusal, PortState};

    // Swapped in on the pod's port: a meter with its own key, naming the pod.
    let mut b = sim::Bench::new(None);
    let impostor = SimModule::new(ModuleKind::DcMeter, sim::METER, sim::METER_SEED, 7);
    let line = sim::scenario(
        "claim_other_id",
        Dir::ToHost,
        0,
        Fault::ClaimOtherId { as_id: sim::POD },
    );
    b.hub
        .links
        .insert(sim::POD_PORT, SimLink::new(impostor, Some(line)));
    let out = b.run(3);
    assert!(sim_quarantined(&out).is_empty());
    assert_eq!(
        b.s.port_state(sim::POD_PORT),
        Some(PortState::Refused(PortRefusal::BadIdentity(
            IdentityError::BadSignature
        )))
    );
    assert_ne!(b.s.module_state(sim::POD), Some(ModuleState::Available));

    // A second device holding the enrolled key, on another port: both are
    // quarantined until a person re-enrolls the module.
    let mut b = sim::Bench::new(None);
    b.run(3);
    assert_eq!(b.s.module_state(sim::POD), Some(ModuleState::Available));
    b.hub
        .links
        .insert(sim::SPARE_PORT, SimLink::new(sim::pod(99), None));
    let out = b.run(3);
    assert_eq!(
        sim_quarantined(&out),
        vec![QuarantineReason::DuplicateId {
            first: sim::POD_PORT,
            second: sim::SPARE_PORT
        }]
    );
    // The genuine pod was told to go to its safe state.
    assert!(b.hub.links[&sim::POD_PORT].module.session().is_some());
    assert!(b.hub.links[&sim::POD_PORT].module.safe_entries >= 3);
    b.hub.links.remove(&sim::SPARE_PORT);
    b.run(3 * POLL_PERIOD_TICKS);
    assert!(matches!(
        b.s.module_state(sim::POD),
        Some(ModuleState::Quarantined(_))
    ));
}

#[test]
fn redteam_hw_033_a_flooding_module_is_quarantined() {
    use sawbones_sim_modules::{Dir, Fault};
    let line = sim::scenario(
        "babble_to_host",
        Dir::ToHost,
        6,
        Fault::Babble { per_tick: 40 },
    );
    let mut b = sim::Bench::new(Some(line));
    b.run(5);
    let now = b.now();
    record_facts(&mut b.s, HazardClass::Class2, now);
    assert!(b.s.preflight(&write(10), b.now()).is_ok());
    let mut out = Vec::new();
    for _ in 0..POLL_PERIOD_TICKS {
        out.extend(b.tick());
    }
    assert!(matches!(
        sim_quarantined(&out)[..],
        [QuarantineReason::Flooding { .. }]
    ));
    assert!(b.s.preflight(&write(10), b.now()).is_err());
    // The healthy meter is untouched.
    assert_eq!(b.s.module_state(sim::METER), Some(ModuleState::Available));
}

#[test]
fn redteam_hw_035_a_silent_or_stuck_line_loses_the_module_and_its_facts() {
    use sawbones_sim_modules::{Dir, Fault};
    for fault in [Fault::Silent, Fault::StuckLine { level: 0xff }] {
        let line = sim::scenario("silent_or_stuck", Dir::ToHost, 6, fault);
        let mut b = sim::Bench::new(Some(line));
        b.run(5);
        let now = b.now();
        record_facts(&mut b.s, HazardClass::Class2, now);
        assert!(b.s.preflight(&write(10), b.now()).is_ok());
        let out = b.run(2 * POLL_PERIOD_TICKS + sawbones_supervisor::LINK_TIMEOUT_TICKS);
        assert!(
            out.contains(&Happened::Lost {
                module: sim::POD,
                port: sim::POD_PORT
            }),
            "{fault:?}"
        );
        assert!(b.s.preflight(&write(10), b.now()).is_err(), "{fault:?}");
        // It stays lost: every identify on the dead line times out.
        b.run(5 * POLL_PERIOD_TICKS);
        assert_eq!(b.s.module_state(sim::POD), Some(ModuleState::Lost));
    }
}

#[test]
fn redteam_hw_031_repeated_identify_replays_quarantine_the_module() {
    let mut b = sim::Bench::new(None);
    let mut out = Vec::new();
    for _ in 0..sawbones_supervisor::MAX_LOSSES {
        while b.s.module_state(sim::POD) != Some(ModuleState::Available) {
            out.extend(b.tick());
        }
        b.hub.replay_identify(sim::POD_PORT);
        while b.s.module_state(sim::POD) == Some(ModuleState::Available) {
            out.extend(b.tick());
        }
    }
    assert_eq!(
        sim_quarantined(&out),
        vec![QuarantineReason::RepeatedLoss {
            losses: sawbones_supervisor::MAX_LOSSES
        }]
    );
    assert!(b.now() < sawbones_supervisor::LOSS_WINDOW_TICKS);
}
