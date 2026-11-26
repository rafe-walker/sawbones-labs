//! Sessions, sequence numbers and authority, on both ends of the link, and a
//! simulated module and a host talking through it.

mod common;

use core::num::NonZeroU64;

use common::*;
use sawbones_module_protocol::*;

const CHALLENGE: Challenge = Challenge(*b"fresh-challenge!");

/// A host and a simulated module with a session open and the identity
/// verified.
struct Pair {
    host: HostLink,
    module: SimModule,
    out: [u8; MAX_FRAME_LEN],
    reply: [u8; MAX_FRAME_LEN],
}

impl Pair {
    fn open() -> Self {
        let mut pair = Pair {
            host: HostLink::new(),
            module: SimModule::new(MODULE_SEED, MODULE),
            out: [0; MAX_FRAME_LEN],
            reply: [0; MAX_FRAME_LEN],
        };
        let n = pair.host.open(CHALLENGE, &mut pair.out).unwrap();
        let m = pair.module.handle(&pair.out[..n], &mut pair.reply).unwrap();
        let claim = match pair.host.receive(&pair.reply[..m]).unwrap().into_claim() {
            Response::Identity(claim) => claim,
            other => panic!("{other:?}"),
        };
        let session = pair.host.session().unwrap();
        let verified = claim
            .verify(MODULE, &key(&MODULE_SEED), &CHALLENGE, session)
            .unwrap();
        assert_eq!(verified.module(), MODULE);
        assert_eq!(verified.manifest_digest(), manifest_digest());
        pair
    }

    /// Sends a request and returns the raw request and reply bytes.
    fn exchange(&mut self, request: &Request<'_>) -> (Vec<u8>, Option<Vec<u8>>) {
        let n = self.host.send(request, &mut self.out).unwrap();
        let sent = self.out[..n].to_vec();
        let reply = self
            .module
            .handle(&sent, &mut self.reply)
            .map(|m| self.reply[..m].to_vec());
        (sent, reply)
    }

    fn call(&mut self, request: &Request<'_>) -> Response<'static> {
        let (_, reply) = self.exchange(request);
        let reply: &'static [u8] = Box::leak(reply.expect("a reply").into_boxed_slice());
        self.host.receive(reply).unwrap().into_claim()
    }
}

#[test]
fn a_simulated_module_and_the_host_talk_through_every_call() {
    let mut p = Pair::open();
    let mut hasher = ManifestHasher::new();
    let mut page = 0;
    loop {
        match p.call(&Request::Describe { page }) {
            Response::Description(d) => {
                hasher.page(d.text);
                page += 1;
                if page == d.page_count {
                    break;
                }
            }
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(hasher.finish(), manifest_digest());
    assert!(
        matches!(p.call(&Request::Health), Response::Health(h) if h.status == HealthStatus::Ok)
    );
    assert!(matches!(p.call(&Request::SelfTest), Response::SelfTest(_)));
    assert!(matches!(
        p.call(&Request::Time { host_time_us: 77 }),
        Response::Time(t) if t.host_time_us == 77
    ));
    let w = ConfigWrite::new(MODULE, ConfigKey(1), 12, Grant::None);
    assert_eq!(
        p.call(&Request::ConfigWrite(w)),
        Response::ConfigValue {
            key: ConfigKey(1),
            value: 12
        }
    );
    assert_eq!(
        p.call(&Request::ConfigRead { key: ConfigKey(1) }),
        Response::ConfigValue {
            key: ConfigKey(1),
            value: 12
        }
    );
    match p.call(&Request::LogRead { from: 4, max: 2 }) {
        Response::Log(log) => assert_eq!(log.iter().map(|e| e.index).collect::<Vec<_>>(), [4, 5]),
        other => panic!("{other:?}"),
    }
    let chunk = Request::StageUpdate(UpdateChunk {
        image_digest: ImageDigest([1; 32]),
        image_len: 4,
        offset: 0,
        bytes: &[1, 2, 3, 4],
    });
    assert_eq!(
        p.call(&chunk),
        Response::UpdateStaged {
            received: 4,
            complete: true
        }
    );
    let a = Actuate::new(MODULE, Channel(0), 500, Grant::RailA);
    assert_eq!(
        p.call(&Request::Actuate(a)),
        Response::Actuated {
            channel: Channel(0),
            setting: 500
        }
    );
    let a = Actuate::new(MODULE, Channel(1), 9, Grant::Lease { port: Port(2) });
    let lease = Request::Actuate(a).lease_target().unwrap();
    assert_eq!(lease.port, Port(2));
    assert_eq!(
        p.call(&Request::Actuate(a)),
        Response::Actuated {
            channel: Channel(1),
            setting: 9
        }
    );
    assert_eq!(p.call(&Request::SafeState), Response::SafeStateEntered);
    assert_eq!(p.module.output, [0, 0]);
    assert_eq!(
        p.call(&Request::Reset(ResetMode::Restart)),
        Response::ResetAccepted
    );
}

#[test]
fn a_duplicate_or_replayed_request_is_dropped() {
    let mut p = Pair::open();
    let a = Actuate::new(MODULE, Channel(0), 1, Grant::RailA);
    let (sent, reply) = p.exchange(&Request::Actuate(a));
    p.host.receive(&reply.unwrap()).unwrap();
    // The same frame again, and an older one, are replays.
    let mut sink = [0u8; MAX_FRAME_LEN];
    assert_eq!(p.module.handle(&sent, &mut sink), None);
    match p.module.link.accept(&sent, || 1, &Table) {
        Err(Rejection::Drop(Dropped::Replayed { seq, last })) => assert_eq!(seq, last),
        other => panic!("{other:?}"),
    }
    assert!(p.module.link.counters().dropped >= 2);
}

#[test]
fn a_frame_from_an_old_session_is_dropped_after_a_new_one_opens() {
    let mut p = Pair::open();
    let a = Actuate::new(MODULE, Channel(0), 1, Grant::RailA);
    let (old, _) = p.exchange(&Request::Actuate(a));
    p.host.abandon();
    // The host opens a new session; the module goes safe.
    let before = p.module.safe_state_entries;
    let n = p.host.open(Challenge([9; 16]), &mut p.out).unwrap();
    let open = p.out[..n].to_vec();
    let m = p.module.handle(&open, &mut p.reply).unwrap();
    p.host.receive(&p.reply[..m]).unwrap();
    assert_eq!(p.module.safe_state_entries, before + 1);
    // The old frame names the old session.
    assert!(matches!(
        p.module.link.accept(&old, || 1, &Table),
        Err(Rejection::Drop(Dropped::WrongSession))
    ));
}

#[test]
fn nothing_but_identify_is_accepted_without_a_session_and_identify_needs_session_0() {
    let mut link = ModuleLink::new(MODULE);
    let health = request_bytes(RequestHeader { session: 0, seq: 1 }, Request::Health);
    assert_eq!(
        link.accept(&health, || 1, &Table),
        Err(Rejection::Drop(Dropped::NoSession))
    );
    let identify = request_bytes(
        RequestHeader { session: 3, seq: 1 },
        Request::Identify {
            challenge: CHALLENGE,
        },
    );
    assert_eq!(
        link.accept(&identify, || 1, &Table),
        Err(Rejection::Drop(Dropped::WrongSession))
    );
    let identify = request_bytes(
        RequestHeader { session: 0, seq: 1 },
        Request::Identify {
            challenge: CHALLENGE,
        },
    );
    assert_eq!(
        link.accept(&identify, || 0, &Table),
        Err(Rejection::Drop(Dropped::ZeroNonce))
    );
    assert_eq!(link.session(), None);
    let accepted = link.accept(&identify, || 42, &Table).unwrap();
    assert!(accepted.new_session);
    assert_eq!(link.session(), NonZeroU64::new(42));
    let mut out = [0u8; MAX_FRAME_LEN];
    link.end_session();
    assert!(link
        .reply(1, &Response::SafeStateEntered, &mut out)
        .is_err());
}

fn module_with_session() -> (ModuleLink, u64) {
    let mut link = ModuleLink::new(MODULE);
    let identify = request_bytes(
        RequestHeader { session: 0, seq: 1 },
        Request::Identify {
            challenge: CHALLENGE,
        },
    );
    link.accept(&identify, || 77, &Table).unwrap();
    (link, 77)
}

fn judge(
    link: &mut ModuleLink,
    session: u64,
    seq: u32,
    request: Request<'_>,
) -> Result<HazardClass, RefusalCode> {
    let bytes = request_bytes(RequestHeader { session, seq }, request);
    match link.accept(&bytes, || 1, &Table) {
        Ok(a) => Ok(a.class),
        Err(Rejection::Refuse { reply_to, code }) => {
            assert_eq!(reply_to, seq);
            Err(code)
        }
        Err(other) => panic!("{other:?}"),
    }
}

#[test]
fn the_module_table_decides_the_class_and_the_authority_must_reach_it() {
    let (mut link, s) = module_with_session();
    let lease = Grant::Lease { port: Port(0) };
    let cases = [
        // key 1 is class 1, key 2 class 2, key 3 class 3, key 9 class 4.
        (ConfigKey(1), Grant::None, Ok(HazardClass::Class1)),
        (ConfigKey(2), Grant::None, Err(RefusalCode::AuthorityTooLow)),
        (ConfigKey(2), Grant::RailA, Ok(HazardClass::Class2)),
        (
            ConfigKey(3),
            Grant::RailA,
            Err(RefusalCode::AuthorityTooLow),
        ),
        (ConfigKey(3), lease, Ok(HazardClass::Class3)),
        (ConfigKey(9), lease, Err(RefusalCode::ClassNeverPermitted)),
        (ConfigKey(5), lease, Err(RefusalCode::UnknownKey)),
    ];
    for (seq, (key, grant, want)) in (10..).zip(cases) {
        let w = ConfigWrite::new(MODULE, key, 1, grant);
        assert_eq!(
            judge(&mut link, s, seq, Request::ConfigWrite(w)),
            want,
            "{key:?} {grant:?}"
        );
    }
    let a = Actuate::new(MODULE, Channel(1), 1, Grant::RailA);
    assert_eq!(
        judge(&mut link, s, 30, Request::Actuate(a)),
        Err(RefusalCode::AuthorityTooLow)
    );
    let a = Actuate::new(MODULE, Channel(1), 1, lease);
    assert_eq!(
        judge(&mut link, s, 31, Request::Actuate(a)),
        Ok(HazardClass::Class3)
    );
    assert_eq!(
        judge(&mut link, s, 32, Request::SafeState),
        Ok(HazardClass::Class0)
    );
    assert!(link.counters().refused >= 4);
}

#[test]
fn a_lease_for_another_action_or_module_is_refused() {
    let (mut link, s) = module_with_session();
    let lease = Grant::Lease { port: Port(0) };
    // A lease made for value 1, sent with value 2.
    let mut w = ConfigWrite::new(MODULE, ConfigKey(3), 1, lease);
    w.value = 2;
    assert_eq!(
        judge(&mut link, s, 10, Request::ConfigWrite(w)),
        Err(RefusalCode::LeaseNotForThisAction)
    );
    // A lease made for another module.
    let w = ConfigWrite::new(OTHER_MODULE, ConfigKey(3), 1, lease);
    assert_eq!(
        judge(&mut link, s, 11, Request::ConfigWrite(w)),
        Err(RefusalCode::LeaseNotForThisAction)
    );
    // A lease for an actuation on the same channel number, spent on a write.
    let a = Actuate::new(MODULE, Channel(3), 1, lease);
    let w = ConfigWrite {
        key: ConfigKey(3),
        value: 1,
        authority: a.authority,
    };
    assert_eq!(
        judge(&mut link, s, 12, Request::ConfigWrite(w)),
        Err(RefusalCode::LeaseNotForThisAction)
    );
    // A refused request spends its sequence number.
    let w = ConfigWrite::new(MODULE, ConfigKey(3), 1, lease);
    let bytes = request_bytes(
        RequestHeader {
            session: s,
            seq: 12,
        },
        Request::ConfigWrite(w),
    );
    assert!(matches!(
        link.accept(&bytes, || 1, &Table),
        Err(Rejection::Drop(Dropped::Replayed { .. }))
    ));
}

#[test]
fn the_host_refuses_replies_it_did_not_ask_for() {
    let mut p = Pair::open();
    // Nothing outstanding.
    let (_, reply) = p.exchange(&Request::Health);
    let reply = reply.unwrap();
    p.host.receive(&reply).unwrap();
    assert_eq!(p.host.receive(&reply), Err(HostError::Unsolicited));

    // A reply to an earlier request while a new one is outstanding.
    p.host.send(&Request::Health, &mut p.out).unwrap();
    assert!(matches!(
        p.host.receive(&reply),
        Err(HostError::WrongReply { .. })
    ));
    assert_eq!(
        p.host.send(&Request::Health, &mut p.out),
        Err(HostError::Busy)
    );
    p.host.abandon();
    assert!(!p.host.busy());
    assert_eq!(
        p.host.send(
            &Request::Identify {
                challenge: CHALLENGE
            },
            &mut p.out
        ),
        Err(HostError::UseOpen)
    );
}

#[test]
fn the_host_refuses_the_wrong_kind_session_or_a_replayed_module_seq() {
    let mut p = Pair::open();
    let session = p.host.session().unwrap().get();
    let n = p.host.send(&Request::Health, &mut p.out).unwrap();
    let seq = decode_request(&p.out[..n]).unwrap().header.seq;
    let reply = |session, module_seq, response| {
        response_bytes(
            ResponseHeader {
                session,
                seq: module_seq,
                reply_to: seq,
            },
            response,
        )
    };
    assert_eq!(
        p.host
            .receive(&reply(session, 50, Response::SafeStateEntered)),
        Err(HostError::WrongKind {
            expected: ResponseKind::Health,
            received: ResponseKind::SafeStateEntered
        })
    );
    let health = Response::Health(HealthReport {
        status: HealthStatus::Ok,
        uptime_ticks: 0,
        fault_code: 0,
        bad_frames: 0,
        refused_frames: 0,
    });
    assert_eq!(
        p.host.receive(&reply(session + 1, 50, health)),
        Err(HostError::WrongSession)
    );
    // The identity was the module's seq 1 in this session.
    assert!(matches!(
        p.host.receive(&reply(session, 1, health)),
        Err(HostError::Replayed { .. })
    ));
    // A refusal answers any request.
    assert!(p
        .host
        .receive(&reply(session, 2, Response::Refused(RefusalCode::Busy)))
        .is_ok());
}

#[test]
fn a_host_link_without_a_session_sends_nothing() {
    let mut host = HostLink::new();
    let mut out = [0u8; MAX_FRAME_LEN];
    assert_eq!(
        host.send(&Request::Health, &mut out),
        Err(HostError::NotOpen)
    );
    host.open(CHALLENGE, &mut out).unwrap();
    assert_eq!(
        host.send(&Request::Health, &mut out),
        Err(HostError::NotOpen)
    );
    // An identity from session 0 never opens one.
    let seq = decode_request(&out[..MIN_FRAME_LEN + 16])
        .unwrap()
        .header
        .seq;
    let claim = signed_identity(
        &MODULE_SEED,
        MODULE,
        CAPS,
        &CHALLENGE,
        NonZeroU64::new(1).unwrap(),
    );
    let zero = response_bytes(
        ResponseHeader {
            session: 0,
            seq: 1,
            reply_to: seq,
        },
        Response::Identity(claim),
    );
    assert_eq!(host.receive(&zero), Err(HostError::ZeroSession));
    assert_eq!(host.session(), None);
}
