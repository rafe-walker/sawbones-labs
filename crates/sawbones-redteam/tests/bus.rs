//! Bus guard attacks (lane `bus`). Each test is one catalog card.

mod support;

use sawbones_busguard::{BlockReason, Decision, FrameError, Function, Port};
use support::*;

const P: Port = Port(0);

fn everything_allowed() -> Vec<sawbones_busguard::Rule> {
    vec![
        rule(1, Function::ReadHoldingRegisters, 0, 65535, 125),
        rule(1, Function::ReadInputRegisters, 0, 65535, 125),
        rule(1, Function::WriteSingleRegister, 0, 65535, 1),
        rule(1, Function::WriteMultipleRegisters, 0, 65535, 123),
    ]
}

#[test]
fn redteam_bus_001_broadcast_and_reserved_units_never_pass() {
    for unit in std::iter::once(0u8).chain(248..=255) {
        let frames = [
            read(unit, 0x03, 100, 1),
            read(unit, 0x04, 100, 1),
            write_single(unit, 100, 7),
            write_multiple(unit, 100, &[7, 8]),
        ];
        for frame in frames {
            let mut g = guard(&everything_allowed());
            lease(&mut g, &frame, 1);
            let before = g.lease();
            let want = if unit == 0 {
                FrameError::Broadcast
            } else {
                FrameError::ReservedAddress(unit)
            };
            assert_eq!(
                g.decide(P, &frame, 2),
                Decision::Block(BlockReason::Frame(want)),
                "unit {unit}"
            );
            assert_eq!(
                g.lease(),
                before,
                "a refused frame must not touch the lease"
            );
        }
    }
}

#[test]
fn redteam_bus_002_unsupported_function_codes_never_pass() {
    for code in 0..=255u8 {
        if matches!(code, 3 | 4 | 6 | 16) {
            continue;
        }
        for len in [8usize, 12] {
            let mut body = vec![1u8, code];
            body.resize(len - 2, 0x11);
            let frame = with_crc(&body);
            let mut g = guard(&everything_allowed());
            lease(&mut g, &frame, 1);
            assert_eq!(
                g.decide(P, &frame, 2),
                Decision::Block(BlockReason::Frame(FrameError::UnsupportedFunction(code))),
                "function {code:#04x}, length {len}"
            );
        }
    }
}

#[test]
fn redteam_bus_003_a_write_multiple_reaching_a_protected_register_is_blocked() {
    // Registers 100 and 101 are permitted; 102 is protected.
    let rules = [rule(1, Function::WriteMultipleRegisters, 100, 101, 2)];
    let reaches = [
        write_multiple(1, 100, &[1, 2, 3]),
        write_multiple(1, 101, &[1, 2]),
        write_multiple(1, 102, &[1]),
    ];
    for frame in &reaches {
        let mut g = guard(&rules);
        lease(&mut g, frame, 1);
        assert_eq!(
            g.decide(P, frame, 2),
            Decision::Block(BlockReason::NotAllowListed)
        );
    }
    // Degenerate counts and ranges: each is refused for its own reason, even
    // with a lease for the exact bytes.
    let zero = write_multiple(1, 100, &[]);
    // Quantity says two registers, the byte count says one and the frame is
    // as long as the byte count says.
    let mismatch = with_crc(&[1, 0x10, 0, 100, 0, 2, 2, 0, 1]);
    let overflow = write_multiple(1, 0xFFFF, &[1, 2]);
    let too_many = write_multiple(1, 0, &[0; 124]);
    let cases = [
        (zero, FrameError::QuantityOutOfRange),
        (mismatch, FrameError::ByteCountMismatch),
        (overflow, FrameError::AddressOverflow),
        (too_many, FrameError::TooLong),
    ];
    for (frame, want) in cases {
        let mut g = guard(&rules);
        lease(&mut g, &frame, 1);
        assert_eq!(
            g.decide(P, &frame, 2),
            Decision::Block(BlockReason::Frame(want))
        );
    }
    // Control: the frame that stays inside the rule passes once, with its lease.
    let ok = write_multiple(1, 100, &[1, 2]);
    let mut g = guard(&rules);
    lease(&mut g, &ok, 1);
    assert_eq!(g.decide(P, &ok, 2), Decision::Pass);
}

#[test]
fn redteam_bus_004_concatenated_trailing_and_split_frames_never_pass() {
    let rules = everything_allowed();
    let w = write_single(1, 100, 7);
    let r = read(1, 0x03, 100, 1);

    let mut g = guard(&rules);
    lease(&mut g, &w, 1);
    let mut joined = r.clone();
    joined.extend_from_slice(&w);
    assert_eq!(
        g.decide(P, &joined, 2),
        Decision::Block(BlockReason::Frame(FrameError::LengthMismatch))
    );
    let mut trailing = w.clone();
    trailing.push(0);
    assert_eq!(
        g.decide(P, &trailing, 2),
        Decision::Block(BlockReason::Frame(FrameError::LengthMismatch))
    );
    for split in 1..w.len() {
        assert!(!g.decide(P, &w[..split], 2).is_pass(), "first part {split}");
        assert!(
            !g.decide(P, &w[split..], 2).is_pass(),
            "second part {split}"
        );
    }
    assert!(g.lease().is_some(), "the lease survives every refusal");
    assert_eq!(g.decide(P, &w, 2), Decision::Pass);
    assert_eq!(g.decide(P, &w, 2), Decision::Block(BlockReason::NoLease));
}

#[test]
fn redteam_bus_006_a_recomputed_crc_does_not_authenticate() {
    let rules = everything_allowed();
    let w = write_single(1, 100, 7);
    let body_len = w.len() - 2;
    let mut g = guard(&rules);
    lease(&mut g, &w, 1);

    for byte in 0..body_len {
        for bit in 0..8 {
            let mut body = w[..body_len].to_vec();
            body[byte] ^= 1 << bit;
            let forged = with_crc(&body);
            let got = g.decide(P, &forged, 2);
            // A flip that turns the write into a permitted read (function
            // 0x04) is an ordinary read, which needs no lease. Nothing else
            // may pass.
            let became_a_read = forged[1] == 0x04;
            assert!(became_a_read || !got.is_pass(), "byte {byte} bit {bit}");
            if (4..6).contains(&byte) {
                // Only the value changed: the frame is well formed and
                // permitted, so the lease digest is what refuses it.
                assert_eq!(got, Decision::Block(BlockReason::DigestMismatch));
            }
        }
    }
    // The two CRC bytes swapped.
    if w[body_len] != w[body_len + 1] {
        let mut swapped = w.clone();
        swapped.swap(body_len, body_len + 1);
        assert_eq!(
            g.decide(P, &swapped, 2),
            Decision::Block(BlockReason::Frame(FrameError::BadCrc))
        );
    }
    // Other frames whose CRC equals the approved frame's CRC: a CRC is 16
    // bits, so some register and value pairs collide. None may pass.
    let want_crc = [w[body_len], w[body_len + 1]];
    let mut collisions = 0;
    for register in 0..32u16 {
        for value in 0..=u16::MAX {
            let candidate = write_single(1, register, value);
            if candidate == w || candidate[body_len..] != want_crc {
                continue;
            }
            collisions += 1;
            assert_eq!(
                g.decide(P, &candidate, 2),
                Decision::Block(BlockReason::DigestMismatch)
            );
        }
    }
    assert!(collisions > 0, "the search found no colliding frame");
    assert!(g.lease().is_some());
    assert_eq!(g.decide(P, &w, 2), Decision::Pass);
}

#[test]
fn redteam_bus_011_a_write_sweep_yields_only_the_leased_frame() {
    // Read-only allow-list, no lease: no write passes, on any unit or address.
    let mut g = guard(&[rule(1, Function::ReadHoldingRegisters, 0, 100, 10)]);
    for unit in 1..=3u8 {
        for register in 0..=u16::MAX {
            let frame = write_single(unit, register, 1);
            assert!(
                !g.decide(P, &frame, 1).is_pass(),
                "unit {unit} reg {register}"
            );
        }
    }
    // One lease for W: the whole sweep yields exactly W.
    let mut g = guard(&everything_allowed());
    let w = write_single(1, 4242, 1);
    lease(&mut g, &w, 1);
    let mut passes = Vec::new();
    for register in 0..=u16::MAX {
        let frame = write_single(1, register, 1);
        if g.decide(P, &frame, 2).is_pass() {
            passes.push(register);
        }
    }
    assert_eq!(passes, [4242]);
}
