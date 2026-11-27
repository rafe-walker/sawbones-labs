//! Read-only questions to verified modules (#103): queued, sent on the
//! module's own link, answered as the module's claim.

mod common;

use common::*;
use sawbones_interlock::HazardClass;
use sawbones_module_protocol::{ConfigKey, RefusalCode};
use sawbones_supervisor::{
    ModuleQuery, ModuleState, QueryOutcome, QueryRefusal, QueryReply, MAX_QUEUED_QUERIES,
};

fn ready() -> Bench {
    let mut b = Bench::new(HazardClass::Class2);
    b.run(3);
    assert_eq!(b.s.module_state(METER), Some(ModuleState::Available));
    b
}

fn answered(outcome: Option<QueryOutcome>) -> QueryReply {
    match outcome {
        Some(QueryOutcome::Answered(reply)) => reply.into_claim(),
        other => panic!("not answered: {other:?}"),
    }
}

#[test]
fn a_manifest_page_comes_back_as_the_modules_bytes() {
    let mut b = ready();
    let id = b.s.query(METER, ModuleQuery::Describe { page: 0 }).unwrap();
    assert_eq!(b.s.take_answer(id), None, "nothing before a step");
    b.run(2);
    let QueryReply::Page(page) = answered(b.s.take_answer(id)) else {
        panic!("not a page");
    };
    assert_eq!(page.page(), 0);
    assert_eq!(page.page_count(), 1);
    assert_eq!(page.as_untrusted_bytes(), MANIFEST);
    assert_eq!(b.s.take_answer(id), None, "an answer is taken once");
}

#[test]
fn a_reading_and_a_refusal_come_back_as_claims() {
    let mut b = ready();
    let reading =
        b.s.query(
            METER,
            ModuleQuery::Read {
                key: ConfigKey(0x100),
            },
        )
        .unwrap();
    let setting =
        b.s.query(METER, ModuleQuery::Read { key: ConfigKey(1) })
            .unwrap();
    b.run(4);
    assert_eq!(
        answered(b.s.take_answer(reading)),
        QueryReply::Value {
            key: ConfigKey(0x100),
            value: 42
        }
    );
    assert_eq!(
        answered(b.s.take_answer(setting)),
        QueryReply::Refused(RefusalCode::UnknownKey)
    );
    assert_eq!(b.s.module_state(METER), Some(ModuleState::Available));
}

#[test]
fn a_question_needs_an_enrolled_available_module() {
    let mut b = Bench::new(HazardClass::Class2);
    let q = ModuleQuery::Describe { page: 0 };
    assert_eq!(
        b.s.query(METER, q),
        Err(QueryRefusal::Unavailable(ModuleState::Absent))
    );
    assert_eq!(
        b.s.query(sawbones_module_protocol::ModuleId(99), q),
        Err(QueryRefusal::NotEnrolled)
    );
    b.run(3);
    for _ in 0..MAX_QUEUED_QUERIES {
        b.s.query(METER, q).unwrap();
    }
    assert_eq!(b.s.query(METER, q), Err(QueryRefusal::Busy));
}

#[test]
fn an_unanswered_question_times_out_and_the_module_is_lost() {
    let mut b = ready();
    b.hub.module(METER_PORT).responsive = false;
    let first = b.s.query(METER, ModuleQuery::Describe { page: 0 }).unwrap();
    let queued = b.s.query(METER, ModuleQuery::Describe { page: 0 }).unwrap();
    b.run(sawbones_supervisor::LINK_TIMEOUT_TICKS + 2);
    assert_eq!(b.s.take_answer(first), Some(QueryOutcome::TimedOut));
    assert_eq!(b.s.take_answer(queued), Some(QueryOutcome::Dropped));
    assert_eq!(b.s.module_state(METER), Some(ModuleState::Lost));
}

#[test]
fn an_unplugged_module_drops_its_outstanding_question() {
    let mut b = ready();
    b.hub.module(METER_PORT).responsive = false;
    let id = b.s.query(METER, ModuleQuery::Describe { page: 0 }).unwrap();
    b.run(1);
    b.hub.unplug(METER_PORT);
    b.run(1);
    assert_eq!(b.s.take_answer(id), Some(QueryOutcome::Dropped));
}

#[test]
fn health_polls_go_on_between_questions() {
    let mut b = ready();
    for _ in 0..5 {
        let id = b.s.query(METER, ModuleQuery::Describe { page: 0 }).unwrap();
        b.run(25);
        assert!(matches!(
            b.s.take_answer(id),
            Some(QueryOutcome::Answered(_))
        ));
    }
    assert_eq!(b.s.module_state(METER), Some(ModuleState::Available));
}

/// Review of #103: a queued question went out before a due health poll,
/// and its answer put the next poll off, so a steady stream of questions
/// starved the health polls. Now a due poll goes first, and only a health
/// reply schedules the next one.
#[test]
fn regression_103_a_steady_question_stream_does_not_starve_health_polls() {
    use sawbones_module_protocol::{decode_request, Request};
    let mut b = ready();
    let health_sent = |b: &Bench| {
        b.hub.sent[&METER_PORT]
            .iter()
            .filter(|f| matches!(decode_request(f), Ok(r) if r.request == Request::Health))
            .count()
    };
    let before = health_sent(&b);
    let ticks = 20 * sawbones_supervisor::POLL_PERIOD_TICKS;
    let mut answered = 0;
    let mut waiting = Vec::new();
    for _ in 0..ticks {
        // Keep the queue full the whole time.
        while let Ok(id) = b.s.query(
            METER,
            ModuleQuery::Read {
                key: ConfigKey(0x100),
            },
        ) {
            waiting.push(id);
        }
        b.run(1);
        waiting.retain(|id| match b.s.take_answer(*id) {
            Some(_) => {
                answered += 1;
                false
            }
            None => true,
        });
    }
    let polls = health_sent(&b) - before;
    assert!(answered > 20, "questions still flow: {answered}");
    assert!(
        polls >= 18,
        "health polls starved: {polls} in {ticks} ticks"
    );
    assert_eq!(b.s.module_state(METER), Some(ModuleState::Available));
}
