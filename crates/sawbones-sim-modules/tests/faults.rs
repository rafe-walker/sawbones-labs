//! One test per fault. Each shows what the protocol layer (the module's
//! `ModuleLink`, the host's `HostLink` and the frame decoder) does about it
//! and that the run ends in a safe state: nothing hazardous executed that
//! should not have, no request left hanging, no panic.
//!
//! Where the right response is the supervisor's (#100, not built yet), the
//! test asserts what the protocol layer guarantees and a doc comment says what
//! #100 must add. [`sawbones_sim_modules::HostHarness`] stands in for the
//! supervisor only as far as a timeout, a refusal budget and the identity
//! check; those are the parts #100 takes over.

mod common;

use common::*;
use sawbones_module_protocol::{
    decode_response, frame_len, DecodeError, Dropped, HostError, IdentityError, Request,
    ResponseKind, MAX_PAYLOAD_LEN,
};
use sawbones_sim_modules::{
    scenarios, Dir, Fault, ModuleKind, Outcome, Scenario, SimLink, Summary,
};

/// The window the transport faults are on for, so a test can also show the
/// link recovers when it ends.
const WINDOW: u64 = 600;

fn windowed(name: &str) -> Bench {
    armed_scenario(scenario(name).until(FROM + WINDOW))
}

fn replied(out: Outcome) -> Summary {
    match out {
        Outcome::Replied(s) => s,
        other => panic!("expected a reply, got {other:?}"),
    }
}

/// After the window the same session still works.
fn recovered(b: &mut Bench) {
    let gap = (FROM + WINDOW + 1).saturating_sub(b.host.now);
    b.host.idle(&mut b.line, gap);
    if b.line.module.session().is_none() {
        assert!(matches!(b.host.open(&mut b.line, 20), Outcome::Replied(_)));
    }
    let out = health(b, 20);
    assert!(
        matches!(
            out,
            Outcome::Replied(Summary {
                kind: ResponseKind::Health,
                ..
            })
        ),
        "{out:?}"
    );
    assert!(!b.host.link.busy());
}

#[test]
fn every_scenario_runs_to_a_safe_end_for_many_seeds() {
    // The catch-all: any fault, any seed, a mix of calls. Nothing panics,
    // every call ends, and nothing is left outstanding.
    let names: Vec<_> = scenarios(0, FROM, OTHER).iter().map(|s| s.name).collect();
    assert!(names.len() >= 20);
    let mut unique = names.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "scenario names are unique");
    for seed in 0..6u64 {
        for s in scenarios(seed, FROM, OTHER) {
            let mut b = with(ModuleKind::InterlockController, s, false);
            let _ = b.host.open(&mut b.line, 15);
            b.host.idle(&mut b.line, FROM.saturating_sub(b.host.now));
            for round in 0..6 {
                let _ = health(&mut b, 15);
                let _ = actuate(&mut b, 0, round, 15);
                let _ = read(&mut b, 0x100, 15);
                if round % 2 == 1 {
                    let _ = b.host.open(&mut b.line, 15);
                }
            }
            b.host.idle(&mut b.line, 700);
            assert!(!b.host.link.busy(), "{} seed {seed}", s.name);
            // The module's safe state is always reachable by a trusted
            // session or by its own link timeout.
            assert!(
                b.line.module.output == [0, 0] || b.line.module.session().is_some(),
                "{} seed {seed}: outputs live with no session",
                s.name
            );
        }
    }
}

#[test]
fn bit_flips_to_the_host_fail_the_check_and_change_nothing() {
    let mut b = windowed("bit_flip_to_host");
    let session = b.host.link.session();
    for _ in 0..20 {
        assert_eq!(health(&mut b, 10), Outcome::Timeout);
    }
    // CRC-32C catches every error of up to 3 bits at these lengths, so every
    // frame is refused as malformed and none is half-believed.
    assert!(b.host.refused.len() >= 20);
    assert!(b
        .host
        .refused
        .iter()
        .all(|e| matches!(e, HostError::Malformed(_))));
    assert_eq!(b.host.link.session(), session);
    assert!(!b.host.link.busy());
    recovered(&mut b);
}

#[test]
fn bit_flips_to_the_module_are_dropped_and_nothing_executes() {
    let mut b = windowed("bit_flip_to_module");
    for setting in 1..=10 {
        assert_eq!(actuate(&mut b, 0, setting, 10), Outcome::Timeout);
    }
    assert_eq!(b.line.module.executed, 0);
    assert_eq!(b.line.module.output, [0, 0]);
    assert!(b.line.module.counters().bad_frames >= 10);
    assert!(b.line.module.rejections.iter().all(|r| matches!(
        r,
        sawbones_module_protocol::Rejection::Drop(Dropped::Malformed(_))
    )));
    // The sequence numbers the host spent are simply skipped.
    recovered(&mut b);
}

#[test]
fn truncated_frames_are_refused_by_both_ends() {
    let mut b = windowed("truncate_to_host");
    for _ in 0..20 {
        assert_eq!(health(&mut b, 10), Outcome::Timeout);
    }
    assert!(b
        .host
        .refused
        .iter()
        .all(|e| matches!(e, HostError::Malformed(DecodeError::Truncated { .. }))));
    recovered(&mut b);

    let mut b = windowed("truncate_to_module");
    for setting in 1..=20 {
        assert_eq!(actuate(&mut b, 0, setting, 10), Outcome::Timeout);
    }
    assert_eq!(b.line.module.executed, 0);
    assert!(b.line.module.counters().bad_frames >= 20);
    recovered(&mut b);
}

/// A reply lost after the module acted: the host sees a timeout and does not
/// know whether the command ran. #100 must read the state back before it
/// retries a command that is not idempotent (#98 design notes).
#[test]
fn a_lost_reply_leaves_the_command_done_and_the_host_unsure() {
    let mut b = windowed("drop_all_to_host");
    assert_eq!(actuate(&mut b, 0, 321, 10), Outcome::Timeout);
    assert_eq!(b.line.module.output[0], 321, "the module did act");
    assert_eq!(b.line.module.executed, 1);
    assert!(!b.host.link.busy());
    // The safe state is still one call away when the line is back.
    b.host.idle(&mut b.line, WINDOW);
    let out = b.host.call(&mut b.line, &Request::SafeState, 10);
    assert_eq!(replied(out).kind, ResponseKind::SafeStateEntered);
    assert_eq!(b.line.module.output, [0, 0]);
}

#[test]
fn requests_lost_on_the_way_never_execute() {
    let mut b = windowed("drop_all_to_module");
    let reached = b.line.reached_module;
    for setting in 1..=5 {
        assert_eq!(actuate(&mut b, 0, setting, 10), Outcome::Timeout);
    }
    assert_eq!(b.line.reached_module, reached);
    assert_eq!(b.line.module.executed, 0);
    assert_eq!(b.host.timeouts, 5);
    recovered(&mut b);
}

#[test]
fn duplicated_replies_are_refused_as_unsolicited() {
    let mut b = windowed("duplicate_to_host");
    assert_eq!(replied(health(&mut b, 10)).kind, ResponseKind::Health);
    b.host.idle(&mut b.line, 3);
    assert_eq!(
        b.host.refused,
        [HostError::Unsolicited, HostError::Unsolicited]
    );
    // A duplicate that lands while the next request is out is refused too,
    // and the real reply still gets through.
    let before = b.host.refused.len();
    assert_eq!(replied(health(&mut b, 10)).kind, ResponseKind::Health);
    let _ = health(&mut b, 10);
    assert!(b.host.refused.len() > before);
    assert!(b
        .host
        .refused
        .iter()
        .all(|e| matches!(e, HostError::Unsolicited | HostError::WrongReply { .. })));
    assert!(!b.host.link.busy());
}

#[test]
fn a_duplicated_command_executes_once() {
    let mut b = windowed("duplicate_to_module");
    assert_eq!(
        replied(actuate(&mut b, 0, 5, 10)).kind,
        ResponseKind::Actuated
    );
    assert_eq!(b.line.module.executed, 1);
    assert!(
        b.line
            .module
            .dropped_for(|d| matches!(d, Dropped::Replayed { .. }))
            >= 2
    );
    // The module sent one reply; the host saw one.
    b.host.idle(&mut b.line, 3);
    assert!(b.host.refused.is_empty());
}

#[test]
fn reordered_replies_do_not_confuse_the_host() {
    let mut b = windowed("reorder_to_host");
    // Reply A is held back and arrives after reply B.
    assert_eq!(health(&mut b, 10), Outcome::Timeout);
    assert_eq!(replied(health(&mut b, 10)).kind, ResponseKind::Health);
    b.host.idle(&mut b.line, 3);
    assert_eq!(b.host.refused, [HostError::Unsolicited]);
    assert!(!b.host.link.busy());
}

#[test]
fn reordered_commands_never_run_out_of_order() {
    let mut b = windowed("reorder_to_module");
    // Command A (setting 1) is held back and arrives after B (setting 2). A
    // is older than the last frame accepted, so the module drops it: the
    // final state is B's, never A's.
    assert_eq!(actuate(&mut b, 0, 1, 10), Outcome::Timeout);
    assert_eq!(
        replied(actuate(&mut b, 0, 2, 10)).kind,
        ResponseKind::Actuated
    );
    assert_eq!(b.line.module.output[0], 2);
    assert_eq!(b.line.module.executed, 1);
    assert!(
        b.line
            .module
            .dropped_for(|d| matches!(d, Dropped::Replayed { .. }))
            >= 1
    );
}

/// A reply later than the deadline is a timeout, and the late reply is
/// refused when it comes. The command ran anyway. #100 owns the deadline
/// policy and the read-back.
#[test]
fn a_reply_past_the_deadline_is_a_timeout_and_the_late_one_is_refused() {
    let mut b = windowed("delay_past_deadline");
    assert_eq!(actuate(&mut b, 0, 7, 20), Outcome::Timeout);
    assert_eq!(b.line.module.executed, 1);
    assert!(!b.host.link.busy());
    b.host.idle(&mut b.line, 600);
    assert!(b.host.refused.contains(&HostError::Unsolicited));
    // A second late reply landing during a new request does not satisfy it.
    recovered(&mut b);
}

/// A babbling module cannot get a frame accepted, but the protocol has no
/// way to make it stop or to tell the supervisor how loud it is. #100 must
/// count refused frames per module, rate limit them, and quarantine a
/// module that babbles.
#[test]
fn a_babbling_module_is_refused_frame_by_frame_and_the_call_gives_up() {
    let mut b = windowed("babble_to_host");
    b.host.flood_limit = 16;
    assert_eq!(health(&mut b, 50), Outcome::Flooded);
    assert!(!b.host.link.busy());
    assert!(b.host.refused.len() > 16);
    // With a bigger budget the genuine reply still arrives, and every junk
    // frame was refused, never accepted.
    b.host.flood_limit = 1000;
    let before = b.host.refused.len();
    assert_eq!(replied(health(&mut b, 50)).kind, ResponseKind::Health);
    assert!(b.host.refused.len() >= before + 30);
    assert_eq!(b.line.module.executed, 0);
}

/// Frames that name the open session (it is visible on the line) and carry a
/// high sequence number make the module drop every later genuine request as
/// a replay: a denial of service, not a hazard. Nothing executes. The host
/// recovers by opening a new session. #100 notices the timeouts; #172 (signed
/// commands) is what stops forged frames from moving the sequence number.
#[test]
fn babbling_at_the_module_executes_nothing_and_a_new_session_recovers() {
    let mut b = windowed("babble_to_module");
    let mut genuine = 0;
    for round in 0..10 {
        if matches!(actuate(&mut b, 0, round, 10), Outcome::Replied(_)) {
            genuine += 1;
        }
    }
    // Only the host's own commands ever ran: forged frames are config reads.
    assert_eq!(b.line.module.executed, genuine);
    // The forged frames wedged the session: later genuine requests were
    // dropped as replays.
    assert!(genuine < 10, "{genuine} of 10 got through");
    assert!(b.line.module.counters().dropped + b.line.module.counters().bad_frames > 100);
    b.host.idle(&mut b.line, WINDOW);
    // After the window the old session may be wedged; a new one is not.
    assert!(matches!(b.host.open(&mut b.line, 20), Outcome::Replied(_)));
    assert_eq!(
        replied(actuate(&mut b, 0, 3, 20)).kind,
        ResponseKind::Actuated
    );
    assert_eq!(b.line.module.executed, genuine + 1);
}

#[test]
fn an_old_request_replayed_after_a_newer_one_is_dropped() {
    let mut b = windowed("replay_old_to_module");
    assert_eq!(
        replied(actuate(&mut b, 0, 1, 10)).kind,
        ResponseKind::Actuated
    );
    assert_eq!(
        replied(actuate(&mut b, 0, 2, 10)).kind,
        ResponseKind::Actuated
    );
    // The line replayed setting 1 after setting 2. The module dropped it.
    assert_eq!(b.line.module.output[0], 2);
    assert_eq!(b.line.module.executed, 2);
    assert!(
        b.line
            .module
            .dropped_for(|d| matches!(d, Dropped::Replayed { .. }))
            >= 1
    );
}

#[test]
fn an_old_reply_replayed_to_the_host_is_refused() {
    let mut b = windowed("replay_old_to_host");
    for _ in 0..4 {
        assert_eq!(replied(health(&mut b, 10)).kind, ResponseKind::Health);
    }
    b.host.idle(&mut b.line, 3);
    assert!(b.host.refused.len() >= 3);
    assert!(b
        .host
        .refused
        .iter()
        .all(|e| matches!(e, HostError::Unsolicited | HostError::WrongReply { .. })));
    assert!(!b.host.link.busy());
}

#[test]
fn frames_replayed_across_a_reboot_are_dropped_and_the_module_stays_safe() {
    // The session is open and a command has been given. The module reboots
    // and every old frame is replayed, both ways.
    let mut b = with(
        ModuleKind::InterlockController,
        scenario("replay_across_reboot"),
        true,
    );
    assert_eq!(
        replied(actuate(&mut b, 0, 77, 10)).kind,
        ResponseKind::Actuated
    );
    let old_session = b.host.link.session().unwrap();
    let executed = b.line.module.executed;
    b.host.idle(&mut b.line, FROM + 5 - b.host.now);
    // Old requests: no session after the reboot, so they are dropped.
    assert!(b.line.module.session().is_none());
    assert!(
        b.line
            .module
            .dropped_for(|d| matches!(d, Dropped::NoSession))
            >= 1
    );
    assert_eq!(b.line.module.executed, executed);
    assert_eq!(b.line.module.output, [0, 0]);
    // Old replies: nothing was asked, so the host refuses them.
    assert!(b.host.refused.contains(&HostError::Unsolicited));
    // The host's old session is dead; it times out rather than hang, and a
    // new session gets a new id.
    assert_eq!(health(&mut b, 10), Outcome::Timeout);
    assert!(matches!(b.host.open(&mut b.line, 20), Outcome::Replied(_)));
    assert_ne!(b.host.link.session().unwrap(), old_session);
    // An old reply replayed into the new session names the old one.
    let old_reply = b.line.recorded(Dir::ToHost)[1].clone();
    let _ = b.host.link.send(&Request::Health, &mut [0u8; 400]);
    b.line.inject_to_host(b.host.now, old_reply);
    b.host.idle(&mut b.line, 2);
    assert!(b
        .host
        .refused
        .iter()
        .any(|e| matches!(e, HostError::WrongSession | HostError::WrongReply { .. })));
    b.host.link.abandon();
}

/// The limit of session ids as replay protection. If the module's session
/// ids repeat (a weak or reseeded generator, which the firmware contract
/// forbids) and the host restarts its sequence numbers, a recorded command
/// is accepted into the new session. The protocol layer alone does not stop
/// it; what does is a hardware random session id (firmware contract) and
/// signed commands (#172), which bind a command to the session and a fresh
/// host value. This test pins today's behaviour so #172 has to change it.
#[test]
fn replay_across_a_reboot_with_a_weak_session_generator_is_accepted_today() {
    let s = scenario("replay_across_reboot_weak_rng");
    let mut b = with(ModuleKind::InterlockController, s, true);
    assert_eq!(
        replied(actuate(&mut b, 0, 77, 10)).kind,
        ResponseKind::Actuated
    );
    let old_session = b.host.link.session().unwrap();
    let recorded_command = b.line.recorded(Dir::ToModule)[1].clone();
    // Reboot; the scenario replays what it recorded (dropped: no session).
    b.host.idle(&mut b.line, FROM + 5 - b.host.now);
    assert_eq!(b.line.module.output, [0, 0]);
    // The host process restarted too: new link, sequence numbers from 1.
    let mut host = host_for(ModuleKind::InterlockController);
    host.now = b.host.now;
    assert!(matches!(host.open(&mut b.line, 20), Outcome::Replied(_)));
    assert_eq!(host.link.session().unwrap(), old_session, "ids repeated");
    b.line.inject_to_module(host.now, &recorded_command);
    assert_eq!(b.line.module.output[0], 77, "the replayed command ran");

    // Control: with a sound generator the same replay is refused.
    let s = scenario("replay_across_reboot");
    let mut b = with(ModuleKind::InterlockController, s, true);
    assert_eq!(
        replied(actuate(&mut b, 0, 77, 10)).kind,
        ResponseKind::Actuated
    );
    let old_session = b.host.link.session().unwrap();
    let recorded_command = b.line.recorded(Dir::ToModule)[1].clone();
    b.host.idle(&mut b.line, FROM + 5 - b.host.now);
    let mut host = host_for(ModuleKind::InterlockController);
    host.now = b.host.now;
    assert!(matches!(host.open(&mut b.line, 20), Outcome::Replied(_)));
    assert_ne!(host.link.session().unwrap(), old_session);
    b.line.inject_to_module(host.now, &recorded_command);
    assert_eq!(b.line.module.output, [0, 0]);
    assert!(
        b.line
            .module
            .dropped_for(|d| matches!(d, Dropped::WrongSession))
            >= 1
    );
}

/// An `Identify` anyone can send in session 0 opens a new session and ends
/// the host's. The module goes to its safe state and the host's next request
/// is dropped. That is a denial of service, never a hazard. #100 must notice
/// repeated session loss and back off or quarantine; #172 (signed commands)
/// is what would let the module refuse an `Identify` the host did not send.
#[test]
fn a_replayed_identify_ends_the_session_and_forces_the_safe_state() {
    let mut b = with(
        ModuleKind::InterlockController,
        scenario("identify_replay"),
        true,
    );
    assert_eq!(
        replied(actuate(&mut b, 0, 500, 10)).kind,
        ResponseKind::Actuated
    );
    assert_eq!(b.line.module.output[0], 500);
    let old_session = b.line.module.session();
    let entries = b.line.module.safe_entries;
    b.host.idle(&mut b.line, FROM + 3 - b.host.now);
    // The module took the replay: new session, safe state.
    assert_ne!(b.line.module.session(), old_session);
    assert!(b.line.module.session().is_some());
    assert_eq!(b.line.module.safe_entries, entries + 1);
    assert_eq!(b.line.module.output, [0, 0]);
    // Its answer to the attacker's Identify reached a host that had asked
    // for nothing.
    assert!(b.host.refused.contains(&HostError::Unsolicited));
    // The host's old session is dead: requests are dropped, never executed.
    assert_eq!(actuate(&mut b, 0, 600, 10), Outcome::Timeout);
    assert_eq!(b.line.module.output, [0, 0]);
    assert!(
        b.line
            .module
            .dropped_for(|d| matches!(d, Dropped::WrongSession))
            >= 1
    );
    assert!(!b.host.link.busy());
    // The host recovers by opening a session again, and verifies it afresh.
    assert!(matches!(b.host.open(&mut b.line, 20), Outcome::Replied(_)));
    assert!(b.host.trusted());
    assert_eq!(
        replied(actuate(&mut b, 0, 5, 10)).kind,
        ResponseKind::Actuated
    );
}

/// A module that signs an identity naming another module fails the host's
/// check, whichever module the host thinks it is talking to. Hub-port
/// binding and duplicate-id quarantine are #100.
#[test]
fn a_module_claiming_another_ones_id_is_never_trusted() {
    let mut b = with(ModuleKind::DcMeter, scenario("claim_other_id"), false);
    let out = b.host.open(&mut b.line, 20);
    assert!(
        matches!(
            out,
            Outcome::IdentityRejected(IdentityError::WrongModule { .. })
        ),
        "{out:?}"
    );
    assert!(!b.host.trusted());
    // Untrusted: the host sends nothing but another Identify.
    assert_eq!(health(&mut b, 10), Outcome::NotTrusted);
    assert_eq!(b.line.module.executed, 0);
    // Checked as the module it claims to be, with that module's key, the
    // signature is not that module's.
    let mut as_other = sawbones_sim_modules::HostHarness::new(
        OTHER,
        key_of(&OTHER_SEED),
        ModuleKind::DcMeter.capabilities(),
        1,
    );
    let out = as_other.open(&mut b.line, 20);
    assert!(
        matches!(out, Outcome::IdentityRejected(IdentityError::BadSignature)),
        "{out:?}"
    );
    assert!(!as_other.trusted());
}

#[test]
fn a_can_emit_beyond_the_signed_identity_does_not_verify() {
    let mut b = with(ModuleKind::DcMeter, scenario("overclaim_can_emit"), false);
    let out = b.host.open(&mut b.line, 20);
    assert!(
        matches!(out, Outcome::IdentityRejected(IdentityError::BadSignature)),
        "{out:?}"
    );
    assert!(!b.host.trusted());
    // The honest version of the same lie: a module that really signs
    // can_emit, enrolled as one that cannot. The signature is good, so the
    // protocol layer alone accepts it; comparing with the enrolment is the
    // supervisor's job (#100), which the harness does here.
    let mut line = SimLink::new(module(ModuleKind::CommsPod), None);
    let mut host = host_for(ModuleKind::DcMeter);
    let out = host.open(&mut line, 20);
    assert!(
        matches!(out, Outcome::CapabilityMismatch { enrolled, signed }
            if !enrolled.can_emit && signed.can_emit),
        "{out:?}"
    );
    assert!(!host.trusted());
    assert_eq!(
        host.call(&mut line, &Request::Health, 10),
        Outcome::NotTrusted
    );
}

#[test]
fn an_oversized_length_field_is_refused_by_both_ends_and_the_stream_reader() {
    // To the host, with the check value fixed up as anyone on the line can.
    let mut b = windowed("oversized_length");
    for _ in 0..10 {
        assert_eq!(health(&mut b, 10), Outcome::Timeout);
    }
    assert!(b.host.refused.iter().all(|e| matches!(
        e,
        HostError::Malformed(DecodeError::Oversized { declared: 65535 })
    )));
    recovered(&mut b);

    // To the module, and for lengths just over the bound.
    for declared in [MAX_PAYLOAD_LEN as u16 + 1, 1000, u16::MAX] {
        for fix_crc in [false, true] {
            let s = Scenario::new(
                "oversized_length_to_module",
                SCENARIO_SEED,
                Dir::ToModule,
                FROM,
                Fault::Oversize { declared, fix_crc },
            );
            let mut b = armed_scenario(s);
            assert_eq!(actuate(&mut b, 0, 9, 10), Outcome::Timeout);
            assert_eq!(b.line.module.executed, 0);
            assert!(b.line.module.rejections.iter().all(|r| matches!(
                r,
                sawbones_module_protocol::Rejection::Drop(Dropped::Malformed(
                    DecodeError::Oversized { .. }
                ))
            )));
            assert_eq!(b.line.module.rejections.len(), 1);
        }
    }

    // A byte-stream reader that sizes its read from the prefix is told no
    // before it waits for or reserves 64 KiB.
    let mut prefix = *b"SB\x01\x81";
    let mut header = prefix.to_vec();
    header.extend_from_slice(&u16::MAX.to_le_bytes());
    assert_eq!(
        frame_len(&header),
        Err(DecodeError::Oversized { declared: 65535 })
    );
    prefix[3] = 0x01;
    assert!(frame_len(&prefix).is_err());
    assert!(decode_response(&header).is_err());
}

/// A stuck line: replies stop (or the line holds one level). The host times
/// out each call and is never left with a request outstanding. The module,
/// which cannot know the host is deaf, keeps its session until its own link
/// timeout, then goes to its safe state: that is the firmware's half of the
/// contract. The protocol has no clock, so declaring the module lost, and
/// driving the interlock from it, is #100.
#[test]
fn a_silent_line_times_out_every_call_and_the_module_goes_safe_on_its_own() {
    let mut b = armed_scenario(scenario("silent_module"));
    assert_eq!(actuate(&mut b, 0, 9, 10), Outcome::Timeout);
    assert_eq!(b.line.module.output[0], 9);
    for _ in 0..5 {
        assert_eq!(health(&mut b, 10), Outcome::Timeout);
    }
    assert_eq!(b.host.timeouts, 6);
    assert!(
        b.host.refused.is_empty(),
        "silence is not a malformed frame"
    );
    assert!(!b.host.link.busy());
    // The host stops asking; the module's link timeout does the rest.
    let entries = b.line.module.safe_entries;
    b.host.idle(&mut b.line, 1500);
    assert_eq!(b.line.module.output, [0, 0]);
    assert!(b.line.module.session().is_none());
    assert_eq!(b.line.module.safe_entries, entries + 1);
}

#[test]
fn a_line_stuck_at_one_level_is_refused_every_tick_and_never_a_frame() {
    let mut b = armed_scenario(scenario("stuck_line_high"));
    assert_eq!(health(&mut b, 20), Outcome::Timeout);
    assert!(b.host.refused.len() >= 20);
    assert!(b
        .host
        .refused
        .iter()
        .all(|e| matches!(e, HostError::Malformed(DecodeError::BadMagic))));
    assert!(!b.host.link.busy());
    // A stream reader resynchronises on the magic: 0xFF is not it.
    assert_eq!(frame_len(&[0xFF; 32]), Err(DecodeError::BadMagic));
    // And a module hearing a stuck line goes safe through its link timeout.
    let s = Scenario::new(
        "stuck_line_to_module",
        SCENARIO_SEED,
        Dir::ToModule,
        FROM,
        Fault::StuckLine { level: 0xFF },
    );
    let mut b = armed_scenario(s);
    assert_eq!(actuate(&mut b, 0, 9, 10), Outcome::Timeout);
    b.host.idle(&mut b.line, 1500);
    assert_eq!(b.line.module.output, [0, 0]);
    assert!(b.line.module.counters().bad_frames > 100);
    assert_eq!(b.line.module.executed, 0);
}
