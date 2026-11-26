//! The guard's rules, one table row at a time, against real-looking frames.

mod common;

use common::{grant, read, replace, rule, with_crc, write_multiple, write_single};
use sawbones_busguard::{
    crc16_modbus, parse, AllowList, AllowListError, BlockReason, BusGuard, Decision, FrameDigest,
    FrameError, Function, Lease, Port, Rule, Tick, UpdateRefusal, LEASE_TTL_TICKS, MAX_RULES,
};

use BlockReason::*;
use Function::*;

const P0: Port = Port(0);
const P1: Port = Port(1);

/// A battery's BMS at unit 1 on port 0: status and limits readable, one
/// harmless setting (0x0010) and a small block (0x0020..=0x0023) writable.
/// The cell over-voltage limit sits at 0x0030: readable, never writable.
/// Unit 2 on port 1 is a meter that is only read. The harmless setting is
/// also writable through port 1, so a lease's port matters.
fn bench_rules() -> Vec<Rule> {
    vec![
        rule(0, 1, ReadHoldingRegisters, 0x0000, 0x003F, 32),
        rule(0, 1, ReadInputRegisters, 0x0100, 0x010F, 16),
        rule(0, 1, WriteSingleRegister, 0x0010, 0x0010, 1),
        rule(0, 1, WriteMultipleRegisters, 0x0020, 0x0023, 4),
        rule(1, 2, ReadHoldingRegisters, 0x0000, 0x0009, 10),
        rule(1, 1, WriteSingleRegister, 0x0010, 0x0010, 1),
    ]
}

const CELL_OV_LIMIT: u16 = 0x0030;
const SETTING: u16 = 0x0010;

/// A guard with the bench allow-list loaded at tick 0.
fn bench_guard() -> BusGuard {
    let mut guard = BusGuard::new();
    guard.apply(replace(&bench_rules()), 0).unwrap();
    guard
}

fn block(reason: BlockReason) -> Decision {
    Decision::Block(reason)
}

fn frame_error(error: FrameError) -> Decision {
    Decision::Block(Frame(error))
}

// --- CRC and parsing -------------------------------------------------------

#[test]
fn crc_matches_the_published_check_value() {
    assert_eq!(crc16_modbus(b"123456789"), 0x4B37);
}

#[test]
fn builders_match_frames_from_the_modbus_specification() {
    // Application protocol v1.1b3, section 6: read holding registers and write
    // single register examples addressed to unit 0x11.
    assert_eq!(
        read(0x11, 0x03, 0x006B, 3),
        [0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87]
    );
    assert_eq!(
        write_single(0x11, 0x0001, 0x0003),
        [0x11, 0x06, 0x00, 0x01, 0x00, 0x03, 0x9A, 0x9B]
    );
    // Widely quoted serial-line examples for unit 1.
    assert_eq!(
        read(0x01, 0x03, 0x0000, 10),
        [0x01, 0x03, 0x00, 0x00, 0x00, 0x0A, 0xC5, 0xCD]
    );
    assert_eq!(
        write_single(0x01, 0x0001, 0x0003),
        [0x01, 0x06, 0x00, 0x01, 0x00, 0x03, 0x98, 0x0B]
    );
    assert_eq!(
        write_multiple(0x01, 0x0001, &[0x000A, 0x0102]),
        [0x01, 0x10, 0x00, 0x01, 0x00, 0x02, 0x04, 0x00, 0x0A, 0x01, 0x02, 0x92, 0x30]
    );
}

#[test]
fn parse_reads_every_field() {
    let bytes = [0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87];
    let frame = parse(&bytes).unwrap();
    assert_eq!(frame.unit(), 0x11);
    assert_eq!(frame.function(), ReadHoldingRegisters);
    assert_eq!(
        (frame.start(), frame.quantity(), frame.end()),
        (0x6B, 3, 0x6D)
    );
    assert_eq!(frame.byte_count(), None);
    assert!(frame.payload().is_empty());
    assert_eq!(frame.crc(), 0x8776);

    let bytes = write_single(0x11, 0x0001, 0x0003);
    let frame = parse(&bytes).unwrap();
    assert_eq!(frame.function(), WriteSingleRegister);
    assert_eq!((frame.start(), frame.quantity()), (1, 1));
    assert_eq!(frame.payload(), [0x00, 0x03]);

    let bytes = write_multiple(0x01, 0x0001, &[0x000A, 0x0102]);
    let frame = parse(&bytes).unwrap();
    assert_eq!(frame.function(), WriteMultipleRegisters);
    assert_eq!((frame.start(), frame.quantity(), frame.end()), (1, 2, 2));
    assert_eq!(frame.byte_count(), Some(4));
    assert_eq!(frame.payload(), [0x00, 0x0A, 0x01, 0x02]);

    let bytes = read(0x01, 0x04, 0x0100, 125);
    assert_eq!(parse(&bytes).unwrap().function(), ReadInputRegisters);
}

#[test]
fn malformed_frames_are_refused_with_their_reason() {
    let good_read = read(1, 0x03, 0, 1);
    let mut bad_crc = good_read.clone();
    bad_crc[7] ^= 0x01;
    let mut trailing = good_read.clone();
    trailing.push(0x00);
    let multiple = write_multiple(1, 0x20, &[1, 2]);

    let cases: Vec<(&str, Vec<u8>, FrameError)> = vec![
        ("empty", vec![], FrameError::Truncated),
        ("three bytes", vec![1, 3, 0], FrameError::Truncated),
        (
            "read cut short",
            good_read[..7].to_vec(),
            FrameError::Truncated,
        ),
        (
            "multiple without byte count",
            multiple[..6].to_vec(),
            FrameError::Truncated,
        ),
        (
            "multiple cut short",
            multiple[..multiple.len() - 1].to_vec(),
            FrameError::Truncated,
        ),
        ("trailing byte", trailing, FrameError::LengthMismatch),
        ("257 bytes", vec![0x01; 257], FrameError::TooLong),
        ("bad CRC", bad_crc, FrameError::BadCrc),
        (
            "write single coil",
            with_crc(&[1, 0x05, 0, 0, 0xFF, 0]),
            FrameError::UnsupportedFunction(0x05),
        ),
        (
            "exception code",
            with_crc(&[1, 0x83, 0x02]),
            FrameError::UnsupportedFunction(0x83),
        ),
        (
            "device id",
            with_crc(&[1, 0x2B, 0x0E, 1, 0]),
            FrameError::UnsupportedFunction(0x2B),
        ),
        (
            "read zero",
            read(1, 0x03, 0, 0),
            FrameError::QuantityOutOfRange,
        ),
        (
            "read 126",
            read(1, 0x04, 0, 126),
            FrameError::QuantityOutOfRange,
        ),
        (
            "write 124",
            with_crc(&[&[1u8, 0x10, 0, 0, 0, 124, 246][..], &[0u8; 246][..]].concat()),
            FrameError::QuantityOutOfRange,
        ),
        (
            "byte count odd",
            with_crc(&[1, 0x10, 0, 0x20, 0, 2, 3, 0, 1, 0]),
            FrameError::ByteCountMismatch,
        ),
        (
            "read past 0xFFFF",
            read(1, 0x03, 0xFFFF, 2),
            FrameError::AddressOverflow,
        ),
        (
            "broadcast single write",
            write_single(0, SETTING, 1),
            FrameError::Broadcast,
        ),
        (
            "broadcast multiple write",
            write_multiple(0, 0x20, &[1]),
            FrameError::Broadcast,
        ),
        ("broadcast read", read(0, 0x03, 0, 1), FrameError::Broadcast),
        (
            "reserved unit",
            read(248, 0x03, 0, 1),
            FrameError::ReservedAddress(248),
        ),
    ];
    for (name, bytes, error) in cases {
        assert_eq!(parse(&bytes), Err(error), "{name}");
        // And the guard blocks it with that reason, lease or no lease.
        let mut guard = bench_guard();
        guard.apply(grant(0, &bytes), 1).unwrap();
        assert_eq!(guard.decide(P0, &bytes, 1), frame_error(error), "{name}");
    }
}

#[test]
fn the_highest_register_and_the_largest_frame_parse() {
    assert!(parse(&read(1, 0x03, 0xFFFF, 1)).is_ok());
    assert!(parse(&read(247, 0x03, 0xFF83, 125)).is_ok());
    let largest = write_multiple(1, 0, &[0xABCD; 123]);
    assert_eq!(largest.len(), 255);
    assert!(parse(&largest).is_ok());
}

// --- Reads -----------------------------------------------------------------

#[test]
fn an_allowed_read_passes_every_time() {
    let mut guard = bench_guard();
    for now in [1, 1, 2, 500, 10_000] {
        assert_eq!(
            guard.decide(P0, &read(1, 0x03, 0x0000, 32), now),
            Decision::Pass
        );
        assert_eq!(
            guard.decide(P0, &read(1, 0x04, 0x010F, 1), now),
            Decision::Pass
        );
        assert_eq!(
            guard.decide(P1, &read(2, 0x03, 0x0000, 10), now),
            Decision::Pass
        );
    }
}

#[test]
fn a_read_no_rule_permits_is_blocked() {
    let cases = [
        ("unit not listed", P0, read(3, 0x03, 0, 1)),
        ("input rule, holding read", P0, read(1, 0x03, 0x0100, 1)),
        ("holding rule, input read", P0, read(1, 0x04, 0x0000, 1)),
        ("starts below the range", P0, read(1, 0x04, 0x00FF, 2)),
        ("runs past the range", P0, read(1, 0x03, 0x003E, 3)),
        ("more than max quantity", P0, read(1, 0x03, 0x0000, 33)),
        ("wrong port", P1, read(1, 0x03, 0x0000, 1)),
        ("other unit's port", P0, read(2, 0x03, 0x0000, 1)),
    ];
    let mut guard = bench_guard();
    for (name, port, bytes) in cases {
        assert_eq!(
            guard.decide(port, &bytes, 1),
            block(NotAllowListed),
            "{name}"
        );
    }
}

#[test]
fn regression_169_reads_are_not_free() {
    // A read transmits too. With nothing allow-listed, even a well-formed read
    // of a status register must not reach the wire.
    let mut guard = BusGuard::new();
    assert_eq!(
        guard.decide(P0, &read(1, 0x03, 0, 1), 0),
        block(NotAllowListed)
    );
}

// --- Writes ----------------------------------------------------------------

#[test]
fn a_leased_write_passes_once_then_blocks() {
    for frame in [
        write_single(1, SETTING, 0x0003),
        write_multiple(1, 0x0020, &[1, 2, 3, 4]),
    ] {
        let mut guard = bench_guard();
        assert_eq!(guard.decide(P0, &frame, 1), block(NoLease));
        guard.apply(grant(0, &frame), 2).unwrap();
        assert_eq!(
            guard.lease(),
            Some(Lease {
                port: P0,
                digest: FrameDigest::of(&frame),
                expires_at: 2 + LEASE_TTL_TICKS,
            })
        );
        assert_eq!(guard.decide(P0, &frame, 3), Decision::Pass);
        assert_eq!(guard.lease(), None);
        assert_eq!(guard.decide(P0, &frame, 3), block(NoLease));
        assert_eq!(guard.decide(P0, &frame, 4), block(NoLease));
    }
}

#[test]
fn a_write_with_any_byte_changed_is_blocked() {
    let approved = write_multiple(1, 0x0020, &[0x0102, 0x0304]);
    for index in 0..approved.len() - 2 {
        let mut body = approved[..approved.len() - 2].to_vec();
        body[index] ^= 0x01;
        // The CRC is recomputed, so this is a well-formed frame that differs
        // from the approved one in one byte.
        let changed = with_crc(&body);
        let mut guard = bench_guard();
        guard.apply(grant(0, &approved), 1).unwrap();
        let decision = guard.decide(P0, &changed, 1);
        assert!(!decision.is_pass(), "byte {index}: {decision:?}");
        // The lease is still there for the approved frame.
        assert_eq!(
            guard.decide(P0, &approved, 1),
            Decision::Pass,
            "byte {index}"
        );
    }
    // A changed value byte in a frame every rule still permits is caught by
    // the digest alone.
    let mut guard = bench_guard();
    guard.apply(grant(0, &approved), 1).unwrap();
    let changed = write_multiple(1, 0x0020, &[0x0102, 0x0305]);
    assert_eq!(guard.decide(P0, &changed, 1), block(DigestMismatch));
}

#[test]
fn regression_169_same_function_other_register_blocks() {
    // The hazard behind the guard: one write function reaches a harmless
    // setting and a battery protection limit, and only the register differs.
    // An approval for the setting must not carry the same value to the limit.
    let mut guard = bench_guard();
    let approved = write_single(1, SETTING, 0x0E10);
    let limit = write_single(1, CELL_OV_LIMIT, 0x0E10);
    guard.apply(grant(0, &approved), 1).unwrap();
    assert_eq!(guard.decide(P0, &limit, 1), block(NotAllowListed));

    // Even under a rule wide enough to cover both, the lease tells them apart.
    let mut guard = BusGuard::new();
    guard
        .apply(replace(&[rule(0, 1, WriteSingleRegister, 0, 0x3F, 1)]), 0)
        .unwrap();
    guard.apply(grant(0, &approved), 1).unwrap();
    assert_eq!(guard.decide(P0, &limit, 1), block(DigestMismatch));
    assert_eq!(guard.decide(P0, &approved, 1), Decision::Pass);
}

#[test]
fn a_leased_write_no_rule_permits_is_blocked() {
    let mut guard = bench_guard();
    let frame = write_single(1, CELL_OV_LIMIT, 1);
    guard.apply(grant(0, &frame), 1).unwrap();
    assert_eq!(guard.decide(P0, &frame, 1), block(NotAllowListed));
    // The lease was not spent by the blocked frame.
    assert!(guard.lease().is_some());
}

#[test]
fn an_expired_lease_blocks() {
    let frame = write_single(1, SETTING, 7);
    let granted_at: Tick = 10;

    let mut guard = bench_guard();
    guard.apply(grant(0, &frame), granted_at).unwrap();
    let last_live = granted_at + LEASE_TTL_TICKS - 1;
    assert_eq!(guard.decide(P0, &frame, last_live), Decision::Pass);

    let mut guard = bench_guard();
    guard.apply(grant(0, &frame), granted_at).unwrap();
    let expiry = granted_at + LEASE_TTL_TICKS;
    assert_eq!(guard.decide(P0, &frame, expiry), block(LeaseExpired));
    assert_eq!(guard.lease(), None);
    assert_eq!(guard.decide(P0, &frame, expiry), block(NoLease));
}

#[test]
fn expiry_is_noticed_by_any_call() {
    let frame = write_single(1, SETTING, 7);
    let mut guard = bench_guard();
    guard.apply(grant(0, &frame), 0).unwrap();
    // A read after expiry ends the lease, and a later write finds none.
    assert_eq!(
        guard.decide(P0, &read(1, 0x03, 0, 1), LEASE_TTL_TICKS),
        Decision::Pass
    );
    assert_eq!(guard.lease(), None);
    assert_eq!(guard.decide(P0, &frame, LEASE_TTL_TICKS), block(NoLease));
}

#[test]
fn a_lease_near_the_end_of_time_still_expires() {
    let frame = write_single(1, SETTING, 7);
    let mut guard = bench_guard();
    guard.apply(grant(0, &frame), Tick::MAX - 1).unwrap();
    assert_eq!(guard.lease().unwrap().expires_at, Tick::MAX);
    assert_eq!(guard.decide(P0, &frame, Tick::MAX), block(LeaseExpired));
}

#[test]
fn a_lease_on_another_port_blocks_and_stays() {
    let frame = write_single(1, SETTING, 9);
    let mut guard = bench_guard();
    guard.apply(grant(0, &frame), 1).unwrap();
    // A rule permits this frame on port 1 too; the lease does not.
    assert_eq!(guard.decide(P1, &frame, 1), block(LeaseOnOtherPort));
    assert_eq!(guard.decide(P0, &frame, 1), Decision::Pass);

    let mut guard = bench_guard();
    guard.apply(grant(1, &frame), 1).unwrap();
    assert_eq!(guard.decide(P0, &frame, 1), block(LeaseOnOtherPort));
    assert_eq!(guard.decide(P1, &frame, 1), Decision::Pass);
}

#[test]
fn a_new_grant_replaces_the_old_lease() {
    let first = write_single(1, SETTING, 1);
    let second = write_single(1, SETTING, 2);
    let mut guard = bench_guard();
    guard.apply(grant(0, &first), 1).unwrap();
    guard.apply(grant(0, &second), 2).unwrap();
    assert_eq!(guard.decide(P0, &first, 2), block(DigestMismatch));
    assert_eq!(guard.decide(P0, &second, 2), Decision::Pass);
    assert_eq!(guard.decide(P0, &first, 2), block(NoLease));
}

#[test]
fn a_new_allow_list_ends_the_lease() {
    let frame = write_single(1, SETTING, 1);
    let mut guard = bench_guard();
    guard.apply(grant(0, &frame), 1).unwrap();
    guard.apply(replace(&bench_rules()), 2).unwrap();
    assert_eq!(guard.lease(), None);
    assert_eq!(guard.decide(P0, &frame, 2), block(NoLease));
}

// --- Time and restart ------------------------------------------------------

#[test]
fn regression_169_a_clock_that_goes_backwards_ends_the_lease() {
    // A lifetime counted on a clock that went backwards would outlive its
    // window; the lease must end instead.
    let frame = write_single(1, SETTING, 5);
    let mut guard = bench_guard();
    guard.apply(grant(0, &frame), 50).unwrap();
    assert_eq!(guard.decide(P0, &frame, 49), block(ClockWentBackwards));
    assert_eq!(guard.lease(), None);
    assert_eq!(guard.decide(P0, &frame, 50), block(NoLease));

    // A read is not time-bound and still passes on a backward step.
    let mut guard = bench_guard();
    guard.apply(grant(0, &frame), 50).unwrap();
    assert_eq!(guard.decide(P0, &read(1, 0x03, 0, 1), 10), Decision::Pass);
    assert_eq!(guard.lease(), None);
}

#[test]
fn a_grant_on_a_backward_clock_is_refused() {
    let frame = write_single(1, SETTING, 5);
    let mut guard = bench_guard();
    assert_eq!(guard.decide(P0, &read(1, 0x03, 0, 1), 100), Decision::Pass);
    assert_eq!(
        guard.apply(grant(0, &frame), 99),
        Err(UpdateRefusal::ClockWentBackwards)
    );
    assert_eq!(guard.lease(), None);
    // The backward tick is the new baseline, so the trust root can grant
    // again.
    guard.apply(grant(0, &frame), 99).unwrap();
    assert_eq!(guard.decide(P0, &frame, 99), Decision::Pass);
}

#[test]
fn a_restart_ends_the_lease_and_keeps_the_allow_list() {
    let frame = write_single(1, SETTING, 5);
    let mut guard = bench_guard();
    guard.apply(grant(0, &frame), 50).unwrap();
    let before = *guard.allow_list();
    guard.restart();
    assert_eq!(guard.lease(), None);
    assert_eq!(*guard.allow_list(), before);
    assert_eq!(guard.decide(P0, &frame, 0), block(NoLease));
    // The clock restarts with the guard: tick 0 after a restart is not a
    // backward step.
    assert_eq!(guard.decide(P0, &read(1, 0x03, 0, 1), 0), Decision::Pass);
    guard.apply(grant(0, &frame), 0).unwrap();
    assert_eq!(guard.decide(P0, &frame, 0), Decision::Pass);
}

// --- The allow-list --------------------------------------------------------

#[test]
fn the_allow_list_refuses_rules_that_permit_nothing() {
    let mut list = AllowList::new();
    for bad in [
        rule(0, 0, WriteSingleRegister, 0, 0, 1),
        rule(0, 248, ReadHoldingRegisters, 0, 0, 1),
        rule(0, 1, ReadHoldingRegisters, 5, 4, 1),
        rule(0, 1, ReadHoldingRegisters, 0, 4, 0),
    ] {
        assert_eq!(
            list.try_push(bad),
            Err(AllowListError::InvalidRule),
            "{bad:?}"
        );
    }
    assert!(list.is_empty());
}

#[test]
fn the_allow_list_has_a_fixed_capacity() {
    let mut list = AllowList::new();
    for unit in 1..=MAX_RULES {
        let unit = u8::try_from(unit).unwrap();
        list.try_push(rule(0, unit, ReadHoldingRegisters, 0, 0, 1))
            .unwrap();
    }
    assert_eq!(list.len(), MAX_RULES);
    assert_eq!(
        list.try_push(rule(0, 100, ReadHoldingRegisters, 0, 0, 1)),
        Err(AllowListError::Full)
    );
    let units: Vec<u8> = list.rules().map(|rule| rule.unit).collect();
    assert_eq!(units, (1..=32).collect::<Vec<u8>>());
}

#[test]
fn an_empty_guard_blocks_everything() {
    let mut guard = BusGuard::default();
    let frame = write_single(1, SETTING, 1);
    guard.apply(grant(0, &frame), 0).unwrap();
    assert_eq!(guard.decide(P0, &frame, 0), block(NotAllowListed));
    assert_eq!(
        guard.decide(P0, &read(1, 0x03, 0, 1), 0),
        block(NotAllowListed)
    );
}
