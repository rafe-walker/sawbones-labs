//! Module link attacks (lane `interlock-hardware`, cards hw-031 to hw-036),
//! driven through the simulated modules in `sawbones-sim-modules`: the real
//! module protocol on an in-memory line with a named fault on it. No clock,
//! no thread, no hardware: every run is the same for the same seeds.

use ed25519_dalek::SigningKey;
use sawbones_module_protocol::{
    decode_request, decode_response, frame_len, Actuate, Channel, DecodeError, Dropped, Grant,
    HostError, IdentityError, ModuleId, ModuleKey, Rejection, Request, MAX_PAYLOAD_LEN,
};
use sawbones_sim_modules::{
    scenarios, Dir, Fault, HostHarness, ModuleKind, Outcome, Scenario, SimLink, SimModule,
};

/// TEST ONLY. The interlock controller's device key seed.
const DEVICE_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY redteam-link-1";
/// TEST ONLY. A second device's key seed.
const IMPOSTOR_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY redteam-link-2";
const SEED: u64 = 0x7e57_0099;
const ID: ModuleId = ModuleId(1);
const OTHER: ModuleId = ModuleId(2);
const FROM: u64 = 100;
const KIND: ModuleKind = ModuleKind::InterlockController;

fn key(seed: &[u8; 32]) -> ModuleKey {
    ModuleKey(SigningKey::from_bytes(seed).verifying_key().to_bytes())
}

fn host() -> HostHarness {
    HostHarness::new(ID, key(&DEVICE_SEED), KIND.capabilities(), SEED)
}

fn named(name: &str) -> Scenario {
    scenarios(SEED, FROM, OTHER)
        .into_iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no scenario {name}"))
}

/// An open, trusted session, before the scenario's first tick.
fn opened(scenario: Scenario) -> (HostHarness, SimLink) {
    let mut line = SimLink::new(
        SimModule::new(KIND, ID, DEVICE_SEED, SEED).with_link_timeout(1000),
        Some(scenario),
    );
    let mut host = host();
    assert!(matches!(host.open(&mut line, 20), Outcome::Replied(_)));
    assert!(host.trusted());
    (host, line)
}

/// The same, with the clock at the scenario's first tick.
fn armed(scenario: Scenario) -> (HostHarness, SimLink) {
    let (mut host, line) = opened(scenario);
    host.now = FROM;
    (host, line)
}

fn actuate(host: &mut HostHarness, line: &mut SimLink, setting: i32) -> Outcome {
    let a = Actuate::new(ID, Channel(0), setting, Grant::RailA);
    host.call(line, &Request::Actuate(a), 10)
}

#[test]
fn redteam_hw_031_a_replayed_identify_only_forces_a_safe_state() {
    let (mut host, mut line) = opened(named("identify_replay"));
    assert!(matches!(
        actuate(&mut host, &mut line, 500),
        Outcome::Replied(_)
    ));
    assert_eq!(line.module.output[0], 500);
    let entries = line.module.safe_entries;
    host.idle(&mut line, FROM + 3 - host.now);
    // The attack lands: the module opened a new session and went safe.
    assert_eq!(line.module.safe_entries, entries + 1);
    assert_eq!(line.module.output, [0, 0]);
    // The host's next command is dropped, not run, and the host is never
    // left waiting.
    assert_eq!(actuate(&mut host, &mut line, 600), Outcome::Timeout);
    assert_eq!(line.module.output, [0, 0]);
    assert!(line
        .module
        .rejections
        .contains(&Rejection::Drop(Dropped::WrongSession)));
    assert!(!host.link.busy());
    // Repeated, it is a denial of service: every replay costs the module its
    // output and the host its session, so the supervisor has to notice (#100).
    let captured = line.recorded(Dir::ToModule)[0].clone();
    assert!(matches!(
        decode_request(&captured).map(|f| f.request),
        Ok(Request::Identify { .. })
    ));
    for _ in 0..5 {
        let before = line.module.safe_entries;
        line.inject_to_module(host.now, &captured);
        assert_eq!(line.module.safe_entries, before + 1);
        assert_eq!(line.module.output, [0, 0]);
    }
    // The host recovers by opening a session and verifying it again.
    assert!(matches!(host.open(&mut line, 20), Outcome::Replied(_)));
    assert!(host.trusted());
}

#[test]
fn redteam_hw_032_a_second_device_cannot_answer_under_an_enrolled_id() {
    // The impostor holds its own key and says it is the enrolled module.
    let impostor = SimModule::new(KIND, OTHER, IMPOSTOR_SEED, SEED).with_link_timeout(1000);
    let claim = Scenario::new(
        "claim_enrolled_id",
        SEED,
        Dir::ToHost,
        0,
        Fault::ClaimOtherId { as_id: ID },
    );
    let mut line = SimLink::new(impostor, Some(claim));
    let mut host = host();
    let out = host.open(&mut line, 20);
    assert_eq!(out, Outcome::IdentityRejected(IdentityError::BadSignature));
    assert!(!host.trusted());
    // Untrusted, the host sends it nothing but another Identify, so no
    // command reaches the impostor.
    let a = Actuate::new(ID, Channel(0), 1, Grant::RailA);
    assert_eq!(
        host.call(&mut line, &Request::Actuate(a), 10),
        Outcome::NotTrusted
    );
    assert_eq!(line.module.executed, 0);
    // The genuine module is unaffected.
    let mut line = SimLink::new(SimModule::new(KIND, ID, DEVICE_SEED, SEED), None);
    assert!(matches!(host.open(&mut line, 20), Outcome::Replied(_)));
    assert!(host.trusted());
}

#[test]
fn redteam_hw_033_a_flood_gets_no_frame_accepted_and_the_call_gives_up() {
    // Toward the host: the junk is all refused, the call gives up at its
    // budget, nothing is left outstanding.
    let (mut host, mut line) = armed(named("babble_to_host"));
    host.flood_limit = 16;
    assert_eq!(host.call(&mut line, &Request::Health, 50), Outcome::Flooded);
    assert!(host.refused.len() > 16);
    assert!(!host.link.busy());
    assert_eq!(line.module.executed, 0);

    // Toward the module: forged frames are never commands.
    let (mut host, mut line) = armed(named("babble_to_module"));
    let mut genuine = 0;
    for setting in 0..10 {
        if matches!(actuate(&mut host, &mut line, setting), Outcome::Replied(_)) {
            genuine += 1;
        }
    }
    assert_eq!(line.module.executed, genuine);
    assert!(line.module.counters().dropped + line.module.counters().bad_frames > 100);
}

#[test]
fn redteam_hw_034_an_oversized_length_is_refused_everywhere() {
    for declared in [MAX_PAYLOAD_LEN as u16 + 1, 1000, u16::MAX] {
        for fix_crc in [false, true] {
            for dir in [Dir::ToModule, Dir::ToHost] {
                let s = Scenario::new(
                    "oversized",
                    SEED,
                    dir,
                    FROM,
                    Fault::Oversize { declared, fix_crc },
                );
                let (mut host, mut line) = armed(s);
                assert_eq!(actuate(&mut host, &mut line, 5), Outcome::Timeout);
                assert!(!host.link.busy());
                match dir {
                    Dir::ToModule => {
                        assert_eq!(line.module.executed, 0);
                        assert!(matches!(
                            line.module.rejections.last(),
                            Some(Rejection::Drop(Dropped::Malformed(
                                DecodeError::Oversized { .. }
                            )))
                        ));
                    }
                    Dir::ToHost => assert!(host
                        .refused
                        .iter()
                        .all(|e| matches!(e, HostError::Malformed(DecodeError::Oversized { .. })))),
                }
            }
        }
    }
    // A stream reader sizing its read from the prefix is told no up front.
    let mut header = b"SB\x01\x81".to_vec();
    header.extend_from_slice(&u16::MAX.to_le_bytes());
    assert_eq!(
        frame_len(&header),
        Err(DecodeError::Oversized { declared: 65535 })
    );
    assert!(decode_response(&header).is_err());
    assert!(decode_request(&header).is_err());
}

#[test]
fn redteam_hw_035_a_stuck_line_times_out_and_the_module_goes_safe_by_itself() {
    // Replies stop. The host gives up on each call; the module, hearing
    // nothing more, ends its session and goes safe at its link timeout.
    let (mut host, mut line) = armed(named("silent_module"));
    assert_eq!(actuate(&mut host, &mut line, 9), Outcome::Timeout);
    assert_eq!(line.module.output[0], 9);
    assert!(!host.link.busy());
    host.idle(&mut line, 1500);
    assert_eq!(line.module.output, [0, 0]);
    assert!(line.module.session().is_none());

    // The line stuck at one level: every tick is a refused non-frame.
    let (mut host, mut line) = armed(named("stuck_line_high"));
    assert_eq!(host.call(&mut line, &Request::Health, 20), Outcome::Timeout);
    assert!(host
        .refused
        .iter()
        .all(|e| matches!(e, HostError::Malformed(DecodeError::BadMagic))));
    assert!(!host.link.busy());
}

#[test]
fn redteam_hw_036_frames_replayed_across_a_reboot_do_not_run() {
    let (mut host, mut line) = opened(named("replay_across_reboot"));
    assert!(matches!(
        actuate(&mut host, &mut line, 77),
        Outcome::Replied(_)
    ));
    let executed = line.module.executed;
    // The scenario fires at its first tick: it reboots the module and replays
    // everything recorded, both ways.
    host.idle(&mut line, FROM + 3 - host.now);
    assert!(line.module.session().is_none());
    assert_eq!(line.module.output, [0, 0]);
    assert!(line
        .module
        .rejections
        .contains(&Rejection::Drop(Dropped::NoSession)));
    assert_eq!(line.module.executed, executed);
    // Old replies reach a host that asked for nothing.
    assert!(host.refused.contains(&HostError::Unsolicited));
    // A recorded command replayed after a new session opens names the old
    // session, so it is dropped.
    let old = line.recorded(Dir::ToModule)[1].clone();
    assert!(matches!(
        decode_request(&old).map(|f| f.request),
        Ok(Request::Actuate(_))
    ));
    assert!(matches!(host.open(&mut line, 20), Outcome::Replied(_)));
    line.inject_to_module(host.now, &old);
    assert_eq!(line.module.output, [0, 0]);
    assert_eq!(line.module.executed, executed);
}
