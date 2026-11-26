//! The shared encoder (#203): golden frames and encodings, the constructor's
//! refusals, and randomised properties.

use sawbones_modbus_encode::{
    crc16_modbus, ActionDigest, ActionError, BusWriteAction, WriteFunction, ACTION_DOMAIN,
    ACTION_ENCODED_LEN, MAX_ACTION_FRAME_LEN, MAX_ACTION_WORDS, MAX_UNIT_ADDRESS,
};
use WriteFunction::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn action(
    port: u8,
    unit: u8,
    function: WriteFunction,
    register: u16,
    values: &[u16],
) -> BusWriteAction {
    BusWriteAction::new(port, unit, function, register, values).unwrap()
}

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

/// A random valid action.
fn random_action(rng: &mut Rng) -> BusWriteAction {
    let count = 1 + rng.below(MAX_ACTION_WORDS as u64) as usize;
    let function = if count == 1 && rng.below(2) == 0 {
        WriteSingleRegister
    } else {
        WriteMultipleRegisters
    };
    let values: Vec<u16> = (0..count).map(|_| rng.next() as u16).collect();
    let register = match rng.below(4) {
        0 => 0,
        1 => 0xFFFF - (count as u16 - 1),
        _ => rng.next() as u16 % (0xFFFF - 8),
    };
    let unit = 1 + rng.below(u64::from(MAX_UNIT_ADDRESS)) as u8;
    action(rng.next() as u8, unit, function, register, &values)
}

#[test]
fn regression_203_frames_match_the_modbus_specification() {
    // Application protocol v1.1b3, section 6.6, addressed to unit 0x11.
    assert_eq!(
        action(0, 0x11, WriteSingleRegister, 0x0001, &[0x0003])
            .frame()
            .as_bytes(),
        [0x11, 0x06, 0x00, 0x01, 0x00, 0x03, 0x9A, 0x9B]
    );
    // Widely quoted serial-line examples for unit 1, as the bus guard's own
    // frame builders pin them.
    assert_eq!(
        action(0, 0x01, WriteSingleRegister, 0x0001, &[0x0003])
            .frame()
            .as_bytes(),
        [0x01, 0x06, 0x00, 0x01, 0x00, 0x03, 0x98, 0x0B]
    );
    assert_eq!(
        action(0, 0x01, WriteMultipleRegisters, 0x0001, &[0x000A, 0x0102])
            .frame()
            .as_bytes(),
        [0x01, 0x10, 0x00, 0x01, 0x00, 0x02, 0x04, 0x00, 0x0A, 0x01, 0x02, 0x92, 0x30]
    );
    // The port is the line, not part of the frame.
    assert_eq!(
        action(3, 0x01, WriteSingleRegister, 0x0001, &[0x0003]).frame(),
        action(0, 0x01, WriteSingleRegister, 0x0001, &[0x0003]).frame()
    );
}

/// The action encoding and its digest, checked against Python's hashlib over
/// the same bytes. If this fails every lease digest changed: change
/// [`ACTION_DOMAIN`] and update both deliberately.
#[test]
fn regression_203_the_action_encoding_and_digest_are_pinned() {
    assert_eq!(
        ACTION_DOMAIN,
        b"sawbones-modbus-encode/bus-write-action/v1\0"
    );
    assert_eq!(ACTION_ENCODED_LEN, 14);
    assert_eq!(MAX_ACTION_FRAME_LEN, 17);

    let single = action(0, 0x11, WriteSingleRegister, 0x0001, &[0x0003]);
    assert_eq!(hex(&single.encode()), "0011060001010003000000000000");
    assert_eq!(
        hex(&single.digest().0),
        "10abc7f751bd2d4424f21176be887d2135a4401d38edcc2267a8712b4b0d1225"
    );

    let multiple = action(2, 0x11, WriteMultipleRegisters, 0x0001, &[0x000A, 0x0102]);
    assert_eq!(hex(&multiple.encode()), "021110000102000a010200000000");
    assert_eq!(
        hex(&multiple.digest().0),
        "b82d5a54bfb21c9fd99814a98fcc11bd34656879ae2fa32e96e1ab56dee3773f"
    );
    assert_eq!(ActionDigest::of(&multiple), multiple.digest());
}

#[test]
fn regression_203_the_constructor_refuses_what_no_frame_can_carry() {
    type Case<'a> = (&'a str, u8, WriteFunction, u16, &'a [u16], ActionError);
    let cases: [Case; 7] = [
        (
            "broadcast",
            0,
            WriteSingleRegister,
            1,
            &[1],
            ActionError::BroadcastUnit,
        ),
        (
            "reserved 248",
            248,
            WriteSingleRegister,
            1,
            &[1],
            ActionError::ReservedUnit(248),
        ),
        (
            "reserved 255",
            255,
            WriteMultipleRegisters,
            1,
            &[1],
            ActionError::ReservedUnit(255),
        ),
        (
            "no values",
            1,
            WriteMultipleRegisters,
            1,
            &[],
            ActionError::NoValues,
        ),
        (
            "too many",
            1,
            WriteMultipleRegisters,
            1,
            &[1, 2, 3, 4, 5],
            ActionError::TooManyValues,
        ),
        (
            "single of two",
            1,
            WriteSingleRegister,
            1,
            &[1, 2],
            ActionError::SingleNeedsOneValue,
        ),
        (
            "past 0xFFFF",
            1,
            WriteMultipleRegisters,
            0xFFFF,
            &[1, 2],
            ActionError::AddressOverflow,
        ),
    ];
    for (name, unit, function, register, values, error) in cases {
        assert_eq!(
            BusWriteAction::new(0, unit, function, register, values),
            Err(error),
            "{name}"
        );
    }
    // The edges that are allowed.
    assert!(BusWriteAction::new(0, 1, WriteSingleRegister, 0xFFFF, &[1]).is_ok());
    assert!(
        BusWriteAction::new(0, MAX_UNIT_ADDRESS, WriteMultipleRegisters, 0xFFFC, &[1; 4]).is_ok()
    );
}

#[test]
fn regression_203_decode_refuses_every_non_canonical_form() {
    let good = action(1, 2, WriteMultipleRegisters, 0x0010, &[7, 8]).encode();
    let with = |at: usize, value: u8| {
        let mut bytes = good;
        bytes[at] = value;
        BusWriteAction::decode(&bytes)
    };
    assert_eq!(with(1, 0), Err(ActionError::BroadcastUnit));
    assert_eq!(with(1, 250), Err(ActionError::ReservedUnit(250)));
    assert_eq!(with(2, 0x05), Err(ActionError::UnknownFunction(0x05)));
    assert_eq!(with(2, 0x06), Err(ActionError::SingleNeedsOneValue));
    assert_eq!(with(5, 0), Err(ActionError::NoValues));
    assert_eq!(with(5, 5), Err(ActionError::TooManyValues));
    for at in 10..ACTION_ENCODED_LEN {
        assert_eq!(with(at, 1), Err(ActionError::NonZeroPadding), "byte {at}");
    }
    assert_eq!(
        BusWriteAction::decode(&good[..13]),
        Err(ActionError::Length)
    );
    assert_eq!(BusWriteAction::decode(&[0; 15]), Err(ActionError::Length));
}

#[test]
fn regression_203_random_actions_round_trip_and_encode_one_frame() {
    let mut rng = Rng(0x0203);
    for seed in 0..20_000 {
        let a = random_action(&mut rng);
        // One encoding, and decoding gives the action back.
        assert_eq!(BusWriteAction::decode(&a.encode()), Ok(a), "seed {seed}");
        // The frame is exactly as long as the function needs, with a
        // correct CRC, and carries the unit, register and values.
        let frame = a.frame();
        let bytes = frame.as_bytes();
        let n = a.values().len();
        let body_len = match a.function() {
            WriteSingleRegister => 6,
            WriteMultipleRegisters => 7 + 2 * n,
        };
        assert_eq!(bytes.len(), body_len + 2, "seed {seed}");
        let (body, crc) = bytes.split_at(body_len);
        assert_eq!(crc, crc16_modbus(body).to_le_bytes(), "seed {seed}");
        assert_eq!(body[0], a.unit());
        assert_eq!(body[1], a.function().code());
        assert_eq!(u16::from_be_bytes([body[2], body[3]]), a.register());
        let values = &body[body_len - 2 * n..];
        let got: Vec<u16> = values
            .chunks_exact(2)
            .map(|p| u16::from_be_bytes([p[0], p[1]]))
            .collect();
        assert_eq!(got, a.values(), "seed {seed}");
        // Deterministic.
        assert_eq!(a.frame(), frame);
        assert_eq!(a.digest(), a.digest());
    }
}

#[test]
fn regression_203_every_one_field_change_changes_the_encoding_and_digest() {
    let mut rng = Rng(0x2030);
    for seed in 0..5_000 {
        let a = random_action(&mut rng);
        let values = a.values().to_vec();
        let mut changed = Vec::new();
        changed.push(BusWriteAction::new(
            a.port().wrapping_add(1),
            a.unit(),
            a.function(),
            a.register(),
            &values,
        ));
        changed.push(BusWriteAction::new(
            a.port(),
            a.unit() % MAX_UNIT_ADDRESS + 1,
            a.function(),
            a.register(),
            &values,
        ));
        changed.push(BusWriteAction::new(
            a.port(),
            a.unit(),
            a.function(),
            a.register().wrapping_add(1),
            &values,
        ));
        let mut other = values.clone();
        other[0] = other[0].wrapping_add(1);
        changed.push(BusWriteAction::new(
            a.port(),
            a.unit(),
            a.function(),
            a.register(),
            &other,
        ));
        let flipped = match a.function() {
            WriteSingleRegister => WriteMultipleRegisters,
            WriteMultipleRegisters => WriteSingleRegister,
        };
        changed.push(BusWriteAction::new(
            a.port(),
            a.unit(),
            flipped,
            a.register(),
            &values,
        ));
        for b in changed.into_iter().flatten() {
            assert_ne!(a, b, "seed {seed}");
            assert_ne!(a.encode(), b.encode(), "seed {seed}: {a:?} {b:?}");
            assert_ne!(a.digest(), b.digest(), "seed {seed}: {a:?} {b:?}");
            // Frames differ too, unless only the port changed.
            if a.port() == b.port() {
                assert_ne!(a.frame(), b.frame(), "seed {seed}: {a:?} {b:?}");
            }
        }
    }
}

// --- #214: the register count comes from the entry's width -------------------

#[test]
fn regression_214_encoder_refuses_a_register_count_that_differs_from_the_width() {
    // Two registers into a one-register signal, and one into a two-register
    // signal, for every function and a spread of registers.
    for register in [0u16, 1, 0x0100, 0xFFFD] {
        assert_eq!(
            BusWriteAction::new_with_width(0, 1, WriteMultipleRegisters, register, 1, &[1, 2]),
            Err(ActionError::WidthMismatch {
                expected: 1,
                got: 2
            })
        );
        assert_eq!(
            BusWriteAction::new_with_width(0, 1, WriteSingleRegister, register, 2, &[1]),
            Err(ActionError::WidthMismatch {
                expected: 2,
                got: 1
            })
        );
        assert_eq!(
            BusWriteAction::new_with_width(0, 1, WriteMultipleRegisters, register, 2, &[1]),
            Err(ActionError::WidthMismatch {
                expected: 2,
                got: 1
            })
        );
    }
    assert_eq!(
        BusWriteAction::new_with_width(0, 1, WriteMultipleRegisters, 5, 2, &[1, 2, 3]),
        Err(ActionError::WidthMismatch {
            expected: 2,
            got: 3
        })
    );
    // Width 0 matches no non-empty write.
    assert_eq!(
        BusWriteAction::new_with_width(0, 1, WriteMultipleRegisters, 5, 0, &[1]),
        Err(ActionError::WidthMismatch {
            expected: 0,
            got: 1
        })
    );
}

#[test]
fn regression_214_a_matching_width_builds_the_same_action_and_digest() {
    // The width is not hashed: an action built with it equals, field for
    // field and digest for digest, the one built without it.
    for (function, values) in [
        (WriteSingleRegister, &[0x1234u16][..]),
        (WriteMultipleRegisters, &[0x1234, 0x5678][..]),
        (WriteMultipleRegisters, &[7][..]),
    ] {
        let width = u8::try_from(values.len()).unwrap();
        let with = BusWriteAction::new_with_width(3, 9, function, 0x0100, width, values).unwrap();
        let without = BusWriteAction::new(3, 9, function, 0x0100, values).unwrap();
        assert_eq!(with, without);
        assert_eq!(with.digest(), without.digest());
        assert_eq!(with.encode(), without.encode());
    }
}
