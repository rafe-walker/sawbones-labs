//! Bus guard attacks (lane `bus`). Each test is one catalog card.

mod support;

use sawbones_busguard::{BlockReason, FrameError, Function, Port, Verdict};
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
                Verdict::Block(BlockReason::Frame(want)),
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
                Verdict::Block(BlockReason::Frame(FrameError::UnsupportedFunction(code))),
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
            Verdict::Block(BlockReason::NotAllowListed)
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
            Verdict::Block(BlockReason::Frame(want))
        );
    }
    // Control: the frame that stays inside the rule passes once, with its lease.
    let ok = write_multiple(1, 100, &[1, 2]);
    let mut g = guard(&rules);
    lease(&mut g, &ok, 1);
    assert_eq!(g.decide(P, &ok, 2), Verdict::Pass);
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
        Verdict::Block(BlockReason::Frame(FrameError::LengthMismatch))
    );
    let mut trailing = w.clone();
    trailing.push(0);
    assert_eq!(
        g.decide(P, &trailing, 2),
        Verdict::Block(BlockReason::Frame(FrameError::LengthMismatch))
    );
    for split in 1..w.len() {
        assert!(!g.decide(P, &w[..split], 2).is_pass(), "first part {split}");
        assert!(
            !g.decide(P, &w[split..], 2).is_pass(),
            "second part {split}"
        );
    }
    assert!(g.lease().is_some(), "the lease survives every refusal");
    assert_eq!(g.decide(P, &w, 2), Verdict::Pass);
    assert_eq!(g.decide(P, &w, 2), Verdict::Block(BlockReason::NoLease));
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
                assert_eq!(got, Verdict::Block(BlockReason::DigestMismatch));
            }
        }
    }
    // The two CRC bytes swapped.
    if w[body_len] != w[body_len + 1] {
        let mut swapped = w.clone();
        swapped.swap(body_len, body_len + 1);
        assert_eq!(
            g.decide(P, &swapped, 2),
            Verdict::Block(BlockReason::Frame(FrameError::BadCrc))
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
                Verdict::Block(BlockReason::DigestMismatch)
            );
        }
    }
    assert!(collisions > 0, "the search found no colliding frame");
    assert!(g.lease().is_some());
    assert_eq!(g.decide(P, &w, 2), Verdict::Pass);
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

// --- bus-015 and bus-017 (#214): the signal's width is the entry's ----------

/// One write-map entry and one bench-only bus fact, for the width cards.
fn width_facts(
    width: u8,
    range: Option<sawbones_policy::ValueRange>,
) -> (sawbones_policy::WriteMap, sawbones_policy::BusFacts) {
    use sawbones_policy::*;
    use sawbones_reading::Trust;
    let writes = WriteMap::new([WriteEntry {
        port: PortId::parse("rs485-1").unwrap(),
        unit: 1,
        signal: SignalId::parse("battery_low_cutoff").unwrap(),
        register: 0x0100,
        width,
        class: HazardClass::Class3,
        kind: SettingKind::BatteryProtectionLimit,
        energy_relevant: true,
        trust: Trust::OperatorAttested,
        range,
    }])
    .unwrap();
    let buses = BusFacts::new([BusFact {
        port: PortId::parse("rs485-1").unwrap(),
        bench_port: 0,
        mastership: Mastership::BenchOnly,
        trust: Trust::OperatorAttested,
    }])
    .unwrap();
    (writes, buses)
}

fn width_write(value: sawbones_policy::WriteValue) -> sawbones_policy::EffectRequest {
    use sawbones_policy::*;
    EffectRequest::BusWrite(BusWrite {
        port: PortId::parse("rs485-1").unwrap(),
        unit: 1,
        signal: SignalId::parse("battery_low_cutoff").unwrap(),
        value,
    })
}

#[test]
fn redteam_bus_015_a_signal_width_comes_from_its_entry_and_policy_encoder_and_guard_agree() {
    use sawbones_policy::{classify, RefusalReason, Requirement, WriteValue};

    // Policy: a 32-bit value for a one-register signal would overwrite the
    // next register. It is refused, and so is the opposite.
    let (one, buses) = width_facts(1, None);
    for v in [WriteValue::U32(0x0001_0002), WriteValue::I32(-1)] {
        let d = classify(&width_write(v), &one, &buses);
        assert_eq!(
            d.requirement(),
            &Requirement::Refused {
                reason: RefusalReason::WidthMismatch {
                    expected: 1,
                    got: 2
                }
            }
        );
    }
    let (two, buses2) = width_facts(2, None);
    let d = classify(&width_write(WriteValue::U16(7)), &two, &buses2);
    assert!(d.is_refused());

    // Encoder: the same refusal one layer down.
    let words = [0x0001u16, 0x0002];
    assert!(sawbones_busguard::BusWriteAction::new_with_width(
        0,
        1,
        sawbones_busguard::WriteFunction::WriteMultipleRegisters,
        0x0100,
        1,
        &words
    )
    .is_err());

    // Guard: with a lease for the one-register action and rules wide enough
    // to allow both, the two-register frame to the same register is blocked.
    let action = sawbones_busguard::BusWriteAction::new_with_width(
        0,
        1,
        sawbones_busguard::WriteFunction::WriteSingleRegister,
        0x0100,
        1,
        &[0x0001],
    )
    .unwrap();
    let mut g = guard(&[
        rule(1, Function::WriteSingleRegister, 0, 65535, 1),
        rule(1, Function::WriteMultipleRegisters, 0, 65535, 2),
    ]);
    assert_eq!(action.frame().as_bytes(), write_single(1, 0x0100, 1));
    lease(&mut g, &write_single(1, 0x0100, 1), 1);
    let wide = write_multiple(1, 0x0100, &words);
    assert_eq!(
        g.decide(P, &wide, 2),
        Verdict::Block(BlockReason::DigestMismatch)
    );
    assert_eq!(g.decide(P, &write_single(1, 0x0100, 1), 2), Verdict::Pass);
}

#[test]
fn redteam_bus_017_a_value_of_the_wrong_width_is_refused_with_or_without_a_range() {
    use sawbones_policy::{classify, RefusalReason, Requirement, ValueRange, WriteValue};

    // An entry with no range used to accept a value of any type.
    let range = ValueRange::new(WriteValue::U16(4400), WriteValue::U16(5000)).unwrap();
    for entry_range in [None, Some(range)] {
        let (writes, buses) = width_facts(1, entry_range);
        for v in [WriteValue::U32(4600), WriteValue::I32(4600)] {
            let d = classify(&width_write(v), &writes, &buses);
            assert!(
                matches!(
                    d.requirement(),
                    Requirement::Refused {
                        reason: RefusalReason::WidthMismatch { .. }
                    }
                ),
                "{v:?} {entry_range:?}"
            );
        }
        // Control: a value of the entry's width is classified as before.
        let d = classify(&width_write(WriteValue::U16(4600)), &writes, &buses);
        assert!(matches!(d.requirement(), Requirement::RailBLease { .. }));
    }
}

// --- bus-015 and bus-017 (#174): the write map comes from a register map ------

const EXAMPLE_MAP: &str = include_str!("../../../data/register-maps/example-sim-device.toml");

fn regmap_body(extra: &str) -> String {
    format!(
        "schema = \"sawbones.regmap/0-provisional\"\n[device]\nid = \"t\"\naddress_base = 0\n{extra}"
    )
}

fn regmap_reg(name: &str, address: u32, table: &str, width: u8, ty: &str, rest: &str) -> String {
    format!(
        "\n[[register]]\nname = \"{name}\"\naddress = {address}\ntable = \"{table}\"\nwidth = {width}\ntype = \"{ty}\"\n{rest}\n"
    )
}

#[test]
fn redteam_bus_015_a_register_map_with_overlap_or_aliasing_does_not_load() {
    use sawbones_regmap::{MapError, RegisterMap};
    let ro = "access = \"read\"";

    // Two names on one register, and a two-register signal over its
    // neighbour, in either file order.
    let aliased = regmap_reg("a", 100, "holding", 1, "u16", ro)
        + &regmap_reg("b", 100, "holding", 1, "u16", ro);
    let spill = regmap_reg("next", 101, "holding", 1, "u16", ro)
        + &regmap_reg("wide", 100, "holding", 2, "u32", ro);
    for body in [aliased, spill] {
        assert!(
            matches!(
                RegisterMap::from_toml(&regmap_body(&body)),
                Err(MapError::Overlap { .. })
            ),
            "{body}"
        );
    }
    // Two names for one signal.
    let same_name =
        regmap_reg("a", 1, "holding", 1, "u16", ro) + &regmap_reg("a", 2, "holding", 1, "u16", ro);
    assert!(matches!(
        RegisterMap::from_toml(&regmap_body(&same_name)),
        Err(MapError::DuplicateName { .. })
    ));
    // A device whose tables alias: the same address in two tables is one
    // register, and the map must say so to be refused (or accepted without).
    let two =
        regmap_reg("m", 5, "input", 1, "u16", ro) + &regmap_reg("s", 5, "holding", 1, "u16", ro);
    assert!(RegisterMap::from_toml(&regmap_body(&two)).is_ok());
    let aliased_tables = format!(
        "schema = \"sawbones.regmap/0-provisional\"\n[device]\nid = \"t\"\naddress_base = 0\ninput_aliases_holding = true\n{two}"
    );
    assert!(matches!(
        RegisterMap::from_toml(&aliased_tables),
        Err(MapError::Overlap { .. })
    ));
    // One-based addresses: a one-based map read as zero-based would shift
    // every register by one. The base is required, so it cannot be assumed,
    // and address 0 does not exist in a one-based map.
    let one_based = format!(
        "schema = \"sawbones.regmap/0-provisional\"\n[device]\nid = \"t\"\naddress_base = 1\n{}",
        regmap_reg("a", 0, "holding", 1, "u16", ro)
    );
    assert!(matches!(
        RegisterMap::from_toml(&one_based),
        Err(MapError::BadAddress { .. })
    ));
}

#[test]
fn redteam_bus_015_both_words_of_a_two_register_signal_belong_to_it_and_a_shift_by_one_leaves_it() {
    use sawbones_policy::{classify, PortId, RefusalReason, Requirement, SignalId, WriteValue};
    use sawbones_reading::Trust;
    use sawbones_regmap::{RegisterMap, Table};

    let map = RegisterMap::from_toml(EXAMPLE_MAP).unwrap();
    let port = PortId::parse("rs485-1").unwrap();
    let writes = map.write_map(&port, 1, Trust::OperatorAttested).unwrap();
    for e in writes.entries() {
        let name = e.signal.as_str();
        let reg = map.register(name).unwrap();
        // Every word the entry's width covers resolves to this signal in the
        // map, so each inherits its class.
        for k in 0..u16::from(e.width) {
            assert_eq!(
                map.register_at(Table::Holding, e.register + k)
                    .unwrap()
                    .name(),
                name
            );
        }
        // A shift of one word either way is never the same signal: it is
        // another mapped register or nothing.
        for shifted in [
            e.register.checked_sub(1),
            e.register.checked_add(u16::from(e.width)),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(other) = map.register_at(Table::Holding, shifted) {
                assert_ne!(other.name(), name);
            }
        }
        assert_eq!(reg.width(), e.width);
    }
    // The two-register setpoint refuses a one-register value, and the
    // one-register brightness refuses a two-register value (the policy side
    // of the same fact).
    let buses = sawbones_policy::BusFacts::new([sawbones_policy::BusFact {
        port: port.clone(),
        bench_port: 0,
        mastership: sawbones_policy::Mastership::BenchOnly,
        trust: Trust::OperatorAttested,
    }])
    .unwrap();
    let write = |name: &str, value| {
        sawbones_policy::EffectRequest::BusWrite(sawbones_policy::BusWrite {
            port: port.clone(),
            unit: 1,
            signal: SignalId::parse(name).unwrap(),
            value,
        })
    };
    for (name, value) in [
        ("output_power_setpoint", WriteValue::I16(5)),
        ("display_brightness", WriteValue::I32(5)),
    ] {
        let d = classify(&write(name, value), &writes, &buses);
        assert!(matches!(
            d.requirement(),
            Requirement::Refused {
                reason: RefusalReason::WidthMismatch { .. }
            }
        ));
    }
}

#[test]
fn redteam_bus_017_every_write_entry_from_a_register_map_has_a_range_and_a_class_at_least_the_policys(
) {
    use sawbones_regmap::{MapError, RegisterMap};

    // The map cannot describe an unlimited or under-classed write.
    let w = |rest: &str| {
        regmap_body(&regmap_reg(
            "a",
            1,
            "holding",
            1,
            "u16",
            &format!("access = \"read_write\"\n{rest}"),
        ))
    };
    let no_range = w("class = \"class2\"\nkind = \"config\"\nenergy_relevant = false");
    assert!(matches!(
        RegisterMap::from_toml(&no_range),
        Err(MapError::WritableNeedsRange { .. })
    ));
    let no_class = w("min = \"0\"\nmax = \"100\"\nkind = \"config\"\nenergy_relevant = false");
    assert!(matches!(
        RegisterMap::from_toml(&no_class),
        Err(MapError::MissingWriteField { field: "class", .. })
    ));
    let low = w("min = \"0\"\nmax = \"100\"\nclass = \"class0\"\nkind = \"config\"\nenergy_relevant = false");
    assert!(matches!(
        RegisterMap::from_toml(&low),
        Err(MapError::ClassBelowFloor { .. })
    ));

    // Every entry a map produces carries its range, so none accepts any
    // value of the right width.
    let map = RegisterMap::from_toml(EXAMPLE_MAP).unwrap();
    let port = sawbones_policy::PortId::parse("rs485-1").unwrap();
    let entries = map.write_entries(&port, 1, sawbones_reading::Trust::OperatorAttested);
    assert!(!entries.is_empty());
    assert!(entries.iter().all(|e| e.range.is_some()));
}

#[test]
fn redteam_bus_017_a_value_outside_the_maps_range_is_refused_by_the_policy_and_the_decoder() {
    use sawbones_policy::{
        classify, BusFact, BusFacts, BusWrite, EffectRequest, Mastership, PortId, RefusalReason,
        Requirement, SignalId, WriteValue,
    };
    use sawbones_reading::Trust;
    use sawbones_regmap::{DecodeError, RegisterMap};

    let map = RegisterMap::from_toml(EXAMPLE_MAP).unwrap();
    let port = PortId::parse("rs485-1").unwrap();
    let writes = map.write_map(&port, 1, Trust::OperatorAttested).unwrap();
    let buses = BusFacts::new([BusFact {
        port: port.clone(),
        bench_port: 0,
        mastership: Mastership::BenchOnly,
        trust: Trust::OperatorAttested,
    }])
    .unwrap();
    let req = |v| {
        EffectRequest::BusWrite(BusWrite {
            port: port.clone(),
            unit: 1,
            signal: SignalId::parse("battery_low_cutoff").unwrap(),
            value: v,
        })
    };
    // 40.0 to 52.0 V at 0.1 V a step is raw 400 to 520.
    for v in [0u16, 399, 521, 0xFFFF] {
        let d = classify(&req(WriteValue::U16(v)), &writes, &buses);
        assert_eq!(
            d.requirement(),
            &Requirement::Refused {
                reason: RefusalReason::ValueOutOfRange
            },
            "{v}"
        );
    }
    for v in [400u16, 520] {
        let d = classify(&req(WriteValue::U16(v)), &writes, &buses);
        assert!(
            matches!(d.requirement(), Requirement::RailBLease { .. }),
            "{v}"
        );
    }
    // A sign flip: -1 sent as a one-register signed value to an unsigned
    // entry is the wrong type, so it is outside the range too.
    let d = classify(&req(WriteValue::I16(-1)), &writes, &buses);
    assert!(d.is_refused());
    // And reading: a reported value outside the valid range is refused, not
    // clamped.
    assert!(matches!(
        map.decode("battery_low_cutoff", &[0xFFFF]),
        Err(sawbones_regmap::ReadError::Decode(DecodeError::OutOfRange))
    ));
}
