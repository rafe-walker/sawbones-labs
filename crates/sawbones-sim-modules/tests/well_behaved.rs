//! Well-behaved modules: every kind answers every call over the in-memory
//! link, identity verifies, hazard classes are the module's own, and a run is
//! the same every time.

mod common;

use common::*;
use sawbones_module_protocol::{
    Actuate, Channel, ConfigKey, ConfigWrite, Grant, HealthStatus, ImageDigest, ManifestHasher,
    Port, Request, ResetMode, ResponseKind, UpdateChunk,
};
use sawbones_sim_modules::{ModuleKind, Outcome, Summary};

fn kind_of(out: Outcome) -> ResponseKind {
    match out {
        Outcome::Replied(Summary { kind, .. }) => kind,
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_kind_opens_a_verified_session_and_serves_its_manifest() {
    for kind in ModuleKind::ALL {
        let mut b = open(kind);
        let mut hasher = ManifestHasher::new();
        for (page, text) in kind.manifest().iter().enumerate() {
            let out = b
                .host
                .call(&mut b.line, &Request::Describe { page: page as u8 }, 20);
            assert_eq!(kind_of(out), ResponseKind::Description, "{kind:?}");
            hasher.bytes(text);
        }
        assert_eq!(hasher.finish(), kind.manifest_digest(), "{kind:?}");
        // Out of range is a refusal, not a silence.
        let out = b
            .host
            .call(&mut b.line, &Request::Describe { page: 15 }, 20);
        assert_eq!(kind_of(out), ResponseKind::Refused);
        assert!(b.host.refused.is_empty());
    }
}

#[test]
fn the_comms_pod_is_the_only_kind_that_signs_can_emit() {
    for kind in ModuleKind::ALL {
        assert_eq!(
            kind.capabilities().can_emit,
            kind == ModuleKind::CommsPod,
            "{kind:?}"
        );
    }
}

#[test]
fn every_kind_answers_the_core_calls() {
    for kind in ModuleKind::ALL {
        let mut b = open(kind);
        let out = health(&mut b, 20);
        assert!(
            matches!(
                out,
                Outcome::Replied(Summary {
                    health: Some(HealthStatus::Ok),
                    ..
                })
            ),
            "{kind:?} {out:?}"
        );
        assert_eq!(
            kind_of(b.host.call(&mut b.line, &Request::SelfTest, 20)),
            ResponseKind::SelfTest
        );
        assert_eq!(
            kind_of(
                b.host
                    .call(&mut b.line, &Request::Time { host_time_us: 5 }, 20)
            ),
            ResponseKind::Time
        );
        let w = ConfigWrite::new(MODULE, ConfigKey(1), 12, Grant::None);
        assert_eq!(
            value(b.host.call(&mut b.line, &Request::ConfigWrite(w), 20)),
            12
        );
        assert_eq!(read(&mut b, 1, 20), read_again(&mut b));
        assert_eq!(
            kind_of(
                b.host
                    .call(&mut b.line, &Request::LogRead { from: 0, max: 2 }, 20)
            ),
            ResponseKind::Log
        );
        let chunk = Request::StageUpdate(UpdateChunk {
            image_digest: ImageDigest([1; 32]),
            image_len: 4,
            offset: 0,
            bytes: &[1, 2, 3, 4],
        });
        assert_eq!(
            kind_of(b.host.call(&mut b.line, &chunk, 20)),
            ResponseKind::UpdateStaged
        );
        // The simulation holds no trust root and never activates.
        let activate = Request::Reset(ResetMode::ActivateStaged {
            image_digest: ImageDigest([1; 32]),
            release: sawbones_module_protocol::ReleaseApproval {
                message: [0; 91],
                signature: [0; 64],
            },
        });
        assert_eq!(
            kind_of(b.host.call(&mut b.line, &activate, 20)),
            ResponseKind::Refused
        );
        assert_eq!(
            kind_of(b.host.call(&mut b.line, &Request::SafeState, 20)),
            ResponseKind::SafeStateEntered
        );
        assert!(b.host.refused.is_empty(), "{kind:?}");
    }
}

fn read_again(b: &mut Bench) -> Outcome {
    read(b, 1, 20)
}

#[test]
fn readings_are_plausible_and_move_with_noise() {
    let mut b = open(ModuleKind::DcMeter);
    let volts: Vec<i64> = (0..8).map(|_| value(read(&mut b, 0x100, 20))).collect();
    assert!(
        volts.iter().all(|v| (11_990..=12_010).contains(v)),
        "{volts:?}"
    );
    assert!(
        volts.iter().any(|v| *v != volts[0]),
        "never changes: {volts:?}"
    );
    let amps = value(read(&mut b, 0x101, 20));
    assert!((1_490..=1_510).contains(&amps));
    // No such reading.
    assert_eq!(kind_of(read(&mut b, 0x1ff, 20)), ResponseKind::Refused);
    // The pod's frame counter and the controller's heartbeat count up.
    let mut pod = open(ModuleKind::CommsPod);
    let a = value(read(&mut pod, 0x100, 20));
    pod.host.idle(&mut pod.line, 1500);
    // The module's own link timeout may have ended the session; reopen.
    if matches!(health(&mut pod, 5), Outcome::Timeout) {
        assert!(matches!(
            pod.host.open(&mut pod.line, 20),
            Outcome::Replied(_)
        ));
    }
    assert!(value(read(&mut pod, 0x100, 20)) > a);
}

#[test]
fn the_module_decides_the_class_and_the_host_authority_has_to_reach_it() {
    let mut b = open(ModuleKind::InterlockController);
    // Channel 0 is class 2: no authority is refused, rail A works.
    let none = Actuate::new(MODULE, Channel(0), 5, Grant::None);
    assert_eq!(
        kind_of(b.host.call(&mut b.line, &Request::Actuate(none), 20)),
        ResponseKind::Refused
    );
    assert_eq!(b.line.module.output, [0, 0]);
    assert_eq!(kind_of(actuate(&mut b, 0, 5, 20)), ResponseKind::Actuated);
    // Channel 1 is class 3: rail A is not enough, a lease for this action is.
    assert_eq!(kind_of(actuate(&mut b, 1, 7, 20)), ResponseKind::Refused);
    let lease = Actuate::new(MODULE, Channel(1), 7, Grant::Lease { port: Port(1) });
    assert_eq!(
        kind_of(b.host.call(&mut b.line, &Request::Actuate(lease), 20)),
        ResponseKind::Actuated
    );
    assert_eq!(b.line.module.output, [5, 7]);
    // Safe state zeroes both outputs.
    let _ = b.host.call(&mut b.line, &Request::SafeState, 20);
    assert_eq!(b.line.module.output, [0, 0]);
    // Class 4 is never done, and an unknown channel is refused.
    let w = ConfigWrite::new(MODULE, ConfigKey(9), 1, Grant::Lease { port: Port(0) });
    assert_eq!(
        kind_of(b.host.call(&mut b.line, &Request::ConfigWrite(w), 20)),
        ResponseKind::Refused
    );
    assert_eq!(kind_of(actuate(&mut b, 5, 1, 20)), ResponseKind::Refused);
    assert_eq!(b.line.module.executed, 2);
}

#[test]
fn a_reset_ends_the_session_and_the_module_must_be_opened_again() {
    let mut b = open(ModuleKind::DcMeter);
    assert_eq!(
        kind_of(
            b.host
                .call(&mut b.line, &Request::Reset(ResetMode::Restart), 20)
        ),
        ResponseKind::ResetAccepted
    );
    assert!(b.line.module.session().is_none());
    assert_eq!(health(&mut b, 20), Outcome::Timeout);
    assert!(matches!(b.host.open(&mut b.line, 20), Outcome::Replied(_)));
    assert!(matches!(health(&mut b, 20), Outcome::Replied(_)));
}

#[test]
fn a_silent_module_goes_to_its_safe_state_by_itself() {
    let mut b = open(ModuleKind::InterlockController);
    assert_eq!(kind_of(actuate(&mut b, 0, 9, 20)), ResponseKind::Actuated);
    b.host.idle(&mut b.line, 1500);
    assert_eq!(b.line.module.output, [0, 0]);
    assert!(b.line.module.session().is_none());
    // Its session is gone, so the host's next request is not answered.
    assert_eq!(health(&mut b, 20), Outcome::Timeout);
    assert!(!b.host.link.busy());
}

#[test]
fn the_same_seeds_give_the_same_run() {
    fn run() -> (Vec<Vec<u8>>, Vec<Vec<u8>>, u64) {
        let mut b = open(ModuleKind::DcMeter);
        for key in [0x100, 0x101, 0x100] {
            let _ = read(&mut b, key, 20);
        }
        let _ = health(&mut b, 20);
        (
            b.line
                .recorded(sawbones_sim_modules::Dir::ToModule)
                .to_vec(),
            b.line.recorded(sawbones_sim_modules::Dir::ToHost).to_vec(),
            b.host.now,
        )
    }
    assert_eq!(run(), run());
    // A different module seed is a different run (other session ids).
    let mut other = Bench {
        host: host_for(ModuleKind::DcMeter),
        line: sawbones_sim_modules::SimLink::new(
            sawbones_sim_modules::SimModule::new(ModuleKind::DcMeter, MODULE, DEVICE_SEED, 99),
            None,
        ),
    };
    assert!(matches!(
        other.host.open(&mut other.line, 20),
        Outcome::Replied(_)
    ));
    let _ = read(&mut other, 0x100, 20);
    assert_ne!(
        run().1[0],
        other.line.recorded(sawbones_sim_modules::Dir::ToHost)[0]
    );
}
