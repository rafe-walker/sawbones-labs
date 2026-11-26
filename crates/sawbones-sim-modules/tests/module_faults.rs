//! The fault switches on the module itself (#99): stuck value, drift,
//! disconnected input, stale data, wrong identity, slow reply, lost
//! heartbeat. These are not protocol violations: the frames are valid and the
//! host link accepts them. They are what the supervisor's plausibility checks
//! and the diagnosis benchmark have to catch, so the tests pin what the
//! simulated module does, deterministically.

mod common;

use common::*;
use sawbones_module_protocol::{HealthStatus, IdentityError};
use sawbones_sim_modules::{ModuleFaults, ModuleKind, Outcome, Summary};

fn faulty(faults: ModuleFaults) -> Bench {
    let mut b = open(ModuleKind::DcMeter);
    let now = b.host.now + 1;
    b.host.now = now;
    b.line.module.set_faults(now, faults);
    b
}

#[test]
fn a_stuck_value_never_changes_and_the_module_still_says_ok() {
    let mut b = faulty(ModuleFaults {
        stuck: true,
        ..ModuleFaults::default()
    });
    let first = value(read(&mut b, 0x100, 20));
    for _ in 0..20 {
        assert_eq!(value(read(&mut b, 0x100, 20)), first);
    }
    assert!(matches!(
        health(&mut b, 20),
        Outcome::Replied(Summary {
            health: Some(HealthStatus::Ok),
            ..
        })
    ));
    assert!(b.host.refused.is_empty());
}

#[test]
fn drift_walks_away_from_the_truth_at_a_fixed_rate() {
    let mut b = faulty(ModuleFaults {
        drift_per_tick: 3,
        ..ModuleFaults::default()
    });
    let onset = b.host.now;
    b.host.idle(&mut b.line, 100);
    let v = value(read(&mut b, 0x100, 20));
    let truth = ModuleKind::DcMeter
        .truth(MODULE_RNG, 0x100, b.host.now - 1)
        .unwrap();
    // One tick of latency between sending and the module reading its input.
    let elapsed = (b.host.now - 1 - onset) as i64;
    assert!(
        (v - truth - 3 * elapsed).abs() <= 12,
        "{v} {truth} {elapsed}"
    );
    assert!(v - 12_000 > 250, "{v}");
}

#[test]
fn a_disconnected_input_reads_zero_and_the_module_reports_degraded() {
    let mut b = faulty(ModuleFaults {
        disconnected: true,
        ..ModuleFaults::default()
    });
    assert_eq!(value(read(&mut b, 0x100, 20)), 0);
    assert_eq!(value(read(&mut b, 0x101, 20)), 0);
    assert!(matches!(
        health(&mut b, 20),
        Outcome::Replied(Summary {
            health: Some(HealthStatus::Degraded),
            ..
        })
    ));
}

#[test]
fn stale_data_is_the_old_value_with_no_flag() {
    let mut b = faulty(ModuleFaults {
        stale_ticks: 50,
        ..ModuleFaults::default()
    });
    b.host.idle(&mut b.line, 100);
    let v = value(read(&mut b, 0x100, 20));
    let at = b.host.now - 1;
    let old = ModuleKind::DcMeter
        .truth(MODULE_RNG, 0x100, at - 50)
        .unwrap();
    assert_eq!(v, old);
    assert!(matches!(
        health(&mut b, 20),
        Outcome::Replied(Summary {
            health: Some(HealthStatus::Ok),
            ..
        })
    ));
}

#[test]
fn a_wrong_identity_is_not_trusted_and_nothing_else_is_sent() {
    let mut line = sawbones_sim_modules::SimLink::new(module(ModuleKind::DcMeter), None);
    line.module.set_faults(
        0,
        ModuleFaults {
            wrong_identity: Some(OTHER),
            ..ModuleFaults::default()
        },
    );
    let mut host = host_for(ModuleKind::DcMeter);
    let out = host.open(&mut line, 20);
    assert!(
        matches!(
            out,
            Outcome::IdentityRejected(IdentityError::WrongModule { .. })
        ),
        "{out:?}"
    );
    assert_eq!(
        host.call(&mut line, &sawbones_module_protocol::Request::Health, 20),
        Outcome::NotTrusted
    );
}

#[test]
fn a_slow_reply_misses_a_short_deadline_and_makes_a_long_one() {
    let mut b = faulty(ModuleFaults {
        slow_reply_ticks: 40,
        ..ModuleFaults::default()
    });
    assert_eq!(health(&mut b, 20), Outcome::Timeout);
    // The late reply arrives with nothing asked for.
    b.host.idle(&mut b.line, 60);
    assert_eq!(
        b.host.refused,
        [sawbones_module_protocol::HostError::Unsolicited]
    );
    assert!(matches!(health(&mut b, 100), Outcome::Replied(_)));
}

#[test]
fn a_lost_heartbeat_answers_everything_but_health() {
    let mut b = faulty(ModuleFaults {
        lost_heartbeat: true,
        ..ModuleFaults::default()
    });
    for _ in 0..3 {
        assert_eq!(health(&mut b, 20), Outcome::Timeout);
    }
    assert!(matches!(read(&mut b, 0x100, 20), Outcome::Replied(_)));
    assert!(!b.host.link.busy());
    assert!(b.host.refused.is_empty());
}

#[test]
fn faults_can_be_switched_off_again() {
    let mut b = faulty(ModuleFaults {
        stuck: true,
        lost_heartbeat: true,
        ..ModuleFaults::default()
    });
    assert_eq!(health(&mut b, 10), Outcome::Timeout);
    let now = b.host.now;
    b.line.module.set_faults(now, ModuleFaults::default());
    assert!(matches!(health(&mut b, 10), Outcome::Replied(_)));
    assert_eq!(b.line.module.faults(), ModuleFaults::default());
}
