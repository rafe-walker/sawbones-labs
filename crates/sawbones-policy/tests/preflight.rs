//! Fact scope, invalidation and the write preflight (#110).
//!
//! Every invalidation trigger has a test that shows a write that passed
//! preflight before the event is refused after it, naming the fact.

mod common;

use std::collections::BTreeSet;

use common::*;
use sawbones_policy::{
    confirm, preflight, relied_facts, BusFact, BusFacts, ChainHead, DeviceIdentity, EffectRequest,
    EventRefusal, FactContent, FactEvent, FactKey, FactLedger, FactProblem, FactRecord, FactScope,
    HazardClass, Interface, InvalidationCause, LedgerId, Mastership, Method, PreflightRefusal,
    PreflightToken, RecordError, Reference, RefusalReason, ScopeField, ScopeLabel, ScopeName,
    SettingKind, Tick, TokenProblem, Uncertainty, Validity, ValueRange, WriteEntry, WriteMap,
    WriteValue, PREFLIGHT_TTL_TICKS,
};
use sawbones_reading::Trust;

const PORT: &str = "rs485-1";
const OTHER_PORT: &str = "rs485-2";
const HEAD: ChainHead = ChainHead([0xAB; 32]);
const MASTERSHIP_TICKS: Tick = 10_000;

fn name(s: &str) -> ScopeName {
    ScopeName::parse(s).unwrap()
}
fn label(s: &str) -> ScopeLabel {
    ScopeLabel::parse(s).unwrap()
}

fn inverter(firmware: &str) -> DeviceIdentity {
    DeviceIdentity {
        model: name("inverter-a"),
        serial: label("SN0001"),
        firmware: label(firmware),
    }
}

fn measured_by(module: &str, firmware: &str) -> Method {
    Method::Measured {
        instrument: name(module),
        instrument_firmware: label(firmware),
        procedure: name("register-probe"),
        uncertainty: Uncertainty {
            plus_minus: 0,
            unit: name("count"),
        },
    }
}

/// The signals of the bench, with how each entry was earned.
fn charge_voltage() -> WriteEntry {
    entry(
        PORT,
        "charge_voltage",
        HazardClass::Class2,
        SettingKind::ChargeVoltage,
        true,
        Trust::OperatorAttested,
    )
}
fn low_cutoff() -> WriteEntry {
    entry(
        PORT,
        "battery_low_cutoff",
        HazardClass::Class3,
        SettingKind::BatteryProtectionLimit,
        true,
        Trust::Measured,
    )
}
fn fan_mode() -> WriteEntry {
    entry(
        PORT,
        "fan_mode",
        HazardClass::Class2,
        SettingKind::Config,
        false,
        Trust::OperatorAttested,
    )
}
fn mains() -> WriteEntry {
    entry(
        PORT,
        "mains_relay",
        HazardClass::Class2,
        SettingKind::MainsSwitching,
        true,
        Trust::OperatorAttested,
    )
}
fn mastership() -> BusFact {
    fact(PORT, Mastership::BenchOnly, Trust::OperatorAttested)
}

fn cv_write() -> EffectRequest {
    write(PORT, "charge_voltage", WriteValue::U16(5600))
}
fn cutoff_write() -> EffectRequest {
    write(PORT, "battery_low_cutoff", WriteValue::U16(4600))
}
fn fan_write() -> EffectRequest {
    write(PORT, "fan_mode", WriteValue::U16(2))
}
fn port_read() -> EffectRequest {
    read(PORT, "charge_voltage")
}

fn cv_key() -> FactKey {
    key_of(&cv_write())
}
fn cutoff_key() -> FactKey {
    key_of(&cutoff_write())
}
fn fan_key() -> FactKey {
    key_of(&fan_write())
}
fn mastership_key() -> FactKey {
    FactKey::Mastership { port: port(PORT) }
}
fn key_of(request: &EffectRequest) -> FactKey {
    relied_facts(request).remove(0)
}

/// A bench with every fact recorded and in scope.
struct Bench {
    writes: WriteMap,
    buses: BusFacts,
    ledger: FactLedger,
    now: Tick,
}

impl Bench {
    fn new() -> Self {
        let writes = WriteMap::new([charge_voltage(), low_cutoff(), fan_mode(), mains()]).unwrap();
        let buses = BusFacts::new([mastership()]).unwrap();
        let mut bench = Self {
            writes,
            buses,
            ledger: FactLedger::new(LedgerId(42)),
            now: 1,
        };
        bench.event(FactEvent::DeviceIdentified {
            port: port(PORT),
            unit: UNIT,
            identity: inverter("V1.0"),
        });
        bench.event(FactEvent::ModuleQualified {
            module: name("meter-a"),
            firmware: label("M1"),
            reference: Reference::Standard(name("voltage-ref")),
        });
        bench.event(FactEvent::ConditionChanged {
            condition: name("bank_disconnected"),
            holds: true,
        });
        for e in [charge_voltage(), mains()] {
            bench
                .record_entry(e, Method::OperatorAttested, &[])
                .unwrap();
        }
        bench
            .record_entry(low_cutoff(), measured_by("meter-a", "M1"), &[])
            .unwrap();
        bench
            .record_entry(fan_mode(), Method::OperatorAttested, &["bank_disconnected"])
            .unwrap();
        bench.record_mastership(mastership()).unwrap();
        bench
    }

    fn tick(&mut self) -> Tick {
        self.now += 1;
        self.now
    }

    fn event(&mut self, event: FactEvent) -> Vec<FactKey> {
        let now = self.tick();
        self.ledger.apply(event, now).unwrap().invalidated
    }

    fn entry_record(&self, e: WriteEntry, method: Method, conditions: &[&str]) -> FactRecord {
        let identity = self.ledger.device_at(&e.port, e.unit).cloned();
        FactRecord {
            scope: FactScope {
                device: identity.or_else(|| Some(inverter("V1.0"))),
                interface: Interface {
                    port: e.port.clone(),
                    bench_port: bench_port_of(e.port.as_str()),
                    unit: Some(e.unit),
                },
                conditions: conditions.iter().map(|c| name(c)).collect(),
                method,
            },
            content: FactContent::WriteEntry(e),
            earned_at: self.now,
            validity: Validity::UntilInvalidated,
        }
    }

    fn record_entry(
        &mut self,
        e: WriteEntry,
        method: Method,
        conditions: &[&str],
    ) -> Result<(), RecordError> {
        let record = self.entry_record(e, method, conditions);
        let now = self.tick();
        self.ledger.record(record, now).map(|_| ())
    }

    fn mastership_record(&self, b: BusFact) -> FactRecord {
        FactRecord {
            scope: FactScope {
                device: None,
                interface: Interface {
                    port: b.port.clone(),
                    bench_port: b.bench_port,
                    unit: None,
                },
                conditions: BTreeSet::new(),
                method: Method::OperatorAttested,
            },
            content: FactContent::Mastership(b),
            earned_at: self.now,
            validity: Validity::For {
                ticks: MASTERSHIP_TICKS,
            },
        }
    }

    fn record_mastership(&mut self, b: BusFact) -> Result<(), RecordError> {
        let record = self.mastership_record(b);
        let now = self.tick();
        self.ledger.record(record, now).map(|_| ())
    }

    fn check(&self, request: &EffectRequest) -> Result<PreflightToken, PreflightRefusal> {
        preflight(
            request,
            &self.writes,
            &self.buses,
            &self.ledger,
            HEAD,
            self.now,
        )
    }

    fn passes(&self, request: &EffectRequest) {
        if let Err(e) = self.check(request) {
            panic!("{request:?} should pass, refused: {e}");
        }
    }

    fn refused(&self, request: &EffectRequest) -> (FactKey, FactProblem) {
        match self.check(request) {
            Err(PreflightRefusal::Fact { fact, problem }) => (fact, problem),
            other => panic!("{request:?} should be refused for a fact, got {other:?}"),
        }
    }

    fn refused_invalidated(&self, request: &EffectRequest, fact: &FactKey) -> InvalidationCause {
        match self.refused(request) {
            (f, FactProblem::Invalidated { by }) if &f == fact => by,
            other => panic!("{request:?}: expected {fact} invalidated, got {other:?}"),
        }
    }
}

fn writes_of_unit_one() -> [EffectRequest; 3] {
    [cv_write(), cutoff_write(), fan_write()]
}

#[test]
fn a_scoped_bench_passes_and_confirms() {
    let bench = Bench::new();
    for request in writes_of_unit_one() {
        let token = bench.check(&request).unwrap();
        let keys: Vec<FactKey> = token.facts().iter().map(|(k, _)| k.clone()).collect();
        assert_eq!(keys, relied_facts(&request));
        for (key, version) in token.facts() {
            assert_eq!(bench.ledger.version(key), Some(*version));
        }
        assert_eq!(token.chain_head(), HEAD);
        assert_eq!(token.generation(), bench.ledger.generation());
        let class = token.decision().class();
        assert_eq!(
            token.action_digest().is_some(),
            class == HazardClass::Class3,
            "{request:?}"
        );
        let confirmed = confirm(
            token,
            &request,
            &bench.writes,
            &bench.buses,
            &bench.ledger,
            HEAD,
            bench.now,
        )
        .unwrap();
        assert_eq!(confirmed.decision().class(), class);
    }
    // Battery protection limits and charge voltages are class 3.
    for request in [cv_write(), cutoff_write()] {
        assert_eq!(
            bench.check(&request).unwrap().decision().class(),
            HazardClass::Class3
        );
    }
    let read = bench.check(&port_read()).unwrap();
    assert_eq!(read.facts().len(), 1);
    assert_eq!(read.decision().class(), HazardClass::Class2);
}

#[test]
fn listening_relies_on_no_fact() {
    let ledger = FactLedger::new(LedgerId(1));
    let token = preflight(
        &listen(PORT),
        &WriteMap::default(),
        &BusFacts::default(),
        &ledger,
        HEAD,
        0,
    )
    .unwrap();
    assert!(token.facts().is_empty());
    assert_eq!(token.decision().class(), HazardClass::Class0);
}

#[test]
fn an_unrecorded_fact_is_refused_as_missing() {
    // The write map and bus facts alone, with no ledger record, are not
    // enough: the write is refused naming the entry.
    let bench = Bench::new();
    let empty = FactLedger::new(LedgerId(42));
    for request in writes_of_unit_one() {
        let refusal = preflight(&request, &bench.writes, &bench.buses, &empty, HEAD, 5);
        assert_eq!(
            refusal.unwrap_err(),
            PreflightRefusal::Fact {
                fact: key_of(&request),
                problem: FactProblem::Missing
            }
        );
    }
    let refusal = preflight(&port_read(), &bench.writes, &bench.buses, &empty, HEAD, 5);
    assert_eq!(refusal.unwrap_err().fact(), Some(&mastership_key()));
}

// --- One test per invalidation trigger. ---

#[test]
fn a_firmware_change_refuses_every_dependent_write() {
    let mut bench = Bench::new();
    for request in writes_of_unit_one() {
        bench.passes(&request);
    }
    let invalidated = bench.event(FactEvent::DeviceIdentified {
        port: port(PORT),
        unit: UNIT,
        identity: inverter("V1.1"),
    });
    assert!(invalidated.contains(&cv_key()) && invalidated.contains(&cutoff_key()));
    for request in writes_of_unit_one() {
        let cause = bench.refused_invalidated(&request, &key_of(&request));
        assert_eq!(
            cause,
            InvalidationCause::DeviceChanged {
                port: port(PORT),
                unit: UNIT
            }
        );
    }
    // Mastership is about the bus, not the inverter: reads still pass.
    bench.passes(&port_read());
    // Reverting the firmware does not bring the facts back.
    bench.event(FactEvent::DeviceIdentified {
        port: port(PORT),
        unit: UNIT,
        identity: inverter("V1.0"),
    });
    assert!(matches!(
        bench.refused(&cv_write()).1,
        FactProblem::Invalidated { .. }
    ));
    // Recording the fact again, on the device as it is now, does.
    bench
        .record_entry(charge_voltage(), Method::OperatorAttested, &[])
        .unwrap();
    bench.passes(&cv_write());
}

#[test]
fn a_serial_or_model_change_refuses_the_write() {
    for change in ["serial", "model"] {
        let mut bench = Bench::new();
        bench.passes(&cv_write());
        let mut identity = inverter("V1.0");
        match change {
            "serial" => identity.serial = label("SN0002"),
            _ => identity.model = name("inverter-b"),
        }
        bench.event(FactEvent::DeviceIdentified {
            port: port(PORT),
            unit: UNIT,
            identity,
        });
        bench.refused_invalidated(&cv_write(), &cv_key());
    }
}

#[test]
fn identifying_the_same_device_again_changes_nothing() {
    let mut bench = Bench::new();
    let generation = bench.ledger.generation();
    let invalidated = bench.event(FactEvent::DeviceIdentified {
        port: port(PORT),
        unit: UNIT,
        identity: inverter("V1.0"),
    });
    assert!(invalidated.is_empty());
    assert_eq!(bench.ledger.generation(), generation);
    bench.passes(&cv_write());
}

#[test]
fn a_lost_device_refuses_the_write() {
    let mut bench = Bench::new();
    bench.passes(&cv_write());
    bench.event(FactEvent::DeviceLost {
        port: port(PORT),
        unit: UNIT,
    });
    let cause = bench.refused_invalidated(&cv_write(), &cv_key());
    assert_eq!(
        cause,
        InvalidationCause::DeviceLost {
            port: port(PORT),
            unit: UNIT
        }
    );
    assert!(bench.ledger.device_at(&port(PORT), UNIT).is_none());
}

#[test]
fn a_device_that_moves_refuses_writes_at_its_old_place() {
    let mut bench = Bench::new();
    bench.passes(&cv_write());
    // The same model and serial now answers as unit 2.
    bench.event(FactEvent::DeviceIdentified {
        port: port(PORT),
        unit: 2,
        identity: inverter("V1.0"),
    });
    bench.refused_invalidated(&cv_write(), &cv_key());
    assert!(bench.ledger.device_at(&port(PORT), UNIT).is_none());
}

#[test]
fn a_rewired_port_refuses_writes_and_reads() {
    let mut bench = Bench::new();
    bench.passes(&cv_write());
    bench.passes(&port_read());
    bench.event(FactEvent::PortRewired { port: port(PORT) });
    let cause = InvalidationCause::PortRewired { port: port(PORT) };
    assert_eq!(bench.refused_invalidated(&cv_write(), &cv_key()), cause);
    assert_eq!(
        bench.refused_invalidated(&port_read(), &mastership_key()),
        cause
    );
    assert!(bench.ledger.device_at(&port(PORT), UNIT).is_none());
}

#[test]
fn rewiring_another_port_leaves_this_one_alone() {
    let mut bench = Bench::new();
    bench.event(FactEvent::PortRewired {
        port: port(OTHER_PORT),
    });
    for request in writes_of_unit_one() {
        bench.passes(&request);
    }
}

#[test]
fn a_foreign_master_refuses_writes_and_reads_on_its_port() {
    let mut bench = Bench::new();
    bench.passes(&cv_write());
    bench.passes(&port_read());
    bench.event(FactEvent::ForeignMasterObserved { port: port(PORT) });
    let cause = InvalidationCause::ForeignMaster { port: port(PORT) };
    // The write-map entry still holds; the mastership fact does not.
    assert_eq!(
        bench.refused_invalidated(&cv_write(), &mastership_key()),
        cause
    );
    assert_eq!(
        bench.refused_invalidated(&port_read(), &mastership_key()),
        cause
    );
}

#[test]
fn a_reflashed_instrument_invalidates_what_it_measured_until_it_re_earns_trust() {
    let mut bench = Bench::new();
    bench.passes(&cutoff_write());
    bench.event(FactEvent::ModuleReflashed {
        module: name("meter-a"),
        firmware: label("M2"),
    });
    let cause = bench.refused_invalidated(&cutoff_write(), &cutoff_key());
    assert_eq!(
        cause,
        InvalidationCause::InstrumentReflashed {
            module: name("meter-a")
        }
    );
    // An attested fact did not come from the meter and still holds.
    bench.passes(&cv_write());
    assert!(!bench.ledger.is_qualified(&name("meter-a")));

    // Nothing it measures counts until it is qualified again.
    let err = bench
        .record_entry(low_cutoff(), measured_by("meter-a", "M2"), &[])
        .unwrap_err();
    assert_eq!(
        err,
        RecordError::Fact {
            problem: FactProblem::InstrumentNotQualified {
                module: name("meter-a")
            }
        }
    );
    // It cannot be its own reference, nor claim another firmware.
    let now = bench.tick();
    assert_eq!(
        bench.ledger.apply(
            FactEvent::ModuleQualified {
                module: name("meter-a"),
                firmware: label("M2"),
                reference: Reference::Module(name("meter-a")),
            },
            now,
        ),
        Err(EventRefusal::SelfReference)
    );
    assert_eq!(
        bench.ledger.apply(
            FactEvent::ModuleQualified {
                module: name("meter-a"),
                firmware: label("M1"),
                reference: Reference::Standard(name("voltage-ref")),
            },
            now,
        ),
        Err(EventRefusal::FirmwareMismatch {
            module: name("meter-a")
        })
    );
    bench.event(FactEvent::ModuleQualified {
        module: name("meter-a"),
        firmware: label("M2"),
        reference: Reference::Standard(name("voltage-ref")),
    });
    // Qualifying again does not restore what it measured before.
    bench.refused_invalidated(&cutoff_write(), &cutoff_key());
    // A measurement under the old firmware is refused.
    assert_eq!(
        bench
            .record_entry(low_cutoff(), measured_by("meter-a", "M1"), &[])
            .unwrap_err(),
        RecordError::Fact {
            problem: FactProblem::InstrumentFirmwareMismatch {
                module: name("meter-a")
            }
        }
    );
    // Measured again, it holds.
    bench
        .record_entry(low_cutoff(), measured_by("meter-a", "M2"), &[])
        .unwrap();
    bench.passes(&cutoff_write());
}

#[test]
fn a_reflash_falls_through_every_module_qualified_against_it() {
    let mut bench = Bench::new();
    // meter-b was checked against meter-a, and meter-c against meter-b.
    for (module, against) in [("meter-b", "meter-a"), ("meter-c", "meter-b")] {
        bench.event(FactEvent::ModuleQualified {
            module: name(module),
            firmware: label("M1"),
            reference: Reference::Module(name(against)),
        });
    }
    bench
        .record_entry(low_cutoff(), measured_by("meter-c", "M1"), &[])
        .unwrap();
    bench.passes(&cutoff_write());
    bench.event(FactEvent::ModuleReflashed {
        module: name("meter-a"),
        firmware: label("M2"),
    });
    assert_eq!(
        bench.refused_invalidated(&cutoff_write(), &cutoff_key()),
        InvalidationCause::InstrumentReflashed {
            module: name("meter-a")
        }
    );
    for m in ["meter-a", "meter-b", "meter-c"] {
        assert!(!bench.ledger.is_qualified(&name(m)), "{m}");
    }
    // meter-b cannot be qualified against the now unqualified meter-a.
    let now = bench.tick();
    assert_eq!(
        bench.ledger.apply(
            FactEvent::ModuleQualified {
                module: name("meter-b"),
                firmware: label("M1"),
                reference: Reference::Module(name("meter-a")),
            },
            now,
        ),
        Err(EventRefusal::ReferenceNotQualified {
            reference: name("meter-a")
        })
    );
}

#[test]
fn a_condition_that_ends_refuses_the_write_and_stays_refused() {
    let mut bench = Bench::new();
    bench.passes(&fan_write());
    bench.event(FactEvent::ConditionChanged {
        condition: name("bank_disconnected"),
        holds: false,
    });
    assert_eq!(
        bench.refused_invalidated(&fan_write(), &fan_key()),
        InvalidationCause::ConditionEnded {
            condition: name("bank_disconnected")
        }
    );
    // Facts not earned under it still hold.
    bench.passes(&cv_write());
    // Recording under a condition that does not hold is refused.
    assert_eq!(
        bench
            .record_entry(fan_mode(), Method::OperatorAttested, &["bank_disconnected"])
            .unwrap_err(),
        RecordError::Fact {
            problem: FactProblem::ConditionNotHolding {
                condition: name("bank_disconnected")
            }
        }
    );
    bench.event(FactEvent::ConditionChanged {
        condition: name("bank_disconnected"),
        holds: true,
    });
    bench.refused_invalidated(&fan_write(), &fan_key());
    bench
        .record_entry(fan_mode(), Method::OperatorAttested, &["bank_disconnected"])
        .unwrap();
    bench.passes(&fan_write());
}

#[test]
fn an_open_contradiction_refuses_until_the_fact_is_recorded_again() {
    let mut bench = Bench::new();
    bench.passes(&cv_write());
    bench.event(FactEvent::ContradictionOpened { fact: cv_key() });
    assert_eq!(
        bench.refused(&cv_write()),
        (cv_key(), FactProblem::OpenContradiction)
    );
    bench
        .record_entry(charge_voltage(), Method::OperatorAttested, &[])
        .unwrap();
    assert!(!bench.ledger.is_contradicted(&cv_key()));
    bench.passes(&cv_write());
}

#[test]
fn a_clock_discontinuity_refuses_everything_and_forgets_the_bench() {
    let mut bench = Bench::new();
    bench.passes(&cv_write());
    bench.event(FactEvent::ClockDiscontinuity);
    for request in writes_of_unit_one() {
        assert_eq!(
            bench.refused_invalidated(&request, &key_of(&request)),
            InvalidationCause::ClockDiscontinuity
        );
    }
    assert_eq!(
        bench.refused_invalidated(&port_read(), &mastership_key()),
        InvalidationCause::ClockDiscontinuity
    );
    assert!(bench.ledger.device_at(&port(PORT), UNIT).is_none());
    assert!(!bench.ledger.is_qualified(&name("meter-a")));
    assert!(!bench.ledger.condition_holds(&name("bank_disconnected")));
}

#[test]
fn a_tick_that_goes_backwards_is_a_clock_discontinuity() {
    let mut bench = Bench::new();
    bench.passes(&cv_write());
    let back = bench.now - 1;
    // Even an event that changes nothing else invalidates every fact.
    bench
        .ledger
        .apply(
            FactEvent::ConditionChanged {
                condition: name("unrelated"),
                holds: true,
            },
            back,
        )
        .unwrap();
    bench.now = back;
    bench.refused_invalidated(&cv_write(), &cv_key());

    // A record with a tick behind the ledger is refused, and is itself a
    // discontinuity.
    let mut bench = Bench::new();
    let record = bench.entry_record(charge_voltage(), Method::OperatorAttested, &[]);
    let back = bench.now - 1;
    assert_eq!(
        bench.ledger.record(record, back),
        Err(RecordError::ClockWentBackwards)
    );
    bench.now = back;
    bench.refused_invalidated(&cv_write(), &cv_key());
}

#[test]
fn a_preflight_with_a_tick_behind_the_ledger_is_refused() {
    let mut bench = Bench::new();
    bench.passes(&cv_write());
    bench.now -= 1;
    assert_eq!(
        bench.refused(&cv_write()),
        (cv_key(), FactProblem::ClockBehindLedger)
    );
}

#[test]
fn a_mastership_fact_expires() {
    let mut bench = Bench::new();
    let earned = bench.now - 1;
    bench.now = earned + MASTERSHIP_TICKS - 1;
    bench.passes(&cv_write());
    bench.passes(&port_read());
    bench.now = earned + MASTERSHIP_TICKS;
    for request in [cv_write(), port_read()] {
        assert_eq!(
            bench.refused(&request),
            (
                mastership_key(),
                FactProblem::Expired {
                    earned_at: earned,
                    valid_for: MASTERSHIP_TICKS
                }
            )
        );
    }
}

// --- Scope and content. ---

#[test]
fn an_edited_write_map_entry_no_longer_matches_its_fact() {
    let mut bench = Bench::new();
    bench.passes(&cv_write());
    let mut moved = charge_voltage();
    moved.register = moved.register.wrapping_add(1);
    bench.writes = WriteMap::new([moved, low_cutoff(), fan_mode()]).unwrap();
    assert_eq!(
        bench.refused(&cv_write()),
        (cv_key(), FactProblem::ContentMismatch)
    );
    // Raising its tier is an edit too.
    let mut raised = charge_voltage();
    raised.trust = Trust::Measured;
    bench.writes = WriteMap::new([raised]).unwrap();
    assert_eq!(
        bench.refused(&cv_write()),
        (cv_key(), FactProblem::ContentMismatch)
    );
}

#[test]
fn a_port_wired_to_another_bench_port_is_out_of_scope() {
    let mut bench = Bench::new();
    let mut moved = mastership();
    moved.bench_port = moved.bench_port.wrapping_add(1);
    bench.buses = BusFacts::new([moved]).unwrap();
    assert_eq!(
        bench.refused(&cv_write()),
        (
            cv_key(),
            FactProblem::OutOfScope {
                field: ScopeField::BenchPort
            }
        )
    );
    assert_eq!(
        bench.refused(&port_read()),
        (mastership_key(), FactProblem::ContentMismatch)
    );
}

#[test]
fn a_write_to_another_unit_has_no_fact() {
    let bench = Bench::new();
    let EffectRequest::BusWrite(mut w) = cv_write() else {
        unreachable!()
    };
    w.unit = 2;
    let request = EffectRequest::BusWrite(w);
    assert_eq!(
        bench.refused(&request),
        (key_of(&request), FactProblem::Missing)
    );
}

#[test]
fn a_record_must_be_scoped_to_its_own_content_and_the_bench() {
    let mut bench = Bench::new();
    let base = bench.entry_record(charge_voltage(), Method::OperatorAttested, &[]);
    let now = bench.now + 1;
    let check = |ledger: &mut FactLedger, record: FactRecord| ledger.record(record, now);

    let mut r = base.clone();
    r.scope.interface.port = port(OTHER_PORT);
    assert_eq!(
        check(&mut bench.ledger, r),
        Err(RecordError::ScopeDoesNotMatchContent {
            field: ScopeField::Port
        })
    );
    let mut r = base.clone();
    r.scope.interface.unit = Some(2);
    assert_eq!(
        check(&mut bench.ledger, r),
        Err(RecordError::ScopeDoesNotMatchContent {
            field: ScopeField::Unit
        })
    );
    let mut r = base.clone();
    r.scope.device = None;
    assert_eq!(
        check(&mut bench.ledger, r),
        Err(RecordError::ScopeDoesNotMatchContent {
            field: ScopeField::DeviceNotStated
        })
    );
    for (field, edit) in [
        (ScopeField::DeviceFirmware, 0),
        (ScopeField::DeviceSerial, 1),
        (ScopeField::DeviceModel, 2),
    ] {
        let mut r = base.clone();
        let d = r.scope.device.as_mut().unwrap();
        match edit {
            0 => d.firmware = label("V9"),
            1 => d.serial = label("SN9"),
            _ => d.model = name("other"),
        }
        assert_eq!(
            check(&mut bench.ledger, r),
            Err(RecordError::Fact {
                problem: FactProblem::OutOfScope { field }
            })
        );
    }
    // A tier and a method that disagree.
    let mut r = base.clone();
    r.scope.method = measured_by("meter-a", "M1");
    assert_eq!(
        check(&mut bench.ledger, r),
        Err(RecordError::Fact {
            problem: FactProblem::MethodDoesNotMatchTier {
                trust: Trust::OperatorAttested
            }
        })
    );
    let r = bench.entry_record(low_cutoff(), Method::OperatorAttested, &[]);
    assert_eq!(
        check(&mut bench.ledger, r),
        Err(RecordError::Fact {
            problem: FactProblem::MethodDoesNotMatchTier {
                trust: Trust::Measured
            }
        })
    );
    // A tier below operator-attested.
    let mut documented = charge_voltage();
    documented.trust = Trust::Documented;
    let r = bench.entry_record(documented, Method::OperatorAttested, &[]);
    assert_eq!(
        check(&mut bench.ledger, r),
        Err(RecordError::Fact {
            problem: FactProblem::NotActionable {
                trust: Trust::Documented
            }
        })
    );
    // Earned in the future.
    let mut r = base.clone();
    r.earned_at = now + 1;
    assert_eq!(
        check(&mut bench.ledger, r),
        Err(RecordError::Fact {
            problem: FactProblem::EarnedInFuture
        })
    );
    // Mastership must expire, and is about the bus, not a unit or device.
    let mut m = bench.mastership_record(mastership());
    m.validity = Validity::UntilInvalidated;
    assert_eq!(
        check(&mut bench.ledger, m),
        Err(RecordError::Fact {
            problem: FactProblem::MastershipNeedsExpiry
        })
    );
    let mut m = bench.mastership_record(mastership());
    m.scope.device = Some(inverter("V1.0"));
    assert_eq!(
        check(&mut bench.ledger, m),
        Err(RecordError::ScopeDoesNotMatchContent {
            field: ScopeField::DeviceNotExpected
        })
    );
    let mut m = bench.mastership_record(mastership());
    m.scope.interface.unit = Some(UNIT);
    assert_eq!(
        check(&mut bench.ledger, m),
        Err(RecordError::ScopeDoesNotMatchContent {
            field: ScopeField::Unit
        })
    );
}

#[test]
fn a_fact_earned_before_a_discontinuity_cannot_be_recorded_after_it() {
    let mut bench = Bench::new();
    let earned = bench.now;
    bench.event(FactEvent::ClockDiscontinuity);
    bench.event(FactEvent::DeviceIdentified {
        port: port(PORT),
        unit: UNIT,
        identity: inverter("V1.0"),
    });
    let mut r = bench.entry_record(charge_voltage(), Method::OperatorAttested, &[]);
    r.earned_at = earned;
    let now = bench.tick();
    assert_eq!(
        bench.ledger.record(r, now),
        Err(RecordError::Fact {
            problem: FactProblem::EarnedBeforeClockDiscontinuity
        })
    );
}

// --- The engine after the facts. ---

#[test]
fn the_engine_still_refuses_after_the_facts_hold() {
    let mut bench = Bench::new();
    let mains_write = write(PORT, "mains_relay", WriteValue::Bool(true));
    assert_eq!(
        bench.check(&mains_write).unwrap_err(),
        PreflightRefusal::Classified {
            reason: RefusalReason::ClassNeverPermitted
        }
    );
    let mut ranged = charge_voltage();
    ranged.range = Some(ValueRange::new(WriteValue::U16(5000), WriteValue::U16(5800)).unwrap());
    bench.writes = WriteMap::new([ranged.clone()]).unwrap();
    bench
        .record_entry(ranged, Method::OperatorAttested, &[])
        .unwrap();
    bench.passes(&cv_write());
    assert_eq!(
        bench
            .check(&write(PORT, "charge_voltage", WriteValue::U16(6000)))
            .unwrap_err(),
        PreflightRefusal::Classified {
            reason: RefusalReason::ValueOutOfRange
        }
    );
}

// --- The token at the moment of action. ---

fn confirm_at(
    bench: &Bench,
    token: PreflightToken,
    request: &EffectRequest,
    head: ChainHead,
    now: Tick,
) -> Result<(), PreflightRefusal> {
    confirm(
        token,
        request,
        &bench.writes,
        &bench.buses,
        &bench.ledger,
        head,
        now,
    )
    .map(|_| ())
}

fn token_problem(result: Result<(), PreflightRefusal>) -> TokenProblem {
    match result {
        Err(PreflightRefusal::Token { problem }) => problem,
        other => panic!("expected a token refusal, got {other:?}"),
    }
}

#[test]
fn any_change_to_the_ledger_voids_a_token() {
    let mut bench = Bench::new();
    let token = bench.check(&cv_write()).unwrap();
    // Even an event that touches no relied-on fact.
    bench.event(FactEvent::ConditionChanged {
        condition: name("lamp_on"),
        holds: true,
    });
    let now = bench.now;
    assert_eq!(
        token_problem(confirm_at(&bench, token, &cv_write(), HEAD, now)),
        TokenProblem::LedgerChanged
    );
}

#[test]
fn a_firmware_change_after_preflight_voids_the_token() {
    let mut bench = Bench::new();
    let token = bench.check(&cutoff_write()).unwrap();
    bench.event(FactEvent::DeviceIdentified {
        port: port(PORT),
        unit: UNIT,
        identity: inverter("V1.1"),
    });
    let now = bench.now;
    assert_eq!(
        token_problem(confirm_at(&bench, token, &cutoff_write(), HEAD, now)),
        TokenProblem::LedgerChanged
    );
}

#[test]
fn a_token_is_for_one_request_one_ledger_one_head_and_a_short_time() {
    let bench = Bench::new();
    let now = bench.now;
    let fresh = || bench.check(&cv_write()).unwrap();

    let other = write(PORT, "charge_voltage", WriteValue::U16(5601));
    assert_eq!(
        token_problem(confirm_at(&bench, fresh(), &other, HEAD, now)),
        TokenProblem::OtherRequest
    );
    assert_eq!(
        token_problem(confirm_at(
            &bench,
            fresh(),
            &cv_write(),
            ChainHead([0; 32]),
            now
        )),
        TokenProblem::ChainHeadMoved
    );
    assert_eq!(
        token_problem(confirm_at(&bench, fresh(), &cv_write(), HEAD, now - 1)),
        TokenProblem::ClockBehindToken
    );
    let token = fresh();
    let expires = token.expires_at();
    assert_eq!(expires, now + PREFLIGHT_TTL_TICKS);
    assert_eq!(
        token_problem(confirm_at(&bench, token, &cv_write(), HEAD, expires)),
        TokenProblem::Expired
    );
    // One tick before expiry it is good.
    confirm_at(&bench, fresh(), &cv_write(), HEAD, expires - 1).unwrap();

    // A token from another ledger with the same history.
    let twin = Bench {
        ledger: FactLedger::new(LedgerId(43)),
        writes: bench.writes.clone(),
        buses: bench.buses.clone(),
        now,
    };
    assert_eq!(
        token_problem(confirm_at(&twin, fresh(), &cv_write(), HEAD, now)),
        TokenProblem::OtherLedger
    );
}

#[test]
fn a_write_map_edited_after_preflight_is_caught_at_confirm() {
    let mut bench = Bench::new();
    let token = bench.check(&cv_write()).unwrap();
    let mut moved = charge_voltage();
    moved.register ^= 0x0100;
    bench.writes = WriteMap::new([moved]).unwrap();
    let now = bench.now;
    assert_eq!(
        confirm_at(&bench, token, &cv_write(), HEAD, now),
        Err(PreflightRefusal::Fact {
            fact: cv_key(),
            problem: FactProblem::ContentMismatch
        })
    );
}

#[test]
fn a_token_expires_with_its_facts_inside_the_window() {
    // A mastership fact that expires between preflight and confirm.
    let mut bench = Bench::new();
    let earned = bench.now - 1;
    bench.now = earned + MASTERSHIP_TICKS - 2;
    let token = bench.check(&cv_write()).unwrap();
    assert_eq!(
        confirm_at(&bench, token, &cv_write(), HEAD, earned + MASTERSHIP_TICKS),
        Err(PreflightRefusal::Fact {
            fact: mastership_key(),
            problem: FactProblem::Expired {
                earned_at: earned,
                valid_for: MASTERSHIP_TICKS
            }
        })
    );
}

// --- Encodings. ---

#[test]
fn records_and_events_round_trip_through_json() {
    let bench = Bench::new();
    let record = bench.entry_record(low_cutoff(), measured_by("meter-a", "M1"), &["c1"]);
    let json = serde_json::to_string(&record).unwrap();
    assert_eq!(serde_json::from_str::<FactRecord>(&json).unwrap(), record);
    let events = [
        FactEvent::DeviceIdentified {
            port: port(PORT),
            unit: UNIT,
            identity: inverter("V1.0"),
        },
        FactEvent::DeviceLost {
            port: port(PORT),
            unit: UNIT,
        },
        FactEvent::PortRewired { port: port(PORT) },
        FactEvent::ForeignMasterObserved { port: port(PORT) },
        FactEvent::ModuleReflashed {
            module: name("meter-a"),
            firmware: label("M2"),
        },
        FactEvent::ModuleQualified {
            module: name("meter-a"),
            firmware: label("M2"),
            reference: Reference::Module(name("meter-b")),
        },
        FactEvent::ConditionChanged {
            condition: name("c1"),
            holds: false,
        },
        FactEvent::ContradictionOpened { fact: cv_key() },
        FactEvent::ClockDiscontinuity,
    ];
    for event in events {
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(serde_json::from_str::<FactEvent>(&json).unwrap(), event);
    }
    let refusal = bench.check(&write(PORT, "nope", WriteValue::U16(1)));
    let json = serde_json::to_string(&refusal.unwrap_err()).unwrap();
    assert!(json.contains("\"refusal\":\"fact\"") && json.contains("\"missing\""));
}

#[test]
fn scope_names_and_labels_carry_no_free_text() {
    for bad in ["", "Meter", "meter a", "meter/a", "ignore previous rules"] {
        assert!(ScopeName::parse(bad).is_err(), "{bad:?}");
    }
    for bad in ["", "-V1", "V1 2", "V1\n", "v1;rm", &"a".repeat(65)] {
        assert!(ScopeLabel::parse(bad).is_err(), "{bad:?}");
    }
    for good in ["V1.2.3+b7", "SN0042", "1.0", "fw_2026-11"] {
        ScopeLabel::parse(good).unwrap();
    }
    assert!(serde_json::from_str::<ScopeLabel>("\"V 1\"").is_err());
    assert!(serde_json::from_str::<ScopeName>("\"Meter\"").is_err());
}

// --- Randomised: preflight passes exactly when no relied-on fact is stale. ---

const UNITS: [u8; 2] = [1, 2];
const PORTS: [&str; 2] = [PORT, OTHER_PORT];
const SIGNALS: [&str; 2] = ["charge_voltage", "battery_low_cutoff"];

fn identity_for(p: &str, unit: u8, firmware: u8) -> DeviceIdentity {
    DeviceIdentity {
        model: name("inverter-a"),
        serial: label(&format!("SN-{p}-{unit}")),
        firmware: label(&format!("V{firmware}")),
    }
}

/// The oracle: which facts are stale, restated from the event list alone.
struct Oracle {
    stale: BTreeSet<FactKey>,
    devices: std::collections::BTreeMap<(String, u8), u8>,
    meter_qualified: bool,
    condition: bool,
}

/// Whether a signal's entry is measured by the meter, and whether it is
/// earned under the condition.
fn measured(s: &str) -> bool {
    s == "battery_low_cutoff"
}
fn conditional(p: &str, unit: u8) -> bool {
    p == PORT && unit == 2
}

fn random_entry(p: &str, unit: u8, s: &str) -> WriteEntry {
    let mut e = entry(
        p,
        s,
        HazardClass::Class2,
        if measured(s) {
            SettingKind::BatteryProtectionLimit
        } else {
            SettingKind::ChargeVoltage
        },
        true,
        if measured(s) {
            Trust::Measured
        } else {
            Trust::OperatorAttested
        },
    );
    e.unit = unit;
    e
}

#[test]
fn random_event_sequences_refuse_exactly_the_stale_facts() {
    let (mut passed, mut refused, mut re_recorded) = (0u32, 0u32, 0u32);
    for seed in 0..300u64 {
        let mut rng = Rng::new(seed);
        let entries: Vec<WriteEntry> = PORTS
            .iter()
            .flat_map(|p| {
                UNITS
                    .iter()
                    .flat_map(move |u| SIGNALS.iter().map(move |s| random_entry(p, *u, s)))
            })
            .collect();
        let mut bench = Bench {
            writes: WriteMap::new(entries.clone()).unwrap(),
            buses: BusFacts::new(
                PORTS
                    .iter()
                    .map(|p| fact(p, Mastership::BenchOnly, Trust::OperatorAttested)),
            )
            .unwrap(),
            ledger: FactLedger::new(LedgerId(seed)),
            now: 1,
        };
        let mut oracle = Oracle {
            stale: BTreeSet::new(),
            devices: Default::default(),
            meter_qualified: true,
            condition: true,
        };
        for p in PORTS {
            for u in UNITS {
                bench.event(FactEvent::DeviceIdentified {
                    port: port(p),
                    unit: u,
                    identity: identity_for(p, u, 1),
                });
                oracle.devices.insert((p.to_owned(), u), 1);
            }
        }
        bench.event(FactEvent::ModuleQualified {
            module: name("meter-a"),
            firmware: label("M1"),
            reference: Reference::Standard(name("voltage-ref")),
        });
        bench.event(FactEvent::ConditionChanged {
            condition: name("c1"),
            holds: true,
        });
        let mut meter_firmware = 1u8;
        let record_entry = |bench: &mut Bench, e: &WriteEntry, fw: u8| {
            let p = e.port.as_str().to_owned();
            let method = if measured(e.signal.as_str()) {
                measured_by("meter-a", &format!("M{fw}"))
            } else {
                Method::OperatorAttested
            };
            let conds: &[&str] = if conditional(&p, e.unit) {
                &["c1"]
            } else {
                &[]
            };
            bench.record_entry(e.clone(), method, conds)
        };
        for e in &entries {
            record_entry(&mut bench, e, meter_firmware).unwrap();
        }
        for p in PORTS {
            let b = fact(p, Mastership::BenchOnly, Trust::OperatorAttested);
            let mut r = bench.mastership_record(b);
            r.validity = Validity::For { ticks: u64::MAX };
            let now = bench.tick();
            bench.ledger.record(r, now).unwrap();
        }

        for step in 0..40 {
            let p = rng.pick(&PORTS);
            let u = rng.pick(&UNITS);
            let keys_at = |p: &str, u: u8| -> Vec<FactKey> {
                SIGNALS
                    .iter()
                    .map(|s| FactKey::WriteEntry {
                        port: port(p),
                        unit: u,
                        signal: signal(s),
                    })
                    .collect()
            };
            match rng.below(10) {
                0 => {
                    let fw = 1 + u8::try_from(rng.below(2)).unwrap();
                    bench.event(FactEvent::DeviceIdentified {
                        port: port(p),
                        unit: u,
                        identity: identity_for(p, u, fw),
                    });
                    let place = (p.to_owned(), u);
                    if oracle.devices.get(&place) != Some(&fw) {
                        oracle.stale.extend(keys_at(p, u));
                    }
                    oracle.devices.insert(place, fw);
                }
                1 => {
                    bench.event(FactEvent::DeviceLost {
                        port: port(p),
                        unit: u,
                    });
                    oracle.stale.extend(keys_at(p, u));
                    oracle.devices.remove(&(p.to_owned(), u));
                }
                2 => {
                    bench.event(FactEvent::PortRewired { port: port(p) });
                    for u in UNITS {
                        oracle.stale.extend(keys_at(p, u));
                        oracle.devices.remove(&(p.to_owned(), u));
                    }
                    oracle.stale.insert(FactKey::Mastership { port: port(p) });
                }
                3 => {
                    bench.event(FactEvent::ForeignMasterObserved { port: port(p) });
                    oracle.stale.insert(FactKey::Mastership { port: port(p) });
                }
                4 => {
                    meter_firmware = meter_firmware.wrapping_add(1).max(1);
                    bench.event(FactEvent::ModuleReflashed {
                        module: name("meter-a"),
                        firmware: label(&format!("M{meter_firmware}")),
                    });
                    oracle.meter_qualified = false;
                    for e in &entries {
                        if measured(e.signal.as_str()) {
                            oracle.stale.insert(key_of_entry(e));
                        }
                    }
                }
                5 => {
                    bench.event(FactEvent::ModuleQualified {
                        module: name("meter-a"),
                        firmware: label(&format!("M{meter_firmware}")),
                        reference: Reference::Standard(name("voltage-ref")),
                    });
                    oracle.meter_qualified = true;
                }
                6 => {
                    let holds = rng.bool();
                    bench.event(FactEvent::ConditionChanged {
                        condition: name("c1"),
                        holds,
                    });
                    if !holds {
                        for e in &entries {
                            if conditional(e.port.as_str(), e.unit) {
                                oracle.stale.insert(key_of_entry(e));
                            }
                        }
                    }
                    oracle.condition = holds;
                }
                7 => {
                    let e = &entries[rng.below(entries.len())];
                    bench.event(FactEvent::ContradictionOpened {
                        fact: key_of_entry(e),
                    });
                    oracle.stale.insert(key_of_entry(e));
                }
                8 if rng.below(4) == 0 => {
                    bench.event(FactEvent::ClockDiscontinuity);
                    oracle.stale.extend(entries.iter().map(key_of_entry));
                    for p in PORTS {
                        oracle.stale.insert(FactKey::Mastership { port: port(p) });
                    }
                    oracle.devices.clear();
                    oracle.meter_qualified = false;
                    oracle.condition = false;
                }
                _ => {
                    // Record an entry again, on the device known now.
                    let e = &entries[rng.below(entries.len())];
                    let place = (e.port.as_str().to_owned(), e.unit);
                    let Some(&fw) = oracle.devices.get(&place) else {
                        continue;
                    };
                    let mut record = bench.entry_record(
                        e.clone(),
                        if measured(e.signal.as_str()) {
                            measured_by("meter-a", &format!("M{meter_firmware}"))
                        } else {
                            Method::OperatorAttested
                        },
                        if conditional(&place.0, e.unit) {
                            &["c1"]
                        } else {
                            &[]
                        },
                    );
                    record.scope.device = Some(identity_for(&place.0, e.unit, fw));
                    let now = bench.tick();
                    let result = bench.ledger.record(record, now);
                    let expect_ok = (!measured(e.signal.as_str()) || oracle.meter_qualified)
                        && (!conditional(&place.0, e.unit) || oracle.condition);
                    assert_eq!(
                        result.is_ok(),
                        expect_ok,
                        "seed {seed} step {step}: {result:?}"
                    );
                    if expect_ok {
                        re_recorded += 1;
                        oracle.stale.remove(&key_of_entry(e));
                    }
                }
            }
            for e in &entries {
                let request = EffectRequest::BusWrite(sawbones_policy::BusWrite {
                    port: e.port.clone(),
                    unit: e.unit,
                    signal: e.signal.clone(),
                    value: WriteValue::U16(1),
                });
                let fresh = relied_facts(&request)
                    .iter()
                    .all(|k| !oracle.stale.contains(k));
                let result = bench.check(&request);
                assert_eq!(
                    result.is_ok(),
                    fresh,
                    "seed {seed} step {step}: {request:?} -> {result:?}"
                );
                if result.is_ok() {
                    passed += 1;
                } else {
                    refused += 1;
                }
                if let Err(refusal) = result {
                    let named = refusal.fact().expect("a fact refusal");
                    assert!(oracle.stale.contains(named), "seed {seed}: names {named}");
                }
            }
        }
    }
    // The run must see both answers, and facts that come back.
    assert!(
        passed > 1_000 && refused > 1_000 && re_recorded > 100,
        "{passed} passed, {refused} refused, {re_recorded} re-recorded"
    );
}

fn key_of_entry(e: &WriteEntry) -> FactKey {
    FactKey::WriteEntry {
        port: e.port.clone(),
        unit: e.unit,
        signal: e.signal.clone(),
    }
}

// --- #231: mutants the weekly mutation run found missed. ---

/// Mutants: `FactLedger::invalidated_by -> None`, `is_contradicted -> false`
/// and `condition_holds -> false`.
#[test]
fn regression_231_policy_ledger_queries_report_state() {
    let mut bench = Bench::new();
    assert!(bench.ledger.invalidated_by(&cv_key()).is_none());
    assert!(!bench.ledger.is_contradicted(&cv_key()));
    assert!(bench.ledger.condition_holds(&name("bank_disconnected")));
    assert!(!bench.ledger.condition_holds(&name("no_such_condition")));

    bench.event(FactEvent::ContradictionOpened { fact: cv_key() });
    assert!(bench.ledger.is_contradicted(&cv_key()));
    assert!(!bench.ledger.is_contradicted(&cutoff_key()));

    bench.event(FactEvent::DeviceIdentified {
        port: port(PORT),
        unit: UNIT,
        identity: inverter("V1.1"),
    });
    assert_eq!(
        bench.ledger.invalidated_by(&cv_key()),
        Some(&InvalidationCause::DeviceChanged {
            port: port(PORT),
            unit: UNIT
        })
    );
    assert!(bench.ledger.invalidated_by(&mastership_key()).is_none());

    bench.event(FactEvent::ConditionChanged {
        condition: name("bank_disconnected"),
        holds: false,
    });
    assert!(!bench.ledger.condition_holds(&name("bank_disconnected")));
}

/// Mutant: `<` with `<=` in `standing_problem`'s epoch check. A fact earned at
/// the very tick of a clock discontinuity belongs to the new epoch; one tick
/// before does not.
#[test]
fn regression_231_policy_a_fact_earned_at_the_epoch_start_is_accepted() {
    let mut bench = Bench::new();
    let template = bench.entry_record(charge_voltage(), Method::OperatorAttested, &[]);
    let epoch = 500;
    bench
        .ledger
        .apply(FactEvent::ClockDiscontinuity, epoch)
        .unwrap();
    bench
        .ledger
        .apply(
            FactEvent::DeviceIdentified {
                port: port(PORT),
                unit: UNIT,
                identity: inverter("V1.0"),
            },
            epoch,
        )
        .unwrap();
    let before = FactRecord {
        earned_at: epoch - 1,
        ..template.clone()
    };
    assert_eq!(
        bench.ledger.record(before, epoch).unwrap_err(),
        RecordError::Fact {
            problem: FactProblem::EarnedBeforeClockDiscontinuity
        }
    );
    let at = FactRecord {
        earned_at: epoch,
        ..template
    };
    assert!(bench.ledger.record(at, epoch).is_ok());
}

/// Mutant: `!=` with `==` in the moved-device filter of `apply`. The same
/// physical device (model and serial) turning up at another port, on the same
/// unit address, means it left its old place: the old place's facts fall.
#[test]
fn regression_231_policy_a_device_seen_on_another_port_leaves_its_old_place() {
    let mut bench = Bench::new();
    bench.passes(&cv_write());
    bench.event(FactEvent::DeviceIdentified {
        port: port(OTHER_PORT),
        unit: UNIT,
        identity: inverter("V1.0"),
    });
    assert!(bench.ledger.device_at(&port(PORT), UNIT).is_none());
    assert!(bench.ledger.device_at(&port(OTHER_PORT), UNIT).is_some());
    assert_eq!(
        bench.refused_invalidated(&cv_write(), &cv_key()),
        InvalidationCause::DeviceChanged {
            port: port(PORT),
            unit: UNIT
        }
    );
}

/// Mutant: `Confirmed::lease_target -> None`. A confirmed class 3 write
/// carries the lease target the interlock must be asked for, with the digest
/// of the action; a class 2 write carries none.
#[test]
fn regression_231_policy_confirmed_class_3_write_has_a_lease_target() {
    let bench = Bench::new();
    for (request, class3) in [(cv_write(), true), (fan_write(), false)] {
        let token = bench.check(&request).unwrap();
        let confirmed = confirm(
            token,
            &request,
            &bench.writes,
            &bench.buses,
            &bench.ledger,
            HEAD,
            bench.now,
        )
        .unwrap();
        match (confirmed.lease_target(), class3) {
            (Some(target), true) => {
                assert_eq!(target.port.0, bench_port_of(PORT));
                assert_eq!(
                    target.action_digest.0,
                    *sawbones_policy::ActionDigest::of(&expected_action(&request)).as_bytes()
                );
            }
            (None, false) => {}
            (got, _) => panic!("{request:?}: lease target {got:?}"),
        }
    }
}
