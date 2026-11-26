//! #203: a write passes only as the frame of the approved action (bus-014),
//! for the right unit (bus-016), as the exact bytes checked (bus-009), and
//! not while another master is heard on the line (bus-020).

mod common;

use common::{action_for, grant_action, read, replace, rule, write_multiple, write_single, Wire};
use sawbones_busguard::{
    parse, ActionDigest, ActionDigestMismatch, BlockReason, BusGuard, BusWriteAction, FrameSink,
    Function, GuardedLine, Heard, LeaseGrant, Port, Rule, TransmitError, UpdateRefusal, Verdict,
    WriteFunction, ECHO_WINDOW_TICKS, MAX_ACTION_WORDS, MAX_UNIT_ADDRESS, MAX_WRITE_QUANTITY,
};

use BlockReason::*;

/// SplitMix64, so every failure is reproducible from its seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn random_action(rng: &mut Rng) -> BusWriteAction {
    let count = 1 + rng.below(MAX_ACTION_WORDS as u64) as usize;
    let function = if count == 1 && rng.below(2) == 0 {
        WriteFunction::WriteSingleRegister
    } else {
        WriteFunction::WriteMultipleRegisters
    };
    let values: Vec<u16> = (0..count).map(|_| rng.next() as u16).collect();
    let register = rng.next() as u16 % (0xFFFF - 8);
    let unit = 1 + rng.below(u64::from(MAX_UNIT_ADDRESS)) as u8;
    BusWriteAction::new(rng.below(4) as u8, unit, function, register, &values).unwrap()
}

/// Rules that let any write to `unit` on `port` through the allow-list, so a
/// block can only come from the lease.
fn wide_write_rules(port: u8, unit: u8) -> Vec<Rule> {
    vec![
        rule(port, unit, Function::WriteSingleRegister, 0, 0xFFFF, 1),
        rule(
            port,
            unit,
            Function::WriteMultipleRegisters,
            0,
            0xFFFF,
            MAX_WRITE_QUANTITY,
        ),
    ]
}

fn guard_with(rules: &[Rule]) -> BusGuard {
    let mut guard = BusGuard::new();
    guard.apply(replace(rules), 0).unwrap();
    guard
}

// --- bus-014: the frame is the approved action's frame ---------------------

/// For any approved action exactly one frame passes, its own, and every
/// one-byte change to it is blocked.
#[test]
fn regression_203_exactly_the_approved_actions_frame_passes() {
    let mut rng = Rng(0x203);
    for seed in 0..400 {
        let action = random_action(&mut rng);
        let port = Port(action.port());
        let frame = action.frame();
        let approved = frame.as_bytes();
        let rules = wide_write_rules(action.port(), action.unit());
        for at in 0..approved.len() {
            for delta in [1u8, 0x80, rng.next() as u8 | 1] {
                let mut guard = guard_with(&rules);
                guard.apply(grant_action(action), 1).unwrap();
                let mut changed = approved.to_vec();
                changed[at] ^= delta;
                let verdict = guard.decide(port, &changed, 2).verdict();
                assert_ne!(
                    verdict,
                    Verdict::Pass,
                    "seed {seed} byte {at}: {changed:02X?}"
                );
                // A change that is still a well-formed, allow-listed write is
                // stopped by the lease, which stays for the approved frame.
                if parse(&changed).is_ok_and(|f| f.unit() == action.unit()) {
                    assert_eq!(
                        verdict,
                        Verdict::Block(DigestMismatch),
                        "seed {seed} byte {at}"
                    );
                }
                assert!(guard.lease().is_some(), "seed {seed} byte {at}");
                assert_eq!(
                    guard.decide(port, approved, 3),
                    Verdict::Pass,
                    "seed {seed}"
                );
                assert_eq!(guard.decide(port, approved, 3), Verdict::Block(NoLease));
            }
        }
    }
}

/// A frame that differs in a whole field (unit, register, value, function)
/// is a different action, even with a valid CRC.
#[test]
fn regression_203_another_action_never_spends_the_lease() {
    let approved =
        BusWriteAction::new(0, 1, WriteFunction::WriteSingleRegister, 0x10, &[3]).unwrap();
    let others = [
        write_single(2, 0x10, 3),
        write_single(1, 0x11, 3),
        write_single(1, 0x10, 4),
        write_multiple(1, 0x10, &[3]),
        write_multiple(1, 0x10, &[3, 0]),
    ];
    let mut rules = wide_write_rules(0, 1);
    rules.extend(wide_write_rules(0, 2));
    for other in others {
        let mut guard = guard_with(&rules);
        guard.apply(grant_action(approved), 1).unwrap();
        assert_eq!(
            guard.decide(Port(0), &other, 2),
            Verdict::Block(DigestMismatch),
            "{other:02X?}"
        );
        assert_eq!(
            guard.decide(Port(0), approved.frame().as_bytes(), 2),
            Verdict::Pass
        );
    }
}

#[test]
fn regression_203_a_grant_whose_digest_is_not_its_actions_is_refused() {
    let action = BusWriteAction::new(0, 1, WriteFunction::WriteSingleRegister, 0x10, &[3]).unwrap();
    let other = BusWriteAction::new(0, 1, WriteFunction::WriteSingleRegister, 0x30, &[3]).unwrap();
    assert_eq!(
        LeaseGrant::new(action, other.digest()),
        Err(ActionDigestMismatch)
    );
    assert_eq!(
        LeaseGrant::new(action, ActionDigest([0; 32])),
        Err(ActionDigestMismatch)
    );
    let grant = LeaseGrant::new(action, action.digest()).unwrap();
    assert_eq!(grant, LeaseGrant::for_action(action));
    assert_eq!(grant.action(), &action);
}

/// The verified path: an approval whose digest is not its action's is
/// refused by the guard even though the trust root signed it.
#[cfg(feature = "approvals")]
#[test]
fn regression_203_a_signed_lease_with_a_mismatched_digest_is_refused() {
    use sawbones_approvals::test_keys::{sign, verifier, TRUST_ROOT_SEED};
    use sawbones_approvals::{Approval, DeviceId, Digest, GrantLease, Message, Role};
    use sawbones_busguard::TrustRootEvent;

    let action = BusWriteAction::new(0, 1, WriteFunction::WriteSingleRegister, 0x10, &[3]).unwrap();
    let mut verifier = verifier(DeviceId(3));
    let sign_lease = |counter, grant| {
        let bytes = Message {
            target: DeviceId(3),
            signer: Role::TrustRoot,
            counter,
            expires_at: 1_000,
            approval: Approval::GrantLease(grant),
        }
        .encode();
        let signature = sign(&TRUST_ROOT_SEED, &bytes);
        (bytes, signature)
    };
    let (bytes, signature) = sign_lease(
        1,
        GrantLease {
            action,
            action_digest: Digest([7; 32]),
        },
    );
    let approved = verifier
        .verify::<GrantLease>(&bytes, &signature, 0)
        .unwrap();
    assert_eq!(
        TrustRootEvent::from_approved_lease(approved),
        Err(ActionDigestMismatch)
    );

    let (bytes, signature) = sign_lease(2, GrantLease::new(action));
    let approved = verifier
        .verify::<GrantLease>(&bytes, &signature, 0)
        .unwrap();
    let mut guard = guard_with(&wide_write_rules(0, 1));
    guard
        .apply(TrustRootEvent::from_approved_lease(approved).unwrap(), 1)
        .unwrap();
    assert_eq!(
        guard.decide(Port(0), action.frame().as_bytes(), 2),
        Verdict::Pass
    );
}

// --- bus-016: the unit address ---------------------------------------------

/// Two paralleled inverters, units 1 and 2, on one bus with the same
/// register map. A lease for one never passes the other's write.
#[test]
fn regression_203_a_lease_for_one_unit_never_passes_another() {
    let mut rules = wide_write_rules(0, 1);
    rules.extend(wide_write_rules(0, 2));
    for (leased, other) in [(1, 2), (2, 1)] {
        let action = BusWriteAction::new(
            0,
            leased,
            WriteFunction::WriteSingleRegister,
            0x2000,
            &[480],
        )
        .unwrap();
        let mut guard = guard_with(&rules);
        guard.apply(grant_action(action), 1).unwrap();
        assert_eq!(
            guard.decide(Port(0), &write_single(other, 0x2000, 480), 2),
            Verdict::Block(DigestMismatch)
        );
        assert_eq!(
            guard.decide(Port(0), &write_single(leased, 0x2000, 480), 2),
            Verdict::Pass
        );
    }
    // And the two actions have different digests.
    let one = action_for(0, &write_single(1, 0x2000, 480)).unwrap();
    let two = action_for(0, &write_single(2, 0x2000, 480)).unwrap();
    assert_ne!(one.digest(), two.digest());
}

// --- bus-009: the bytes checked are the bytes sent -------------------------

#[test]
fn regression_203_the_pass_owns_the_checked_bytes() {
    let action = BusWriteAction::new(0, 1, WriteFunction::WriteSingleRegister, 0x10, &[3]).unwrap();
    let mut guard = guard_with(&wide_write_rules(0, 1));
    guard.apply(grant_action(action), 1).unwrap();

    // The firmware's shared buffer, which a DMA engine or another core may
    // still write after the decision.
    let mut buffer = action.frame().as_bytes().to_vec();
    let decided = buffer.clone();
    let pass = guard.decide(Port(0), &buffer, 2).into_pass().unwrap();
    // The buffer is rewritten between the decision and the transmit call:
    // a write to the battery limit, with a valid CRC.
    buffer.copy_from_slice(&write_single(1, 0x30, 0xFFFF));
    assert_ne!(buffer, decided);

    let wire = Wire::default();
    let mut line = GuardedLine::new(Port(0), wire.clone());
    assert_eq!(pass.as_bytes(), decided);
    line.transmit(pass).unwrap();
    assert_eq!(wire.frames(), vec![decided]);
}

#[test]
fn regression_203_a_pass_goes_out_only_on_its_own_port() {
    let read_rule = rule(1, 1, Function::ReadHoldingRegisters, 0, 9, 10);
    let mut guard = guard_with(&[read_rule]);
    let frame = read(1, 0x03, 0, 1);
    let pass = guard.decide(Port(1), &frame, 1).into_pass().unwrap();
    assert_eq!(pass.port(), Port(1));
    let wire = Wire::default();
    let mut other = GuardedLine::new(Port(0), wire.clone());
    assert_eq!(other.transmit(pass), Err(TransmitError::OtherPort));
    assert!(wire.frames().is_empty());

    let pass = guard.decide(Port(1), &frame, 1).into_pass().unwrap();
    let mut line = GuardedLine::new(Port(1), wire.clone());
    assert_eq!(line.port(), Port(1));
    line.transmit(pass).unwrap();
    assert_eq!(wire.frames(), vec![frame]);
}

#[test]
fn regression_203_a_blocked_frame_gives_no_pass() {
    let mut guard = guard_with(&wide_write_rules(0, 1));
    let decision = guard.decide(Port(0), &write_single(1, 0x10, 3), 1);
    assert!(!decision.is_pass());
    assert_eq!(decision.verdict(), Verdict::Block(NoLease));
    assert!(decision.into_pass().is_none());
    // Too long to copy: blocked before anything is judged.
    assert_eq!(
        guard.decide(Port(0), &[0; 257], 1),
        Verdict::Block(Frame(sawbones_busguard::FrameError::TooLong))
    );
}

/// A sink that fails, so the error reaches the caller.
struct Broken;

impl FrameSink for Broken {
    type Error = &'static str;
    fn send_frame(&mut self, _: &[u8]) -> Result<(), &'static str> {
        Err("line fault")
    }
}

#[test]
fn regression_203_a_transmitter_fault_is_reported() {
    let mut guard = guard_with(&[rule(0, 1, Function::ReadHoldingRegisters, 0, 9, 10)]);
    let pass = guard
        .decide(Port(0), &read(1, 0x03, 0, 1), 1)
        .into_pass()
        .unwrap();
    let mut line = GuardedLine::new(Port(0), Broken);
    assert_eq!(line.transmit(pass), Err(TransmitError::Sink("line fault")));
}

// --- bus-020: another master on the line -----------------------------------

fn bench() -> (BusGuard, BusWriteAction, Vec<u8>) {
    let action = BusWriteAction::new(0, 1, WriteFunction::WriteSingleRegister, 0x10, &[3]).unwrap();
    let mut rules = wide_write_rules(0, 1);
    rules.push(rule(0, 1, Function::ReadHoldingRegisters, 0, 0x3F, 32));
    let guard = guard_with(&rules);
    (guard, action, read(1, 0x03, 0, 2))
}

#[test]
fn regression_203_a_foreign_request_blocks_writes_until_a_new_allow_list() {
    let (mut guard, action, poll) = bench();
    guard.apply(grant_action(action), 1).unwrap();
    // Another master polls the inverter.
    assert_eq!(
        guard.observe_line(Port(0), &read(1, 0x03, 0x20, 4), 2),
        Heard::ForeignRequest
    );
    assert_eq!(guard.foreign_requests_seen(), 1);
    // The lease has ended and no write passes, not even the approved one.
    assert_eq!(guard.lease(), None);
    assert_eq!(
        guard.decide(Port(0), action.frame().as_bytes(), 3),
        Verdict::Block(OtherMasterSeen)
    );
    assert_eq!(
        guard.apply(grant_action(action), 4),
        Err(UpdateRefusal::OtherMasterSeen)
    );
    // Reads still follow their rules.
    assert_eq!(guard.decide(Port(0), &poll, 5), Verdict::Pass);
    // A restart does not clear it.
    guard.restart();
    assert_eq!(guard.foreign_requests_seen(), 1);
    assert_eq!(
        guard.apply(grant_action(action), 6),
        Err(UpdateRefusal::OtherMasterSeen)
    );
    // A new allow-list from the trust root does.
    let (fresh, _, _) = bench();
    guard
        .apply(
            replace(&fresh.allow_list().rules().copied().collect::<Vec<_>>()),
            7,
        )
        .unwrap();
    assert_eq!(guard.foreign_requests_seen(), 0);
    guard.apply(grant_action(action), 8).unwrap();
    assert_eq!(
        guard.decide(Port(0), action.frame().as_bytes(), 9),
        Verdict::Pass
    );
}

#[test]
fn regression_203_the_guards_own_echo_is_not_another_master() {
    let (mut guard, action, poll) = bench();
    guard.apply(grant_action(action), 1).unwrap();
    // A half-duplex receiver hears the guard's own frames.
    assert!(guard.decide(Port(0), &poll, 2).is_pass());
    assert_eq!(guard.observe_line(Port(0), &poll, 2), Heard::OwnEcho);
    assert!(guard
        .decide(Port(0), action.frame().as_bytes(), 3)
        .is_pass());
    assert_eq!(
        guard.observe_line(Port(0), action.frame().as_bytes(), 3),
        Heard::OwnEcho
    );
    // The slave's reply to a write single register is the same bytes.
    assert_eq!(
        guard.observe_line(Port(0), action.frame().as_bytes(), 4),
        Heard::OwnEcho
    );
    // A read response is not a request: unit, 0x03, byte count, data, CRC.
    let mut response = vec![1, 0x03, 4, 0, 1, 0, 2];
    let crc = sawbones_busguard::crc16_modbus(&response).to_le_bytes();
    response.extend_from_slice(&crc);
    assert_eq!(
        guard.observe_line(Port(0), &response, 4),
        Heard::NotARequest
    );
    // Nor is noise, nor an exception response.
    assert_eq!(
        guard.observe_line(Port(0), &[0xFF; 5], 4),
        Heard::NotARequest
    );
    assert_eq!(guard.foreign_requests_seen(), 0);

    // The same frame heard on another port, or long after, is someone else.
    assert_eq!(guard.observe_line(Port(1), &poll, 5), Heard::ForeignRequest);
    let (mut guard, _, poll) = bench();
    assert!(guard.decide(Port(0), &poll, 10).is_pass());
    assert_eq!(
        guard.observe_line(Port(0), &poll, 10 + ECHO_WINDOW_TICKS + 1),
        Heard::ForeignRequest
    );
    // A broadcast from another master counts.
    let (mut guard, _, _) = bench();
    assert_eq!(
        guard.observe_line(Port(0), &write_single(0, 0x10, 1), 1),
        Heard::ForeignRequest
    );
}

/// A capture holding only the guard's own transmissions, heard back within
/// the window, never counts as another master; one foreign request among
/// them always does.
#[test]
fn regression_203_random_captures_of_own_traffic_are_never_foreign() {
    let mut rng = Rng(0x2020);
    for seed in 0..200 {
        let (mut guard, _, _) = bench();
        let mut now = 1;
        let mut own = Vec::new();
        for _ in 0..50 {
            let frame = read(1, 0x03, rng.below(0x20) as u16, 1 + rng.below(8) as u16);
            assert!(guard.decide(Port(0), &frame, now).is_pass());
            own.push((frame, now));
            now += rng.below(3);
            // Hear back one of the last few frames sent: still in the
            // echo window and still remembered.
            let recent = own.len().min(sawbones_busguard::ECHO_SLOTS);
            let (heard, at) = &own[own.len() - 1 - rng.below(recent as u64) as usize];
            assert!(now - at <= ECHO_WINDOW_TICKS, "seed {seed}");
            assert_eq!(
                guard.observe_line(Port(0), heard, now),
                Heard::OwnEcho,
                "seed {seed}"
            );
            assert_eq!(guard.foreign_requests_seen(), 0, "seed {seed}");
        }
        let foreign = write_single(1, 0x10 + rng.below(4) as u16, rng.next() as u16);
        assert_eq!(
            guard.observe_line(Port(0), &foreign, now),
            Heard::ForeignRequest,
            "seed {seed}"
        );
        assert!(guard.foreign_requests_seen() >= 1);
    }
}
