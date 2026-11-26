//! Attacks with stale, replayed or self-supplied facts against the policy
//! preflight (#110): lanes `bus` and `agent-injection`.

use std::collections::BTreeSet;

use sawbones_interlock::HazardClass;
use sawbones_policy::{
    confirm, preflight, relied_facts, BusFact, BusFacts, BusRead, BusWrite, ChainHead,
    DeviceIdentity, EffectRequest, FactContent, FactEvent, FactKey, FactLedger, FactProblem,
    FactRecord, FactScope, Interface, InvalidationCause, LedgerId, Mastership, Method, PortId,
    PreflightRefusal, ScopeLabel, ScopeName, SettingKind, SignalId, Tick, TokenProblem, Validity,
    WriteEntry, WriteMap, WriteValue,
};
use sawbones_reading::Trust;

const HEAD: ChainHead = ChainHead([7; 32]);

fn port() -> PortId {
    PortId::parse("rs485-1").unwrap()
}

fn identity(firmware: &str) -> DeviceIdentity {
    DeviceIdentity {
        model: ScopeName::parse("inverter-a").unwrap(),
        serial: ScopeLabel::parse("SN0001").unwrap(),
        firmware: ScopeLabel::parse(firmware).unwrap(),
    }
}

fn cv_entry() -> WriteEntry {
    WriteEntry {
        port: port(),
        unit: 1,
        signal: SignalId::parse("charge_voltage").unwrap(),
        register: 0x0020,
        width: 1,
        class: HazardClass::Class2,
        kind: SettingKind::ChargeVoltage,
        energy_relevant: true,
        trust: Trust::OperatorAttested,
        range: None,
    }
}

fn bus_fact() -> BusFact {
    BusFact {
        port: port(),
        bench_port: 0,
        mastership: Mastership::BenchOnly,
        trust: Trust::OperatorAttested,
    }
}

fn cv_write(value: u16) -> EffectRequest {
    EffectRequest::BusWrite(BusWrite {
        port: port(),
        unit: 1,
        signal: SignalId::parse("charge_voltage").unwrap(),
        value: WriteValue::U16(value),
    })
}

fn cv_read() -> EffectRequest {
    EffectRequest::BusRead(BusRead {
        port: port(),
        unit: 1,
        signal: SignalId::parse("charge_voltage").unwrap(),
    })
}

/// A bench where a class 3 charge-voltage write passes preflight.
fn bench() -> (WriteMap, BusFacts, FactLedger, Tick) {
    let mut ledger = FactLedger::new(LedgerId(1));
    ledger
        .apply(
            FactEvent::DeviceIdentified {
                port: port(),
                unit: 1,
                identity: identity("V1.0"),
            },
            1,
        )
        .unwrap();
    let scope = |unit, device| FactScope {
        device,
        interface: Interface {
            port: port(),
            bench_port: 0,
            unit,
        },
        conditions: BTreeSet::new(),
        method: Method::OperatorAttested,
    };
    ledger
        .record(
            FactRecord {
                content: FactContent::WriteEntry(cv_entry()),
                scope: scope(Some(1), Some(identity("V1.0"))),
                earned_at: 1,
                validity: Validity::UntilInvalidated,
            },
            2,
        )
        .unwrap();
    ledger
        .record(
            FactRecord {
                content: FactContent::Mastership(bus_fact()),
                scope: scope(None, None),
                earned_at: 1,
                validity: Validity::For { ticks: 10_000 },
            },
            2,
        )
        .unwrap();
    (
        WriteMap::new([cv_entry()]).unwrap(),
        BusFacts::new([bus_fact()]).unwrap(),
        ledger,
        3,
    )
}

/// bus-020: BenchOnly is a declaration. When the bench hears another
/// master, the host feeds that to the ledger and the policy fact itself is
/// downgraded: every write and read on the port is refused, naming the
/// mastership fact, until the owner attests it again.
#[test]
fn redteam_bus_020_a_foreign_master_invalidates_the_mastership_fact() {
    let (writes, buses, mut ledger, now) = bench();
    for request in [cv_write(5600), cv_read()] {
        assert!(preflight(&request, &writes, &buses, &ledger, HEAD, now).is_ok());
    }
    let applied = ledger
        .apply(FactEvent::ForeignMasterObserved { port: port() }, now + 1)
        .unwrap();
    let key = FactKey::Mastership { port: port() };
    assert_eq!(applied.invalidated, vec![key.clone()]);
    for request in [cv_write(5600), cv_read()] {
        assert_eq!(
            preflight(&request, &writes, &buses, &ledger, HEAD, now + 2).unwrap_err(),
            PreflightRefusal::Fact {
                fact: key.clone(),
                problem: FactProblem::Invalidated {
                    by: InvalidationCause::ForeignMaster { port: port() }
                }
            }
        );
    }
    // The second master going quiet does not bring the fact back; the bus
    // facts still say BenchOnly, but the ledger does not believe them.
    for t in 3..50 {
        assert!(preflight(&cv_read(), &writes, &buses, &ledger, HEAD, now + t).is_err());
    }
}

/// bus-021: the state a write was approved against changes between the
/// check and the action. Any ledger change, a firmware change on the
/// device, an edited write map or a moved chain head voids the token, and a
/// token cannot be spent on another value or replayed.
#[test]
fn redteam_bus_021_a_change_between_preflight_and_action_voids_the_token() {
    // The device is reflashed by a second write path after preflight.
    let (writes, buses, mut ledger, now) = bench();
    let token = preflight(&cv_write(5600), &writes, &buses, &ledger, HEAD, now).unwrap();
    assert_eq!(token.decision().class(), HazardClass::Class3);
    ledger
        .apply(
            FactEvent::DeviceIdentified {
                port: port(),
                unit: 1,
                identity: identity("V1.1"),
            },
            now + 1,
        )
        .unwrap();
    assert_eq!(
        confirm(
            token,
            &cv_write(5600),
            &writes,
            &buses,
            &ledger,
            HEAD,
            now + 1
        )
        .unwrap_err(),
        PreflightRefusal::Token {
            problem: TokenProblem::LedgerChanged
        }
    );
    assert!(preflight(&cv_write(5600), &writes, &buses, &ledger, HEAD, now + 2).is_err());

    // The store records something new: the chain head moves.
    let (writes, buses, ledger, now) = bench();
    let token = preflight(&cv_write(5600), &writes, &buses, &ledger, HEAD, now).unwrap();
    assert_eq!(
        confirm(
            token,
            &cv_write(5600),
            &writes,
            &buses,
            &ledger,
            ChainHead([8; 32]),
            now
        )
        .unwrap_err(),
        PreflightRefusal::Token {
            problem: TokenProblem::ChainHeadMoved
        }
    );

    // A token for one value is not a token for another.
    let token = preflight(&cv_write(5600), &writes, &buses, &ledger, HEAD, now).unwrap();
    assert_eq!(
        confirm(token, &cv_write(6400), &writes, &buses, &ledger, HEAD, now).unwrap_err(),
        PreflightRefusal::Token {
            problem: TokenProblem::OtherRequest
        }
    );

    // The write map is edited to point the signal at another register.
    let token = preflight(&cv_write(5600), &writes, &buses, &ledger, HEAD, now).unwrap();
    let mut moved = cv_entry();
    moved.register = 0x0021;
    let edited = WriteMap::new([moved]).unwrap();
    assert!(matches!(
        confirm(token, &cv_write(5600), &edited, &buses, &ledger, HEAD, now),
        Err(PreflightRefusal::Fact {
            problem: FactProblem::ContentMismatch,
            ..
        })
    ));

    // A held token goes stale.
    let token = preflight(&cv_write(5600), &writes, &buses, &ledger, HEAD, now).unwrap();
    let late = token.expires_at();
    assert_eq!(
        confirm(token, &cv_write(5600), &writes, &buses, &ledger, HEAD, late).unwrap_err(),
        PreflightRefusal::Token {
            problem: TokenProblem::Expired
        }
    );
    // `confirm` takes the token by value and it is not Clone, so one
    // program cannot present it twice; the compile-time half of this is the
    // signature itself.
}

/// inj-022: a hand-off note or an agent's say-so is never a relied-on
/// fact. The preflight has no parameter for facts from the agent: it works
/// out the facts a request relies on from the request alone, and a request
/// with nothing recorded behind it is refused whatever the agent believes.
#[test]
fn redteam_inj_022_preflight_relies_only_on_facts_it_derives_itself() {
    let (writes, buses, ledger, now) = bench();
    let token = preflight(&cv_write(5600), &writes, &buses, &ledger, HEAD, now).unwrap();
    let relied: Vec<FactKey> = token.facts().iter().map(|(k, _)| k.clone()).collect();
    assert_eq!(relied, relied_facts(&cv_write(5600)));
    assert_eq!(
        relied,
        vec![
            FactKey::WriteEntry {
                port: port(),
                unit: 1,
                signal: SignalId::parse("charge_voltage").unwrap()
            },
            FactKey::Mastership { port: port() }
        ]
    );
    // The write map and bus facts alone, as an agent could assemble them,
    // do not pass: nothing is recorded in this ledger.
    let empty = FactLedger::new(LedgerId(1));
    assert_eq!(
        preflight(&cv_write(5600), &writes, &buses, &empty, HEAD, now).unwrap_err(),
        PreflightRefusal::Fact {
            fact: relied[0].clone(),
            problem: FactProblem::Missing
        }
    );
    // An entry at a tier the agent could produce is refused at record time.
    let mut inferred = cv_entry();
    inferred.trust = Trust::AgentInferred;
    let mut ledger = FactLedger::new(LedgerId(2));
    ledger
        .apply(
            FactEvent::DeviceIdentified {
                port: port(),
                unit: 1,
                identity: identity("V1.0"),
            },
            1,
        )
        .unwrap();
    let record = FactRecord {
        content: FactContent::WriteEntry(inferred),
        scope: FactScope {
            device: Some(identity("V1.0")),
            interface: Interface {
                port: port(),
                bench_port: 0,
                unit: Some(1),
            },
            conditions: BTreeSet::new(),
            method: Method::OperatorAttested,
        },
        earned_at: 1,
        validity: Validity::UntilInvalidated,
    };
    assert!(ledger.record(record, 2).is_err());
}
