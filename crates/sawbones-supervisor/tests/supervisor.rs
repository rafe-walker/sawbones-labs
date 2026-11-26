//! The supervisor skeleton (#100) against in-memory fake modules, the real
//! interlock state machine and a fake bus.

mod common;

use common::*;
use sawbones_interlock::{HazardClass, HEARTBEAT_PERIOD_TICKS};
use sawbones_module_protocol::{Capabilities, Challenge, IdentityError, ImageDigest, Request};
use sawbones_module_protocol::{HostLink, MAX_FRAME_LEN};
use sawbones_policy::{
    BusRead, BusWrite, EffectRequest, FactEvent, FactKey, FactProblem, InvalidationCause,
    PreflightRefusal, Reference, ScopeName, TokenProblem, WriteValue,
};
use sawbones_supervisor::{
    ArmRefusal, BusReply, Happened, InvokeRefusal, ModuleState, Outcome, PortRefusal, PortState,
    QuarantineReason, ToInterlock, LINK_TIMEOUT_TICKS, POLL_PERIOD_TICKS,
};

fn write(value: u16) -> EffectRequest {
    EffectRequest::BusWrite(BusWrite {
        port: bus_port(),
        unit: 1,
        signal: signal(),
        value: WriteValue::U16(value),
    })
}

fn read() -> EffectRequest {
    EffectRequest::BusRead(BusRead {
        port: bus_port(),
        unit: 1,
        signal: signal(),
    })
}

fn quarantined(out: &[Happened]) -> Vec<QuarantineReason> {
    out.iter()
        .filter_map(|h| match h {
            Happened::Quarantined { reason, .. } => Some(*reason),
            _ => None,
        })
        .collect()
}

// ---- Registry -------------------------------------------------------------

#[test]
fn enrolled_modules_on_their_ports_become_available() {
    let mut b = Bench::new(HazardClass::Class2);
    let out = b.tick();
    assert!(out.contains(&Happened::Available {
        module: POD,
        port: POD_PORT
    }));
    assert_eq!(b.s.module_state(POD), Some(ModuleState::Available));
    assert_eq!(b.s.module_state(METER), Some(ModuleState::Available));
    assert_eq!(
        b.s.module_state(sawbones_module_protocol::ModuleId(99)),
        None
    );
}

#[test]
fn a_duplicate_id_is_quarantined_on_both_ports_until_re_enrolled() {
    let mut b = Bench::new(HazardClass::Class2);
    b.tick();
    // A second device holding the pod's key appears on a spare port.
    b.hub.plug(SPARE_PORT, FakeModule::new(POD_SEED, POD));
    let out = b.tick();
    assert_eq!(
        quarantined(&out),
        vec![QuarantineReason::DuplicateId {
            first: POD_PORT,
            second: SPARE_PORT
        }]
    );
    let why = PortRefusal::Quarantined(POD);
    assert_eq!(b.s.port_state(POD_PORT), Some(PortState::Refused(why)));
    assert_eq!(b.s.port_state(SPARE_PORT), Some(PortState::Refused(why)));
    assert!(matches!(
        b.s.module_state(POD),
        Some(ModuleState::Quarantined(
            QuarantineReason::DuplicateId { .. }
        ))
    ));
    // Both were told to go to their safe state.
    for port in [POD_PORT, SPARE_PORT] {
        let last = b.hub.sent[&port].last().unwrap();
        let frame = sawbones_module_protocol::decode_request(last).unwrap();
        assert_eq!(frame.request, Request::SafeState);
    }
    // Unplugging the copy changes nothing: the quarantine is sticky.
    b.hub.unplug(SPARE_PORT);
    b.run(POLL_PERIOD_TICKS * 3);
    assert!(matches!(
        b.s.module_state(POD),
        Some(ModuleState::Quarantined(_))
    ));
    // A person re-enrolls it; it is identified again on the next poll.
    b.s.re_enroll(pod_enrollment()).unwrap();
    b.run(POLL_PERIOD_TICKS + 1);
    assert_eq!(b.s.module_state(POD), Some(ModuleState::Available));
}

#[test]
fn a_module_on_another_hub_port_is_quarantined() {
    let mut b = Bench::new(HazardClass::Class2);
    let pod = b.hub.unplug(POD_PORT).unwrap();
    b.hub.plug(SPARE_PORT, pod);
    let out = b.tick();
    assert_eq!(
        quarantined(&out),
        vec![QuarantineReason::WrongHubPort {
            enrolled: POD_PORT,
            seen: SPARE_PORT
        }]
    );
    // Moving it back does not help: a person has to re-enroll it.
    let pod = b.hub.unplug(SPARE_PORT).unwrap();
    b.hub.plug(POD_PORT, pod);
    b.run(3);
    assert!(matches!(
        b.s.module_state(POD),
        Some(ModuleState::Quarantined(_))
    ));
}

#[test]
fn signed_can_emit_beyond_the_enrollment_is_quarantined() {
    let mut b = Bench::new(HazardClass::Class2);
    b.hub.module(METER_PORT).capabilities = Capabilities {
        can_emit: true,
        max_class: HazardClass::Class0,
    };
    let out = b.tick();
    assert_eq!(
        quarantined(&out),
        vec![QuarantineReason::CapabilityExceeded {
            enrolled: quiet(),
            claimed: Capabilities {
                can_emit: true,
                max_class: HazardClass::Class0
            }
        }]
    );
    assert_eq!(b.s.module_state(POD), Some(ModuleState::Available));
}

#[test]
fn a_higher_class_than_enrolled_is_quarantined() {
    let mut b = Bench::new(HazardClass::Class2);
    b.hub.module(METER_PORT).capabilities.max_class = HazardClass::Class2;
    let out = b.tick();
    assert!(matches!(
        quarantined(&out)[..],
        [QuarantineReason::CapabilityExceeded { .. }]
    ));
}

#[test]
fn a_manifest_other_than_the_pinned_one_is_quarantined() {
    let mut b = Bench::new(HazardClass::Class2);
    b.hub.module(METER_PORT).manifest = sawbones_module_protocol::ManifestDigest([9; 32]);
    let out = b.tick();
    assert_eq!(quarantined(&out), vec![QuarantineReason::ManifestMismatch]);
}

#[test]
fn a_device_that_cannot_sign_is_refused_not_quarantined() {
    let mut b = Bench::new(HazardClass::Class2);
    // A stranger claims to be the meter, on the meter's port.
    b.hub.unplug(METER_PORT);
    b.hub
        .plug(METER_PORT, FakeModule::new(STRANGER_SEED, METER));
    let out = b.tick();
    assert!(quarantined(&out).is_empty());
    assert_eq!(
        b.s.port_state(METER_PORT),
        Some(PortState::Refused(PortRefusal::BadIdentity(
            IdentityError::BadSignature
        )))
    );
    // An id nobody enrolled grants nothing either.
    b.hub.plug(
        SPARE_PORT,
        FakeModule::new(STRANGER_SEED, sawbones_module_protocol::ModuleId(77)),
    );
    b.tick();
    assert_eq!(
        b.s.port_state(SPARE_PORT),
        Some(PortState::Refused(PortRefusal::Unenrolled(
            sawbones_module_protocol::ModuleId(77)
        )))
    );
}

// ---- Link timeouts --------------------------------------------------------

#[test]
fn a_lost_module_invalidates_its_facts_and_blocks_a_dependent_write() {
    let mut b = Bench::new(HazardClass::Class2);
    b.tick();
    record_facts(&mut b.s, HazardClass::Class2, b.now);
    let early = b.s.preflight(&write(10), b.now).unwrap();

    // The pod stops answering. Its next poll misses the deadline.
    b.hub.module(POD_PORT).responsive = false;
    let out = b.run(POLL_PERIOD_TICKS + LINK_TIMEOUT_TICKS);
    assert!(out.contains(&Happened::Lost {
        module: POD,
        port: POD_PORT
    }));
    let invalidated: Vec<&FactKey> = out
        .iter()
        .filter_map(|h| match h {
            Happened::FactsInvalidated(keys) => Some(keys),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(invalidated.len(), 2, "{invalidated:?}");
    assert_eq!(b.s.module_state(POD), Some(ModuleState::Lost));

    // A new preflight names the invalidated fact.
    let refusal = b.s.preflight(&write(10), b.now).unwrap_err();
    assert!(
        matches!(
            &refusal,
            PreflightRefusal::Fact {
                fact: FactKey::WriteEntry { .. },
                problem: FactProblem::Invalidated {
                    by: InvalidationCause::PortRewired { .. }
                }
            }
        ),
        "{refusal:?}"
    );
    // A token taken before the loss is refused too, before confirm.
    b.arm();
    let got =
        b.s.invoke(&write(10), early, b.now, &mut b.bus, &mut b.interlock);
    assert_eq!(got, Err(InvokeRefusal::CarrierNotAvailable { module: POD }));
    assert!(b.bus.performed.is_empty());

    // It comes back and is identified again, but its facts stay invalid.
    b.hub.module(POD_PORT).responsive = true;
    b.run(POLL_PERIOD_TICKS + 1);
    assert_eq!(b.s.module_state(POD), Some(ModuleState::Available));
    assert!(b.s.preflight(&write(10), b.now).is_err());
}

#[test]
fn a_reply_inside_the_deadline_keeps_a_module_available() {
    let mut b = Bench::new(HazardClass::Class2);
    let out = b.run(POLL_PERIOD_TICKS * 20);
    assert!(!out.iter().any(|h| matches!(h, Happened::Lost { .. })));
    assert_eq!(b.s.module_state(POD), Some(ModuleState::Available));
}

#[test]
fn an_unplugged_module_is_lost_and_its_facts_invalidated() {
    let mut b = Bench::new(HazardClass::Class2);
    b.tick();
    record_facts(&mut b.s, HazardClass::Class2, b.now);
    b.hub.unplug(POD_PORT);
    let out = b.tick();
    assert!(out.contains(&Happened::Unplugged(POD_PORT)));
    assert!(out.contains(&Happened::Lost {
        module: POD,
        port: POD_PORT
    }));
    assert_eq!(b.s.module_state(POD), Some(ModuleState::Absent));
    assert!(b.s.preflight(&write(10), b.now).is_err());
}

#[test]
fn an_identify_replay_ends_in_a_loss_and_a_new_identify() {
    // #98: a replayed Identify ends the module's session. The supervisor's
    // next poll names the old session, the module drops it, the deadline
    // passes, and the module is lost, its facts invalidated, and identified
    // again: a denial of service, never an action on a stale session.
    let mut b = Bench::new(HazardClass::Class2);
    b.tick();
    record_facts(&mut b.s, HazardClass::Class2, b.now);
    let mut other = HostLink::new();
    let mut buf = [0u8; MAX_FRAME_LEN];
    let len = other.open(Challenge([0xee; 16]), &mut buf).unwrap();
    let _ = b.hub.module(POD_PORT).handle(&buf[..len]);

    let out = b.run(POLL_PERIOD_TICKS + LINK_TIMEOUT_TICKS);
    assert!(out.contains(&Happened::Lost {
        module: POD,
        port: POD_PORT
    }));
    assert!(b.s.preflight(&write(10), b.now).is_err());
    b.run(POLL_PERIOD_TICKS + 1);
    assert_eq!(b.s.module_state(POD), Some(ModuleState::Available));
}

#[test]
fn a_new_firmware_digest_unqualifies_an_instrument() {
    let mut b = Bench::new(HazardClass::Class2);
    b.tick();
    let meter = ScopeName::parse("dc-meter").unwrap();
    let firmware = sawbones_policy::ScopeLabel::parse(&"07".repeat(32)).unwrap();
    b.s.apply_fact_event(
        FactEvent::ModuleQualified {
            module: meter.clone(),
            firmware,
            reference: Reference::Standard(ScopeName::parse("vref").unwrap()),
        },
        b.now,
    )
    .unwrap();
    assert!(b.s.ledger().is_qualified(&meter));
    // Reflashed, and back on its port.
    let mut m = b.hub.unplug(METER_PORT).unwrap();
    b.tick();
    m.firmware = ImageDigest([8; 32]);
    m.link = sawbones_module_protocol::ModuleLink::new(METER);
    b.hub.plug(METER_PORT, m);
    b.tick();
    assert_eq!(b.s.module_state(METER), Some(ModuleState::Available));
    assert!(!b.s.ledger().is_qualified(&meter));
}

// ---- Heartbeat ------------------------------------------------------------

#[test]
fn the_main_loop_keeps_the_rails_up() {
    let mut b = Bench::new(HazardClass::Class2);
    b.arm();
    for _ in 0..50 * HEARTBEAT_PERIOD_TICKS {
        b.tick();
        assert!(b.interlock.outputs.rail_a);
        assert_eq!(b.interlock.outputs.fault, None);
    }
}

#[test]
fn a_missed_heartbeat_drops_the_rails_within_the_interlocks_budget() {
    let mut b = Bench::new(HazardClass::Class2);
    b.arm();
    // Run until the supervisor has just answered, then hang it.
    loop {
        let out = b.tick();
        if out.contains(&Happened::HeartbeatAnswered) {
            break;
        }
    }
    // The reply is delivered on the next interlock tick.
    let last_reply = b.now;
    let mut dropped = None;
    for _ in 0..3 * HEARTBEAT_PERIOD_TICKS {
        b.tick_hung();
        if !b.interlock.outputs.rail_a {
            dropped = Some(b.now - 1);
            break;
        }
    }
    let dropped = dropped.expect("rail A stayed on with the supervisor hung");
    assert!(dropped <= last_reply + 2 * HEARTBEAT_PERIOD_TICKS);
    assert_eq!(
        b.interlock.outputs.fault,
        Some(sawbones_interlock::FaultReason::HeartbeatLost)
    );
    // The supervisor sees the fault and drops its armed plan.
    let out = b.tick();
    assert!(out.contains(&Happened::PlanDropped));
    assert!(b.s.armed_plan().is_none());
}

// ---- Isolation ------------------------------------------------------------

#[test]
fn a_stop_isolation_finding_refuses_arming() {
    let mut b = Bench::new(HazardClass::Class2);
    b.tick();
    let refused = b.s.arm(&clipped_plan(), &mut b.interlock).unwrap_err();
    let ArmRefusal::Isolation { violations } = refused;
    assert!(matches!(
        violations[..],
        [sawbones_isolation::Finding::ModuleNotFloating { .. }]
    ));
    assert!(b.s.armed_plan().is_none());
    assert!(!b.interlock.requests.contains(&ToInterlock::RequestArm));

    // Even with rail A on by the switch, a class 2 write is refused.
    record_facts(&mut b.s, HazardClass::Class2, b.now);
    b.interlock.arm_switch_on = false;
    b.run(3);
    b.interlock.arm_switch_on = true;
    b.run(2);
    assert!(b.interlock.outputs.rail_a);
    let token = b.s.preflight(&write(10), b.now).unwrap();
    let got =
        b.s.invoke(&write(10), token, b.now, &mut b.bus, &mut b.interlock);
    assert_eq!(got, Err(InvokeRefusal::NotArmed));
    assert!(b.bus.performed.is_empty());
}

// ---- The write path -------------------------------------------------------

#[test]
fn a_class_2_write_goes_through_preflight_and_confirm() {
    let mut b = Bench::new(HazardClass::Class2);
    b.arm();
    record_facts(&mut b.s, HazardClass::Class2, b.now);
    let token = b.s.preflight(&write(10), b.now).unwrap();
    assert_eq!(token.decision().class(), HazardClass::Class2);
    let got =
        b.s.invoke(&write(10), token, b.now, &mut b.bus, &mut b.interlock);
    assert_eq!(got, Ok(Outcome::Done));
    assert_eq!(b.bus.performed, vec![(write(10), 0)]);
}

#[test]
fn a_token_for_one_request_does_not_carry_out_another() {
    let mut b = Bench::new(HazardClass::Class2);
    b.arm();
    record_facts(&mut b.s, HazardClass::Class2, b.now);
    let token = b.s.preflight(&write(10), b.now).unwrap();
    let got =
        b.s.invoke(&write(11), token, b.now, &mut b.bus, &mut b.interlock);
    assert_eq!(
        got,
        Err(InvokeRefusal::Preflight(PreflightRefusal::Token {
            problem: TokenProblem::OtherRequest
        }))
    );
    assert!(b.bus.performed.is_empty());
}

#[test]
fn a_write_with_no_reply_blocks_its_port_until_a_read_back() {
    let mut b = Bench::new(HazardClass::Class2);
    b.arm();
    record_facts(&mut b.s, HazardClass::Class2, b.now);
    b.bus.reply = BusReply::NoReply;
    let token = b.s.preflight(&write(10), b.now).unwrap();
    let got =
        b.s.invoke(&write(10), token, b.now, &mut b.bus, &mut b.interlock);
    assert_eq!(got, Ok(Outcome::Unknown));
    assert!(b.s.pending_write(&bus_port()).is_some());

    // Another write, even the same one, is refused.
    b.bus.reply = BusReply::Done;
    let token = b.s.preflight(&write(10), b.now).unwrap();
    let got =
        b.s.invoke(&write(10), token, b.now, &mut b.bus, &mut b.interlock);
    assert!(matches!(got, Err(InvokeRefusal::OutcomeUnknown { .. })));
    assert_eq!(b.bus.performed.len(), 1);

    // A read back with no reply leaves it pending.
    b.bus.reply = BusReply::NoReply;
    let token = b.s.preflight(&read(), b.now).unwrap();
    let got =
        b.s.invoke(&read(), token, b.now, &mut b.bus, &mut b.interlock);
    assert_eq!(got, Ok(Outcome::NoReply));
    assert!(b.s.pending_write(&bus_port()).is_some());

    // A read back that answers opens the port again.
    b.bus.reply = BusReply::Value(WriteValue::U16(10));
    let token = b.s.preflight(&read(), b.now).unwrap();
    let got =
        b.s.invoke(&read(), token, b.now, &mut b.bus, &mut b.interlock);
    assert_eq!(
        got,
        Ok(Outcome::ReadBack {
            value: WriteValue::U16(10),
            applied: true
        })
    );
    assert!(b.s.pending_write(&bus_port()).is_none());
    b.bus.reply = BusReply::Done;
    let token = b.s.preflight(&write(12), b.now).unwrap();
    let got =
        b.s.invoke(&write(12), token, b.now, &mut b.bus, &mut b.interlock);
    assert_eq!(got, Ok(Outcome::Done));
}

#[test]
fn a_class_3_write_needs_a_panel_granted_lease_and_spends_it() {
    let mut b = Bench::new(HazardClass::Class3);
    b.arm();
    b.interlock.key_switch_on = true;
    record_facts(&mut b.s, HazardClass::Class3, b.now);

    // No lease yet: refused.
    let token = b.s.preflight(&write(10), b.now).unwrap();
    let target = token.decision().requirement().lease_target().unwrap();
    let got =
        b.s.invoke(&write(10), token, b.now, &mut b.bus, &mut b.interlock);
    assert_eq!(got, Err(InvokeRefusal::LeaseNotLive));

    // Ask, and a person presses the panel button.
    let token = b.s.preflight(&write(10), b.now).unwrap();
    assert_eq!(b.s.request_lease(&token, &mut b.interlock), Ok(target));
    b.tick();
    b.interlock.panel_button_pressed = true;
    b.tick();
    b.interlock.panel_button_pressed = false;
    b.tick();
    assert_eq!(b.interlock.outputs.rail_b, Some(target.port));

    // The token from before the ticks is still good: nothing changed.
    let got =
        b.s.invoke(&write(10), token, b.now, &mut b.bus, &mut b.interlock);
    assert_eq!(got, Ok(Outcome::Done));
    assert_eq!(
        b.interlock.requests.last(),
        Some(&ToInterlock::LeaseUsed { target })
    );
    b.tick();
    assert_eq!(b.interlock.outputs.rail_b, None);
}

#[test]
fn a_lease_is_not_requested_while_a_write_is_of_unknown_effect() {
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
    let got =
        b.s.invoke(&write(10), token, b.now, &mut b.bus, &mut b.interlock);
    assert_eq!(got, Ok(Outcome::Unknown));
    let token = b.s.preflight(&write(10), b.now).unwrap();
    assert!(matches!(
        b.s.request_lease(&token, &mut b.interlock),
        Err(InvokeRefusal::OutcomeUnknown { .. })
    ));
}

#[test]
fn the_config_refuses_ambiguous_enrollments() {
    use sawbones_policy::{BusFacts, LedgerId, WriteMap};
    use sawbones_supervisor::{Config, ConfigError, Supervisor};
    let mut second = meter_enrollment();
    second.carries = vec![bus_port()];
    let got = Supervisor::new(
        Config {
            ledger: LedgerId(1),
            enrollments: vec![pod_enrollment(), second],
            writes: WriteMap::default(),
            buses: BusFacts::default(),
        },
        Box::new(Counter(0)),
    );
    assert!(matches!(got, Err(ConfigError::SharedBusPort(_))));
}
