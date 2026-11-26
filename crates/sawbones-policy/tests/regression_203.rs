//! #203: the unit address is part of every write (bus-016), a class 3
//! decision carries the one bus action it approves (bus-014), and the policy
//! engine, the interlock, the signed approval and the bus guard agree on that
//! action's digest.

mod common;

use common::*;
use sawbones_approvals::test_keys::{sign, verifier, TRUST_ROOT_SEED};
use sawbones_approvals::{
    Approval, BusGuardAllowList, DeviceId, Digest, GrantLease, Message, Role,
};
use sawbones_busguard::{
    AllowList, BusGuard, Function, Port as GuardPort, Rule, TrustRootEvent, Verdict,
};
use sawbones_interlock::{
    step, Event, FaultRecord, HeartbeatChallenge, Inputs, Outputs, State, RAIL_B_PORTS,
};
use sawbones_policy::{
    bus_action, classify, ActionDigest, BusFacts, BusWrite, BusWriteAction, EffectRequest,
    FactError, HazardClass, Mastership, RefusalReason, Requirement, SettingKind, WriteEntry,
    WriteFunction, WriteMap, WriteValue,
};
use sawbones_reading::Trust;

const GUARD: DeviceId = DeviceId(0x203);

fn limit_entry(unit: u8, register: u16) -> WriteEntry {
    WriteEntry {
        unit,
        register,
        ..entry(
            "rs485-1",
            "battery_low_cutoff",
            HazardClass::Class3,
            SettingKind::BatteryProtectionLimit,
            true,
            Trust::OperatorAttested,
        )
    }
}

fn write_to(unit: u8, value: WriteValue) -> EffectRequest {
    EffectRequest::BusWrite(BusWrite {
        port: port("rs485-1"),
        unit,
        signal: signal("battery_low_cutoff"),
        value,
    })
}

fn bench_buses() -> BusFacts {
    BusFacts::new([fact(
        "rs485-1",
        Mastership::BenchOnly,
        Trust::OperatorAttested,
    )])
    .unwrap()
}

fn leased(decision: &sawbones_policy::Decision) -> (BusWriteAction, ActionDigest) {
    match decision.requirement() {
        Requirement::RailBLease {
            action,
            action_digest,
            ..
        } => (*action, *action_digest),
        other => panic!("not a lease: {other:?}"),
    }
}

// --- bus-016: units ---------------------------------------------------------

/// Two paralleled inverters share a signal name on one bus. Their writes are
/// two actions with two digests, and an entry for one unit never authorises
/// the other.
#[test]
fn regression_203_the_unit_is_part_of_the_entry_the_request_and_the_digest() {
    let both = WriteMap::new([limit_entry(1, 0x2000), limit_entry(2, 0x2000)]).unwrap();
    let one = classify(&write_to(1, WriteValue::U16(4600)), &both, &bench_buses());
    let two = classify(&write_to(2, WriteValue::U16(4600)), &both, &bench_buses());
    let (a1, d1) = leased(&one);
    let (a2, d2) = leased(&two);
    assert_eq!((a1.unit(), a2.unit()), (1, 2));
    assert_ne!(d1, d2);
    assert_ne!(a1.frame(), a2.frame());

    let only_one = WriteMap::new([limit_entry(1, 0x2000)]).unwrap();
    for unit in [0, 2, 3, 247, 248, 255] {
        assert_eq!(
            classify(
                &write_to(unit, WriteValue::U16(4600)),
                &only_one,
                &bench_buses()
            )
            .requirement(),
            &Requirement::Refused {
                reason: RefusalReason::UnknownSignal
            },
            "unit {unit}"
        );
    }
    // The request record differs by unit too.
    assert_ne!(
        write_to(1, WriteValue::U16(1)).canonical_bytes(),
        write_to(2, WriteValue::U16(1)).canonical_bytes()
    );
}

#[test]
fn regression_203_entries_need_a_real_unit_and_ports_one_wire_each() {
    for unit in [0, 248, 255] {
        assert_eq!(
            WriteMap::new([limit_entry(unit, 1)]),
            Err(FactError::InvalidUnit {
                port: port("rs485-1"),
                unit,
                signal: signal("battery_low_cutoff"),
            })
        );
    }
    assert!(WriteMap::new([limit_entry(1, 1), limit_entry(247, 1)]).is_ok());
    let mut b = fact("rs485-2", Mastership::BenchOnly, Trust::Measured);
    b.bench_port = bench_port_of("rs485-1");
    assert_eq!(
        BusFacts::new([fact("rs485-1", Mastership::BenchOnly, Trust::Measured), b]),
        Err(FactError::DuplicateBenchPort {
            bench_port: bench_port_of("rs485-1"),
            first: port("rs485-1"),
            second: port("rs485-2"),
        })
    );
}

// --- bus-014: the decision carries the exact action ------------------------

/// Each value type becomes fixed register words and a fixed function.
#[test]
fn regression_203_values_become_these_words() {
    let single = WriteFunction::WriteSingleRegister;
    let multiple = WriteFunction::WriteMultipleRegisters;
    let cases: [(WriteValue, WriteFunction, &[u16]); 8] = [
        (WriteValue::Bool(false), single, &[0]),
        (WriteValue::Bool(true), single, &[1]),
        (WriteValue::U16(4600), single, &[4600]),
        (WriteValue::I16(-1), single, &[0xFFFF]),
        (WriteValue::I16(-300), single, &[0xFED4]),
        (WriteValue::U32(0x1234_5678), multiple, &[0x1234, 0x5678]),
        (WriteValue::I32(-2), multiple, &[0xFFFF, 0xFFFE]),
        (WriteValue::I32(0x0001_0000), multiple, &[0x0001, 0x0000]),
    ];
    for (value, function, words) in cases {
        let action = bus_action(3, 9, 0x0100, value).unwrap();
        assert_eq!(
            action,
            BusWriteAction::new(3, 9, function, 0x0100, words).unwrap(),
            "{value:?}"
        );
    }
    // A two-register value at the last register has no frame.
    assert_eq!(bus_action(0, 1, 0xFFFF, WriteValue::U32(1)), None);
    assert!(bus_action(0, 1, 0xFFFF, WriteValue::U16(1)).is_some());
}

#[test]
fn regression_203_an_unencodable_write_is_refused() {
    let map = WriteMap::new([limit_entry(1, 0xFFFF)]).unwrap();
    let d = classify(&write_to(1, WriteValue::U32(7)), &map, &bench_buses());
    assert_eq!(
        d.requirement(),
        &Requirement::Refused {
            reason: RefusalReason::UnencodableWrite
        }
    );
    assert_eq!(d.class(), HazardClass::Class4);
    let d = classify(&write_to(1, WriteValue::U16(7)), &map, &bench_buses());
    assert_eq!(d.class(), HazardClass::Class3);
}

#[test]
fn regression_203_the_decision_names_the_entry_s_register_unit_and_bench_port() {
    let map = WriteMap::new([limit_entry(5, 0x0123)]).unwrap();
    let d = classify(&write_to(5, WriteValue::U16(4600)), &map, &bench_buses());
    let (action, digest) = leased(&d);
    assert_eq!(action.port(), bench_port_of("rs485-1"));
    assert_eq!(action.unit(), 5);
    assert_eq!(action.register(), 0x0123);
    assert_eq!(action.values(), [4600]);
    assert_eq!(digest, ActionDigest::of(&action));
    assert_eq!(
        action.frame().as_bytes(),
        [0x05, 0x06, 0x01, 0x23, 0x11, 0xF8, 0x75, 0xAA]
    );
}

// --- One digest end to end -------------------------------------------------

/// Drives the interlock from a clean boot to a live rail B lease for
/// `target`, with a supervisor that answers each heartbeat challenge once.
fn interlock_lease(target: sawbones_interlock::LeaseTarget) -> (State, Outputs, u64) {
    let idle = Inputs {
        estop_pressed: false,
        arm_switch_on: false,
        key_switch_on: false,
        heartbeat: None,
        panel_button_pressed: false,
        rail_a_sensed: false,
        rail_b_sensed: [false; RAIL_B_PORTS],
        firmware_jumper_fitted: false,
        now: 0,
    };
    let armed = Inputs {
        arm_switch_on: true,
        ..idle
    };
    let keyed = Inputs {
        key_switch_on: true,
        ..armed
    };
    let pressed = Inputs {
        panel_button_pressed: true,
        ..keyed
    };
    let script = [
        (idle, None),
        (armed, None),
        (
            keyed,
            Some(Event::RequestLease {
                class: HazardClass::Class3,
                target,
            }),
        ),
        (pressed, None),
    ];
    let boot = Event::Restart {
        record: Some(FaultRecord::Clear),
        nonce: 0x203,
    };
    let (mut state, mut last) = step(State::POWER_ON, armed, Some(boot));
    let mut answered: Option<HeartbeatChallenge> = None;
    let mut now = 0;
    for (inputs, event) in script {
        now += 1;
        // The rails' feedback follows what was commanded.
        let mut rail_b_sensed = [false; RAIL_B_PORTS];
        if let Some(p) = last.rail_b {
            rail_b_sensed[usize::from(p.0)] = true;
        }
        let heartbeat = if last.challenge == answered {
            None
        } else {
            answered = last.challenge;
            last.challenge.map(HeartbeatChallenge::reply)
        };
        let inputs = Inputs {
            now,
            rail_a_sensed: last.rail_a,
            rail_b_sensed,
            heartbeat,
            ..inputs
        };
        (state, last) = step(state, inputs, event);
        assert_eq!(last.refusal, None, "tick {now}");
        assert_eq!(last.fault, None, "tick {now}");
    }
    (state, last, now)
}

/// One write, classified class 3, leased by the interlock, approved by a
/// signed trust-root message and checked by the bus guard: every stage
/// carries the same 32 digest bytes, and the guard passes exactly the frame
/// of the classified action, once.
#[test]
fn regression_203_policy_interlock_approval_and_guard_agree_on_one_action() {
    let mut rng = Rng::new(0x203);
    let mut verifier = verifier(GUARD);
    for seed in 0..64u64 {
        let unit = 1 + rng.below(247) as u8;
        let register = (rng.next_u64() as u16) % 0xFF00;
        let value = rng.value();
        let map = WriteMap::new([limit_entry(unit, register)]).unwrap();
        // Bench port 0..RAIL_B_PORTS, so the interlock gates it.
        let p = format!("rs485-{}", rng.below(RAIL_B_PORTS));
        let mut bus = fact(&p, Mastership::BenchOnly, Trust::OperatorAttested);
        bus.bench_port = rng.below(RAIL_B_PORTS) as u8;
        let buses = BusFacts::new([bus.clone()]).unwrap();
        let request = EffectRequest::BusWrite(BusWrite {
            port: port(&p),
            unit,
            signal: signal("battery_low_cutoff"),
            value,
        });
        let map = WriteMap::new(map.entries().map(|e| WriteEntry {
            port: port(&p),
            ..e.clone()
        }))
        .unwrap();

        // Policy.
        let decision = classify(&request, &map, &buses);
        let (action, digest) = leased(&decision);
        assert_eq!(
            action,
            bus_action(bus.bench_port, unit, register, value).unwrap()
        );

        // Interlock: the lease target is the action's port and digest.
        let target = decision.requirement().lease_target().unwrap();
        assert_eq!(target.port.0, action.port());
        assert_eq!(&target.action_digest.0, digest.as_bytes());
        let (state, outputs, now) = interlock_lease(target);
        assert!(outputs.rail_b_on(target.port), "seed {seed}");

        // The trust root's approval carries the action and the same digest.
        let grant = GrantLease::new(action);
        assert_eq!(&grant.action_digest.0, digest.as_bytes());
        let signed = |counter: u64, approval: Approval| {
            let bytes = Message {
                target: GUARD,
                signer: Role::TrustRoot,
                counter,
                expires_at: 1_000,
                approval,
            }
            .encode();
            (bytes, sign(&TRUST_ROOT_SEED, &bytes))
        };
        let (bytes, signature) = signed(2 * seed + 1, Approval::GrantLease(grant));
        let approved = verifier
            .verify::<GrantLease>(&bytes, &signature, 0)
            .unwrap();

        // The guard: its lease has the same digest and passes only the
        // action's frame.
        let mut list = AllowList::new();
        let function = match action.function() {
            WriteFunction::WriteSingleRegister => Function::WriteSingleRegister,
            WriteFunction::WriteMultipleRegisters => Function::WriteMultipleRegisters,
        };
        list.try_push(Rule {
            port: GuardPort(action.port()),
            unit,
            function,
            first: 0,
            last: 0xFFFF,
            max_quantity: 4,
        })
        .unwrap();
        let (list_bytes, list_signature) = signed(
            2 * seed + 2,
            Approval::BusGuardAllowList(BusGuardAllowList {
                allow_list: Digest(list.digest()),
            }),
        );
        let list_approved = verifier
            .verify::<BusGuardAllowList>(&list_bytes, &list_signature, 0)
            .unwrap();
        let mut guard = BusGuard::new();
        guard
            .apply(
                TrustRootEvent::from_approved_allow_list(list_approved, list).unwrap(),
                0,
            )
            .unwrap();
        guard
            .apply(TrustRootEvent::from_approved_lease(approved).unwrap(), 1)
            .unwrap();
        let lease = guard.lease().unwrap();
        assert_eq!(&lease.action_digest.0, digest.as_bytes());
        assert_eq!(lease.action, action);
        let frame = action.frame();
        let mut other = frame.as_bytes().to_vec();
        other[0] = if unit == 247 { 1 } else { unit + 1 };
        assert_ne!(
            guard.decide(GuardPort(action.port()), &other, 2),
            Verdict::Pass
        );
        assert_eq!(
            guard.decide(GuardPort(action.port()), frame.as_bytes(), 2),
            Verdict::Pass
        );
        assert_ne!(
            guard.decide(GuardPort(action.port()), frame.as_bytes(), 3),
            Verdict::Pass
        );

        // And the interlock accepts the use of exactly that target.
        let mut rail_b_sensed = [false; RAIL_B_PORTS];
        rail_b_sensed[usize::from(target.port.0)] = true;
        let inputs = Inputs {
            estop_pressed: false,
            arm_switch_on: true,
            key_switch_on: true,
            heartbeat: None,
            panel_button_pressed: false,
            rail_a_sensed: true,
            rail_b_sensed,
            firmware_jumper_fitted: false,
            now: now + 1,
        };
        let (_, used) = step(state, inputs, Some(Event::LeaseUsed { target }));
        assert_eq!(used.refusal, None, "seed {seed}");
    }
}
