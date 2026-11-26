//! Properties, checked two ways: exhaustively over every combination of the
//! enum-valued facts against an independent oracle, and over random maps and
//! requests from a seeded generator.

mod common;

use common::*;
use sawbones_policy::{
    class_rank, classify, ActionDigest, BusFact, BusFacts, BusListen, BusRead, BusWrite, Decision,
    EffectRequest, HazardClass, Mastership, RefusalReason, Requirement, SettingKind, WriteEntry,
    WriteMap, WriteValue,
};
use sawbones_reading::Trust;

const CASES: u64 = 2_000;

/// The class and requirement must agree, and class 4 is always refused.
fn assert_consistent(decision: &Decision, request: &EffectRequest, context: &str) {
    match (decision.class(), decision.requirement()) {
        (HazardClass::Class0 | HazardClass::Class1, Requirement::Nothing) => {}
        (HazardClass::Class2, Requirement::RailA { port }) => {
            assert_eq!(port, request.port(), "{context}");
        }
        (
            HazardClass::Class3,
            Requirement::RailBLease {
                port,
                action,
                action_digest,
            },
        ) => {
            assert_eq!(port, request.port(), "{context}");
            assert_eq!(*action, expected_action(request), "{context}");
            assert_eq!(*action_digest, ActionDigest::of(action), "{context}");
        }
        (HazardClass::Class4, Requirement::Refused { .. }) => {}
        (class, req) => panic!("{context}: class {class:?} with {req:?}"),
    }
}

/// The rules restated independently, as a rank, with `None` for refused.
fn oracle_write_rank(entry: &WriteEntry, bus: Option<&BusFact>) -> Option<u8> {
    let trusted = |t: Trust| matches!(t, Trust::Measured | Trust::OperatorAttested);
    if !trusted(entry.trust) {
        return None;
    }
    let kind_min = match entry.kind {
        SettingKind::Config => 0,
        SettingKind::BatteryProtectionLimit | SettingKind::ChargeVoltage => 3,
        SettingKind::MainsSwitching | SettingKind::PvStringSwitching => 4,
    };
    let energy = if entry.energy_relevant { 2 } else { 0 };
    let rank = class_rank(entry.class).max(kind_min).max(energy).max(2);
    let bus_ok = bus.is_some_and(|f| trusted(f.trust) && f.mastership == Mastership::BenchOnly);
    (rank < 4 && bus_ok).then_some(rank)
}

#[test]
fn every_combination_of_facts_matches_the_oracle() {
    let mut buses: Vec<Option<BusFact>> = vec![None];
    for m in MASTERSHIPS {
        for t in TIERS {
            buses.push(Some(fact("p", m, t)));
        }
    }
    let mut cases = 0;
    for class in CLASSES {
        for kind in KINDS {
            for energy in [false, true] {
                for trust in TIERS {
                    for bus in &buses {
                        let e = entry("p", "s", class, kind, energy, trust);
                        let writes = WriteMap::new([e.clone()]).unwrap();
                        let facts = BusFacts::new(bus.clone()).unwrap();
                        let request = write("p", "s", WriteValue::U16(7));
                        let d = classify(&request, &writes, &facts);
                        let context = format!("{e:?} {bus:?}");
                        assert_consistent(&d, &request, &context);
                        let got = (!d.is_refused()).then(|| class_rank(d.class()));
                        assert_eq!(got, oracle_write_rank(&e, bus.as_ref()), "{context}");
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 5 * 5 * 2 * 5 * 11);
}

fn random_world(rng: &mut Rng) -> (WriteMap, BusFacts) {
    let mut entries = Vec::new();
    for _ in 0..rng.below(12) {
        let e = entry(
            &rng.name("port"),
            &rng.name("sig"),
            rng.pick(&CLASSES),
            rng.pick(&KINDS),
            rng.bool(),
            rng.pick(&TIERS),
        );
        if !entries
            .iter()
            .any(|x: &WriteEntry| x.port == e.port && x.signal == e.signal)
        {
            entries.push(e);
        }
    }
    let mut facts = Vec::new();
    for _ in 0..rng.below(6) {
        let f = fact(&rng.name("port"), rng.pick(&MASTERSHIPS), rng.pick(&TIERS));
        if !facts.iter().any(|x: &BusFact| x.port == f.port) {
            facts.push(f);
        }
    }
    (
        WriteMap::new(entries).unwrap(),
        BusFacts::new(facts).unwrap(),
    )
}

fn random_request(rng: &mut Rng, writes: &WriteMap) -> EffectRequest {
    match rng.below(4) {
        // A write to a signal the map knows, if it knows any.
        0 => match writes
            .entries()
            .nth(rng.below(writes.entries().count().max(1)))
        {
            Some(e) => EffectRequest::BusWrite(BusWrite {
                port: e.port.clone(),
                unit: e.unit,
                signal: e.signal.clone(),
                value: rng.value(),
            }),
            None => write(&rng.name("port"), &rng.name("sig"), rng.value()),
        },
        1 => write(&rng.name("port"), &rng.name("sig"), rng.value()),
        2 => read(&rng.name("port"), &rng.name("sig")),
        _ => listen(&rng.name("port")),
    }
}

#[test]
fn random_decisions_obey_every_property() {
    for seed in 0..CASES {
        let mut rng = Rng::new(seed);
        let (writes, buses) = random_world(&mut rng);
        for _ in 0..8 {
            let request = random_request(&mut rng, &writes);
            let d = classify(&request, &writes, &buses);
            let context = format!("seed {seed}: {request:?}");

            assert_consistent(&d, &request, &context);
            assert_eq!(
                d,
                classify(&request, &writes, &buses),
                "deterministic: {context}"
            );

            match &request {
                EffectRequest::BusWrite(w) => match writes.get(&w.port, w.unit, &w.signal) {
                    // Never lower than the entry's class.
                    Some(e) => {
                        assert!(class_rank(d.class()) >= class_rank(e.class), "{context}");
                        assert!(
                            class_rank(d.class()) >= class_rank(e.kind.minimum_class()),
                            "{context}"
                        );
                        if e.class == HazardClass::Class4
                            || e.kind.minimum_class() == HazardClass::Class4
                        {
                            // Class 4 is always refused. The reason may be
                            // the entry's tier, which is checked first.
                            assert!(d.is_refused(), "{context}");
                            assert_eq!(d.class(), HazardClass::Class4, "{context}");
                        }
                    }
                    // Unknown signals are always refused.
                    None => assert_eq!(
                        d.requirement(),
                        &Requirement::Refused {
                            reason: RefusalReason::UnknownSignal
                        },
                        "{context}"
                    ),
                },
                // A read transmits: never below class 2.
                EffectRequest::BusRead(_) => {
                    assert!(class_rank(d.class()) >= 2, "{context}");
                }
                EffectRequest::BusListen(_) => {
                    assert_eq!(d.class(), HazardClass::Class0, "{context}");
                }
            }
        }
    }
}

/// Context can raise a class, never lower it: making an entry
/// energy-relevant, raising its class, or raising its kind's minimum never
/// gives a less demanding decision.
#[test]
fn context_only_raises() {
    let buses = BusFacts::new([fact("p", Mastership::BenchOnly, Trust::Measured)]).unwrap();
    let strictness = |e: &WriteEntry| {
        let d = classify(
            &write("p", "s", WriteValue::Bool(true)),
            &WriteMap::new([e.clone()]).unwrap(),
            &buses,
        );
        class_rank(d.class())
    };
    let mut rng = Rng::new(0x5eed);
    for _ in 0..CASES {
        let base = entry(
            "p",
            "s",
            rng.pick(&CLASSES),
            rng.pick(&KINDS),
            rng.bool(),
            rng.pick(&TIERS),
        );
        let before = strictness(&base);
        let energy = WriteEntry {
            energy_relevant: true,
            ..base.clone()
        };
        assert!(strictness(&energy) >= before, "{base:?}");
        for class in CLASSES {
            if class_rank(class) >= class_rank(base.class) {
                let raised = WriteEntry {
                    class,
                    ..base.clone()
                };
                assert!(strictness(&raised) >= before, "{base:?} -> {class:?}");
            }
        }
        for kind in KINDS {
            if class_rank(kind.minimum_class()) >= class_rank(base.kind.minimum_class()) {
                let raised = WriteEntry {
                    kind,
                    ..base.clone()
                };
                assert!(strictness(&raised) >= before, "{base:?} -> {kind:?}");
            }
        }
    }
}

/// Every single-field change to a request, including a change of value
/// width with the same number, changes the digest.
fn mutations(request: &EffectRequest, rng: &mut Rng) -> Vec<EffectRequest> {
    let other_port = port("other-port");
    let other_signal = signal("other-signal");
    match request {
        EffectRequest::BusWrite(w) => {
            let mut out = vec![
                EffectRequest::BusWrite(BusWrite {
                    port: other_port,
                    ..w.clone()
                }),
                EffectRequest::BusWrite(BusWrite {
                    signal: other_signal,
                    ..w.clone()
                }),
                EffectRequest::BusWrite(BusWrite {
                    unit: w.unit % 247 + 1,
                    ..w.clone()
                }),
                EffectRequest::BusRead(BusRead {
                    port: w.port.clone(),
                    unit: w.unit,
                    signal: w.signal.clone(),
                }),
            ];
            let mut v = rng.value();
            while v == w.value {
                v = rng.value();
            }
            out.push(EffectRequest::BusWrite(BusWrite {
                value: v,
                ..w.clone()
            }));
            // Same number, different width or signedness.
            let widened = match w.value {
                WriteValue::Bool(b) => WriteValue::U16(u16::from(b)),
                WriteValue::U16(n) => WriteValue::U32(u32::from(n)),
                WriteValue::I16(n) => WriteValue::I32(i32::from(n)),
                WriteValue::U32(n) => WriteValue::I32(n as i32),
                WriteValue::I32(n) => WriteValue::U32(n as u32),
            };
            out.push(EffectRequest::BusWrite(BusWrite {
                value: widened,
                ..w.clone()
            }));
            out
        }
        EffectRequest::BusRead(r) => vec![
            EffectRequest::BusRead(BusRead {
                port: other_port,
                ..r.clone()
            }),
            EffectRequest::BusRead(BusRead {
                signal: other_signal,
                ..r.clone()
            }),
            EffectRequest::BusRead(BusRead {
                unit: r.unit % 247 + 1,
                ..r.clone()
            }),
            EffectRequest::BusListen(BusListen {
                port: r.port.clone(),
            }),
        ],
        EffectRequest::BusListen(l) => vec![
            EffectRequest::BusListen(BusListen { port: other_port }),
            EffectRequest::BusRead(BusRead {
                port: l.port.clone(),
                unit: UNIT,
                signal: other_signal,
            }),
        ],
    }
}

#[test]
fn the_encoding_changes_if_any_field_changes() {
    let empty = WriteMap::default();
    for seed in 0..CASES {
        let mut rng = Rng::new(seed);
        let request = random_request(&mut rng, &empty);
        let bytes = request.canonical_bytes();
        assert_eq!(bytes, request.clone().canonical_bytes(), "seed {seed}");
        for changed in mutations(&request, &mut rng) {
            assert_ne!(changed, request, "seed {seed}: mutation must differ");
            assert_ne!(
                changed.canonical_bytes(),
                bytes,
                "seed {seed}: {request:?} vs {changed:?}"
            );
        }
    }
}

/// The length prefixes keep a boundary shift between port and signal from
/// producing the same bytes.
#[test]
fn the_encoding_is_injective_across_field_boundaries() {
    let a = write("ab", "c", WriteValue::U16(1));
    let b = write("a", "bc", WriteValue::U16(1));
    assert_ne!(a.canonical_bytes(), b.canonical_bytes());
}
