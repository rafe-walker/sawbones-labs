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
    let got =
        b.s.invoke(&write(10), token, b.now, &mut b.bus, &mut b.interlock);
    assert_eq!(got, Ok(Outcome::Unknown));

    // The agent retries: no new lease, no write.
    let token = b.s.preflight(&write(10), b.now).unwrap();
    assert!(matches!(
        b.s.request_lease(&token, &mut b.interlock),
        Err(InvokeRefusal::OutcomeUnknown { .. })
    ));
    let got =
        b.s.invoke(&write(10), token, b.now, &mut b.bus, &mut b.interlock);
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
    let got =
        b.s.invoke(&read, token, b.now, &mut b.bus, &mut b.interlock);
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
