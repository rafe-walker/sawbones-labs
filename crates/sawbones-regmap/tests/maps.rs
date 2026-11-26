//! Loading, decoding and the policy agreement for register maps (#174).

use sawbones_policy::{
    classify, BusFact, BusFacts, BusWrite, EffectRequest, HazardClass, Mastership, PortId,
    RefusalReason, Requirement, SettingKind, SignalId, WriteValue,
};
use sawbones_reading::{
    Ident, Provenance, TimestampQuality, Transport, Trust, UnitBasis, UtcNanos, WorldText,
};
use sawbones_regmap::{
    Access, ByteOrder, Codec, DataType, Decimal, DecodeError, MapError, ObservationContext,
    ReadError, RegisterMap, Table, WordOrder,
};

const EXAMPLE: &str = include_str!("../../../data/register-maps/example-sim-device.toml");

fn example() -> RegisterMap {
    RegisterMap::from_toml(EXAMPLE).expect("the example map loads")
}

fn d(s: &str) -> Decimal {
    Decimal::parse(s).unwrap()
}

/// A map with the given register tables appended to a valid header.
fn map_with(body: &str) -> String {
    format!(
        "schema = \"sawbones.regmap/0-provisional\"\n[device]\nid = \"t\"\naddress_base = 0\n{body}"
    )
}

/// One register table, with extra keys appended.
fn reg(name: &str, address: u32, table: &str, width: u8, ty: &str, extra: &str) -> String {
    format!(
        "\n[[register]]\nname = \"{name}\"\naddress = {address}\ntable = \"{table}\"\nwidth = {width}\ntype = \"{ty}\"\n{extra}\n"
    )
}

fn load(body: &str) -> Result<RegisterMap, MapError> {
    RegisterMap::from_toml(&map_with(body))
}

const WRITABLE: &str = "access = \"read_write\"\nmin = \"0\"\nmax = \"100\"\nclass = \"class2\"\nkind = \"config\"\nenergy_relevant = false";

// --- the example ---------------------------------------------------------------

#[test]
fn the_example_loads_and_translates_one_based_addresses() {
    let m = example();
    assert_eq!(m.device_id().as_str(), "example-sim-device");
    let pv = m.register("pv_power").unwrap();
    assert_eq!(
        (pv.table(), pv.address(), pv.width()),
        (Table::Input, 100, 1)
    );
    assert_eq!(m.register("energy_total").unwrap().address(), 104);
    assert_eq!(
        m.register_at(Table::Input, 105).unwrap().name(),
        "energy_total"
    );
    assert!(m.register_at(Table::Input, 106).is_none());
    assert!(m.register_at(Table::Holding, 100).is_none());
}

#[test]
fn the_example_decodes_to_exact_values_with_units() {
    let m = example();
    let v = |name: &str, words: &[u16]| m.decode(name, words).unwrap().to_text();
    assert_eq!(v("pv_power", &[1823]).as_str(), "1823");
    assert_eq!(v("battery_voltage", &[5320]).as_str(), "53.20");
    assert_eq!(v("battery_current", &[0xFF9C]).as_str(), "-10.0");
    assert_eq!(v("heatsink_temperature", &[130]).as_str(), "25.0");
    // 123456 tenths of a kWh, low word first, low byte first in each word.
    let raw: u32 = 123_456;
    let words = [(raw as u16).swap_bytes(), ((raw >> 16) as u16).swap_bytes()];
    assert_eq!(v("energy_total", &words).as_str(), "12345.6");
    assert_eq!(v("output_power_setpoint", &[0xFFFF, 0xFFFE]).as_str(), "-2");
    assert_eq!(
        m.register("battery_voltage")
            .unwrap()
            .unit()
            .unwrap()
            .as_str(),
        "V"
    );
}

#[test]
fn the_example_gives_an_observation_with_no_trust_and_the_declared_unit() {
    let m = example();
    let ctx = ObservationContext {
        source_id: Ident::new("example-sim-device/unit1").unwrap(),
        received_at: UtcNanos::from_unix_nanos(1_795_631_400_000_000_000),
        source_timestamp: None,
        timestamp_quality: TimestampQuality::HostReceive {
            uncertainty_ns: 1_000_000_000,
        },
        provenance: Provenance {
            transport: Transport::Mqtt,
            broker: "sim.local:1883".to_owned(),
            topic: WorldText::new("sim/words", 64),
            raw_payload: WorldText::new("5320", 64),
            retained: false,
        },
    };
    let o = m.observe("battery_voltage", &[5320], &ctx).unwrap();
    assert_eq!(o.quantity.as_str(), "battery_voltage");
    assert_eq!(o.value.text(), "53.20");
    assert_eq!(o.unit.as_ref().unwrap().as_str(), "V");
    assert_eq!(o.unit_basis, UnitBasis::DeclaredBySource);
    // The same refusals as decode.
    assert_eq!(
        m.observe("battery_voltage", &[10], &ctx),
        Err(ReadError::Decode(DecodeError::OutOfRange))
    );
    assert!(matches!(
        m.observe("nope", &[1], &ctx),
        Err(ReadError::UnknownRegister(_))
    ));
}

#[test]
fn decode_block_reports_each_register_and_skips_the_ones_not_wholly_inside() {
    let m = example();
    // Input words from wire address 100: pv, voltage, current, temp, energy.
    let words = [1823u16, 5320, 0xFF9C, 130, 0, 0];
    let out = m.decode_block(Table::Input, 100, &words);
    let names: Vec<_> = out.iter().map(|(r, _)| r.name()).collect();
    assert_eq!(
        names,
        [
            "pv_power",
            "battery_voltage",
            "battery_current",
            "heatsink_temperature",
            "energy_total"
        ]
    );
    assert!(out.iter().all(|(_, r)| r.is_ok()));
    // A block that stops in the middle of the two-register value drops it.
    let short = m.decode_block(Table::Input, 100, &words[..5]);
    assert_eq!(short.len(), 4);
    // One bad register does not hide the rest.
    let bad = m.decode_block(Table::Input, 100, &[1823, 1, 0xFF9C]);
    assert_eq!(bad.len(), 3);
    assert!(bad[1].1.is_err() && bad[0].1.is_ok() && bad[2].1.is_ok());
}

#[test]
fn a_write_only_register_is_not_readable() {
    let m = load(&reg(
        "reset_counter",
        1,
        "holding",
        1,
        "u16",
        "access = \"write\"\nmin = \"0\"\nmax = \"1\"\nclass = \"class3\"\nkind = \"config\"\nenergy_relevant = true",
    ))
    .unwrap();
    assert_eq!(m.register("reset_counter").unwrap().access(), Access::Write);
    assert_eq!(
        m.decode("reset_counter", &[1]),
        Err(ReadError::NotReadable("reset_counter".to_owned()))
    );
}

// --- decoding properties -------------------------------------------------------

/// A small deterministic generator, so the properties need no new crate.
struct Xorshift(u64);

impl Xorshift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

const TYPES: [DataType; 6] = [
    DataType::U16,
    DataType::I16,
    DataType::U32,
    DataType::I32,
    DataType::U64,
    DataType::I64,
];

fn raw_in_type(t: DataType, rng: &mut Xorshift) -> i128 {
    let span = (t.max_raw() - t.min_raw()) as u128 + 1;
    let pick = (u128::from(rng.next()) << 64 | u128::from(rng.next())) % span;
    t.min_raw() + pick as i128
}

#[test]
fn a_raw_value_round_trips_through_words_in_every_order() {
    let mut rng = Xorshift(0x9E37_79B9_7F4A_7C15);
    for t in TYPES {
        for bo in [ByteOrder::Big, ByteOrder::Little] {
            for wo in [WordOrder::Big, WordOrder::Little] {
                let c = Codec::new(t, bo, wo, Decimal::ONE, Decimal::ZERO, None, None).unwrap();
                let mut cases = vec![t.min_raw(), t.max_raw(), 0, 1];
                cases.extend((0..500).map(|_| raw_in_type(t, &mut rng)));
                for raw in cases {
                    let (words, n) = c.words_for_raw(raw).unwrap();
                    assert_eq!(n, usize::from(t.words()));
                    assert_eq!(c.raw(&words[..n]).unwrap(), raw, "{t:?} {bo:?} {wo:?}");
                }
                // Out of the type is refused, one past each end.
                assert_eq!(
                    c.words_for_raw(t.max_raw() + 1),
                    Err(DecodeError::RawOutsideType)
                );
                assert_eq!(
                    c.words_for_raw(t.min_raw() - 1),
                    Err(DecodeError::RawOutsideType)
                );
            }
        }
    }
}

#[test]
fn the_decoded_value_is_exactly_raw_times_scale_plus_offset() {
    let mut rng = Xorshift(0xD1B5_4A32_D192_ED03);
    for (scale, offset) in [
        ("1", "0"),
        ("0.1", "0"),
        ("0.01", "-40"),
        ("-0.5", "10"),
        ("25", "0.125"),
    ] {
        for t in TYPES {
            let c = Codec::new(
                t,
                ByteOrder::Big,
                WordOrder::Big,
                d(scale),
                d(offset),
                None,
                None,
            )
            .unwrap();
            for _ in 0..200 {
                let raw = raw_in_type(t, &mut rng);
                let (words, n) = c.words_for_raw(raw).unwrap();
                let v = c.decode(&words[..n]).unwrap();
                // Check against independent integer arithmetic at the
                // common scale, then the text.
                let (sm, ss) = (c.scale().mantissa(), u32::from(c.scale().scale()));
                let (om, os) = (c.offset().mantissa(), u32::from(c.offset().scale()));
                let dd = ss.max(os);
                let want = raw * sm * 10i128.pow(dd - ss) + om * 10i128.pow(dd - os);
                assert_eq!(
                    v.cmp_value(&Decimal::new(want, dd as u8).unwrap()),
                    std::cmp::Ordering::Equal
                );
                assert_eq!(v.scale(), dd as u8);
                // And back: the raw value is recoverable from the value.
                assert_eq!(
                    c.raw_floor_ceil(v).unwrap(),
                    (raw, raw),
                    "{scale} {offset} {t:?}"
                );
            }
        }
    }
}

#[test]
fn scale_is_monotone_in_the_raw_value_and_sign_comes_from_the_type() {
    let c = Codec::new(
        DataType::I16,
        ByteOrder::Big,
        WordOrder::Big,
        d("0.1"),
        Decimal::ZERO,
        None,
        None,
    )
    .unwrap();
    let mut last: Option<Decimal> = None;
    for raw in (i16::MIN..=i16::MAX).step_by(7) {
        let v = c.decode(&[raw as u16]).unwrap();
        if let Some(l) = last {
            assert_eq!(l.cmp_value(&v), std::cmp::Ordering::Less);
        }
        assert_eq!(v.mantissa() < 0, raw < 0);
        last = Some(v);
    }
}

#[test]
fn a_value_outside_the_range_is_refused_at_both_edges_for_every_word_value() {
    let c = Codec::new(
        DataType::U16,
        ByteOrder::Big,
        WordOrder::Big,
        d("0.1"),
        d("-5"),
        Some(d("0.0")),
        Some(d("20.0")),
    )
    .unwrap();
    // raw 50 -> 0.0, raw 250 -> 20.0
    for w in 0..=u16::MAX {
        let r = c.decode(&[w]);
        if (50..=250).contains(&w) {
            assert!(r.is_ok(), "{w}");
        } else {
            assert_eq!(r, Err(DecodeError::OutOfRange), "{w}");
        }
    }
}

#[test]
fn the_decoder_is_total_and_never_prints_a_float() {
    // Every word pair for a 32-bit register gives a value or a named error.
    let c = Codec::new(
        DataType::I32,
        ByteOrder::Little,
        WordOrder::Little,
        d("0.001"),
        d("0.5"),
        Some(d("-1000")),
        Some(d("1000")),
    )
    .unwrap();
    let mut rng = Xorshift(7);
    for _ in 0..20_000 {
        let w = [rng.next() as u16, rng.next() as u16];
        match c.decode(&w) {
            Ok(v) => {
                let t = v.to_text();
                assert!(!t.as_str().contains(['e', 'E', 'N', 'i']), "{t:?}");
                assert!(Decimal::parse(t.as_str()).is_ok());
            }
            Err(DecodeError::OutOfRange) => {}
            Err(e) => panic!("{e}"),
        }
    }
    assert!(c.decode(&[1]).is_err());
}

// --- strict loading: one refusal per rule --------------------------------------

#[test]
fn refuses_an_unknown_key_at_every_level() {
    let top = format!(
        "{}\nbogus = 1\n",
        map_with(&reg("a", 0, "input", 1, "u16", "access = \"read\""))
    );
    assert!(matches!(
        RegisterMap::from_toml(&top),
        Err(MapError::Parse(_))
    ));
    let dev = "schema = \"sawbones.regmap/0-provisional\"\n[device]\nid = \"t\"\naddress_base = 0\nextra = 1\n".to_owned()
        + &reg("a", 0, "input", 1, "u16", "access = \"read\"");
    assert!(matches!(
        RegisterMap::from_toml(&dev),
        Err(MapError::Parse(_))
    ));
    let r = load(&reg(
        "a",
        0,
        "input",
        1,
        "u16",
        "access = \"read\"\ncolour = \"red\"",
    ));
    assert!(matches!(r, Err(MapError::Parse(_))));
}

#[test]
fn refuses_missing_required_keys_and_floats() {
    // No type, no width, no access.
    let r = load("\n[[register]]\nname = \"a\"\naddress = 0\ntable = \"input\"\nwidth = 1\naccess = \"read\"\n");
    assert!(matches!(r, Err(MapError::Parse(_))));
    // A float scale is not text, so it does not load.
    let r = load(&reg(
        "a",
        0,
        "input",
        1,
        "u16",
        "access = \"read\"\nscale = 0.1",
    ));
    assert!(matches!(r, Err(MapError::Parse(_))));
    // An unknown type and an unknown table.
    assert!(matches!(
        load(&reg("a", 0, "input", 1, "f32", "access = \"read\"")),
        Err(MapError::Parse(_))
    ));
    assert!(matches!(
        load(&reg("a", 0, "coil", 1, "u16", "access = \"read\"")),
        Err(MapError::Parse(_))
    ));
}

#[test]
fn refuses_a_bad_schema_device_and_empty_or_oversized_maps() {
    let r = RegisterMap::from_toml("schema = \"x\"\n[device]\nid = \"t\"\naddress_base = 0\n[[register]]\nname=\"a\"\naddress=0\ntable=\"input\"\nwidth=1\ntype=\"u16\"\naccess=\"read\"\n");
    assert_eq!(r.unwrap_err(), MapError::UnsupportedSchema("x".to_owned()));
    let r = RegisterMap::from_toml("schema = \"sawbones.regmap/0-provisional\"\n[device]\nid = \"has space\"\naddress_base = 0\n[[register]]\nname=\"a\"\naddress=0\ntable=\"input\"\nwidth=1\ntype=\"u16\"\naccess=\"read\"\n");
    assert_eq!(r.unwrap_err(), MapError::BadDeviceId);
    let r = RegisterMap::from_toml("schema = \"sawbones.regmap/0-provisional\"\n[device]\nid = \"t\"\naddress_base = 2\n[[register]]\nname=\"a\"\naddress=0\ntable=\"input\"\nwidth=1\ntype=\"u16\"\naccess=\"read\"\n");
    assert_eq!(r.unwrap_err(), MapError::BadAddressBase(2));
    assert_eq!(
        RegisterMap::from_toml("schema = \"sawbones.regmap/0-provisional\"\nregister = []\n[device]\nid = \"t\"\naddress_base = 0\n").unwrap_err(),
        MapError::NoRegisters
    );
    let huge = " ".repeat(sawbones_regmap::MAX_MAP_BYTES + 1);
    assert_eq!(
        RegisterMap::from_toml(&huge).unwrap_err(),
        MapError::TooLarge
    );
}

#[test]
fn refuses_bad_and_duplicate_names() {
    for bad in ["Upper", "1st", "has space", "", "a.b"] {
        let r = load(&reg(bad, 0, "input", 1, "u16", "access = \"read\""));
        assert_eq!(
            r.unwrap_err(),
            MapError::BadName {
                name: bad.to_owned()
            },
            "{bad:?}"
        );
    }
    let body = reg("a", 0, "input", 1, "u16", "access = \"read\"")
        + &reg("a", 5, "input", 1, "u16", "access = \"read\"");
    assert_eq!(
        load(&body).unwrap_err(),
        MapError::DuplicateName {
            name: "a".to_owned()
        }
    );
}

#[test]
fn refuses_a_width_that_does_not_fit_the_type() {
    for (ty, width) in [
        ("u16", 2u8),
        ("u16", 0),
        ("i32", 1),
        ("u32", 4),
        ("u64", 2),
        ("i64", 3),
    ] {
        let r = load(&reg("a", 0, "input", width, ty, "access = \"read\""));
        assert!(
            matches!(r, Err(MapError::WidthMismatch { .. })),
            "{ty} width {width}: {r:?}"
        );
    }
    assert!(load(&reg("a", 0, "input", 4, "u64", "access = \"read\"")).is_ok());
}

#[test]
fn refuses_addresses_outside_the_range_and_spans_past_the_end() {
    // address_base = 0, so 65536 is out; 65535 is the last register.
    let r = load(&reg("a", 65536, "input", 1, "u16", "access = \"read\""));
    assert!(matches!(r, Err(MapError::BadAddress { .. })));
    assert!(load(&reg("a", 65535, "input", 1, "u16", "access = \"read\"")).is_ok());
    let r = load(&reg("a", 65535, "input", 2, "u32", "access = \"read\""));
    assert!(matches!(r, Err(MapError::SpanPastEnd { .. })));
    // One-based: address 0 does not exist.
    let r = RegisterMap::from_toml(&format!(
        "schema = \"sawbones.regmap/0-provisional\"\n[device]\nid = \"t\"\naddress_base = 1\n{}",
        reg("a", 0, "input", 1, "u16", "access = \"read\"")
    ));
    assert!(matches!(r, Err(MapError::BadAddress { .. })));
}

#[test]
fn refuses_numbers_that_are_not_plain_decimals_and_unsound_arithmetic() {
    for (field, bad) in [
        ("scale", "1e3"),
        ("scale", "NaN"),
        ("offset", "0x10"),
        ("min", ""),
        ("max", "1,5"),
        ("scale", "0.0000000001"),
    ] {
        let r = load(&reg(
            "a",
            0,
            "input",
            1,
            "u16",
            &format!("access = \"read\"\n{field} = \"{bad}\""),
        ));
        assert!(
            matches!(r, Err(MapError::BadNumber { field: f, .. }) if f == field),
            "{field}={bad:?}: {r:?}"
        );
    }
    let r = load(&reg(
        "a",
        0,
        "input",
        1,
        "u16",
        "access = \"read\"\nscale = \"0\"",
    ));
    assert!(matches!(
        r,
        Err(MapError::Codec {
            error: sawbones_regmap::CodecError::ZeroScale,
            ..
        })
    ));
    let r = load(&reg(
        "a",
        0,
        "input",
        1,
        "u16",
        "access = \"read\"\nmin = \"5\"\nmax = \"4\"",
    ));
    assert!(matches!(
        r,
        Err(MapError::Codec {
            error: sawbones_regmap::CodecError::RangeInverted,
            ..
        })
    ));
    let r = load(&reg(
        "a",
        0,
        "input",
        4,
        "u64",
        "access = \"read\"\nscale = \"99999999999999.9999\"\noffset = \"0.000000001\"",
    ));
    assert!(matches!(
        r,
        Err(MapError::Codec {
            error: sawbones_regmap::CodecError::CanOverflow,
            ..
        })
    ));
    let r = load(&reg(
        "a",
        0,
        "input",
        1,
        "u16",
        "access = \"read\"\nunit = \"a b\"",
    ));
    assert!(matches!(r, Err(MapError::BadUnit { .. })));
}

#[test]
fn refuses_a_word_order_on_a_single_register() {
    let r = load(&reg(
        "a",
        0,
        "input",
        1,
        "u16",
        "access = \"read\"\nword_order = \"little\"",
    ));
    assert!(matches!(r, Err(MapError::WordOrderOnSingle { .. })));
    assert!(load(&reg(
        "a",
        0,
        "input",
        1,
        "u16",
        "access = \"read\"\nbyte_order = \"little\""
    ))
    .is_ok());
}

#[test]
fn refuses_overlapping_and_aliased_registers() {
    // Same address.
    let same = reg("a", 10, "holding", 1, "u16", "access = \"read\"")
        + &reg("b", 10, "holding", 1, "u16", "access = \"read\"");
    assert_eq!(
        load(&same).unwrap_err(),
        MapError::Overlap {
            first: "a".into(),
            second: "b".into()
        }
    );
    // A two-register value runs into the next register, whatever the file order.
    let wide = reg("neighbour", 11, "holding", 1, "u16", "access = \"read\"")
        + &reg("wide", 10, "holding", 2, "u32", "access = \"read\"");
    assert_eq!(
        load(&wide).unwrap_err(),
        MapError::Overlap {
            first: "wide".into(),
            second: "neighbour".into()
        }
    );
    // Touching is not overlapping.
    let touch = reg("a", 10, "holding", 2, "u32", "access = \"read\"")
        + &reg("b", 12, "holding", 1, "u16", "access = \"read\"");
    assert!(load(&touch).is_ok());
    // A register in the middle of a four-register value.
    let inside = reg("big", 20, "holding", 4, "u64", "access = \"read\"")
        + &reg("mid", 22, "holding", 1, "u16", "access = \"read\"");
    assert!(matches!(load(&inside), Err(MapError::Overlap { .. })));
}

#[test]
fn input_and_holding_may_share_an_address_unless_the_device_says_they_alias() {
    let body = reg("m", 10, "input", 1, "u16", "access = \"read\"")
        + &reg("s", 10, "holding", 1, "u16", "access = \"read\"");
    assert!(load(&body).is_ok());
    let aliased = format!(
        "schema = \"sawbones.regmap/0-provisional\"\n[device]\nid = \"t\"\naddress_base = 0\ninput_aliases_holding = true\n{body}"
    );
    assert_eq!(
        RegisterMap::from_toml(&aliased).unwrap_err(),
        MapError::Overlap {
            first: "m".into(),
            second: "s".into()
        }
    );
}

#[test]
fn refuses_a_writable_register_that_is_unsafe_to_describe() {
    // Missing class, kind, energy flag: each named.
    for (drop, field) in [
        ("class", "class"),
        ("kind", "kind"),
        ("energy_relevant", "energy_relevant"),
    ] {
        let extra: String = WRITABLE
            .lines()
            .filter(|l| !l.starts_with(drop))
            .collect::<Vec<_>>()
            .join("\n");
        let r = load(&reg("a", 0, "holding", 1, "u16", &extra));
        assert_eq!(
            r.unwrap_err(),
            MapError::MissingWriteField {
                name: "a".into(),
                field
            },
            "{drop}"
        );
    }
    // No range at all, or only one end.
    for extra in [
        "access = \"read_write\"\nclass = \"class2\"\nkind = \"config\"\nenergy_relevant = false",
        "access = \"read_write\"\nmin = \"0\"\nclass = \"class2\"\nkind = \"config\"\nenergy_relevant = false",
    ] {
        let r = load(&reg("a", 0, "holding", 1, "u16", extra));
        assert!(matches!(r, Err(MapError::WritableNeedsRange { .. })), "{extra}");
    }
    // Input registers are read only.
    let r = load(&reg("a", 0, "input", 1, "u16", WRITABLE));
    assert!(matches!(r, Err(MapError::WritableInputTable { .. })));
    // The policy can write one or two registers.
    let r = load(&reg("a", 0, "holding", 4, "u64", WRITABLE));
    assert!(matches!(r, Err(MapError::WritableTooWide { .. })));
}

#[test]
fn refuses_a_class_below_the_policy_floor() {
    let with = |class: &str, kind: &str, energy: bool| {
        load(&reg(
            "a",
            0,
            "holding",
            1,
            "u16",
            &format!("access = \"read_write\"\nmin = \"0\"\nmax = \"100\"\nclass = \"{class}\"\nkind = \"{kind}\"\nenergy_relevant = {energy}"),
        ))
    };
    // Any write transmits, so it is at least class 2.
    assert!(matches!(
        with("class0", "config", false),
        Err(MapError::ClassBelowFloor { .. })
    ));
    assert!(matches!(
        with("class1", "config", false),
        Err(MapError::ClassBelowFloor { .. })
    ));
    assert!(with("class2", "config", false).is_ok());
    // A battery protection limit or charge voltage is class 3 at least.
    assert!(matches!(
        with("class2", "battery_protection_limit", true),
        Err(MapError::ClassBelowFloor { .. })
    ));
    assert!(matches!(
        with("class2", "charge_voltage", false),
        Err(MapError::ClassBelowFloor { .. })
    ));
    assert!(with("class3", "charge_voltage", false).is_ok());
    // Mains and PV string switching are class 4.
    assert!(matches!(
        with("class3", "mains_switching", true),
        Err(MapError::ClassBelowFloor { .. })
    ));
    assert!(with("class4", "mains_switching", true).is_ok());
}

#[test]
fn refuses_class_fields_on_a_read_only_register() {
    for (field, line) in [
        ("class", "class = \"class2\""),
        ("kind", "kind = \"config\""),
        ("energy_relevant", "energy_relevant = false"),
    ] {
        let r = load(&reg(
            "a",
            0,
            "holding",
            1,
            "u16",
            &format!("access = \"read\"\n{line}"),
        ));
        assert_eq!(
            r.unwrap_err(),
            MapError::WriteFieldOnReadOnly {
                name: "a".into(),
                field
            }
        );
    }
}

#[test]
fn refuses_a_range_with_no_writable_raw_value() {
    // Only 0 to 100 fit a u16 at scale 1; a range wholly above is empty.
    let r = load(&reg(
        "a",
        0,
        "holding",
        1,
        "u16",
        "access = \"read_write\"\nmin = \"70000\"\nmax = \"80000\"\nclass = \"class2\"\nkind = \"config\"\nenergy_relevant = false",
    ));
    assert!(matches!(r, Err(MapError::EmptyRawRange { .. })));
    // A range that falls between two reachable values is empty too.
    let r = load(&reg(
        "a",
        0,
        "holding",
        1,
        "u16",
        "access = \"read_write\"\nscale = \"10\"\nmin = \"1\"\nmax = \"9\"\nclass = \"class2\"\nkind = \"config\"\nenergy_relevant = false",
    ));
    assert!(matches!(r, Err(MapError::EmptyRawRange { .. })));
}

// --- the map and the policy agree ------------------------------------------------

fn port() -> PortId {
    PortId::parse("rs485-1").unwrap()
}

fn buses() -> BusFacts {
    BusFacts::new([BusFact {
        port: port(),
        bench_port: 0,
        mastership: Mastership::BenchOnly,
        trust: Trust::OperatorAttested,
    }])
    .unwrap()
}

fn write_request(name: &str, value: WriteValue) -> EffectRequest {
    EffectRequest::BusWrite(BusWrite {
        port: port(),
        unit: 1,
        signal: SignalId::parse(name).unwrap(),
        value,
    })
}

fn value_of(t: DataType, raw: i128) -> WriteValue {
    match t {
        DataType::U16 => WriteValue::U16(raw as u16),
        DataType::I16 => WriteValue::I16(raw as i16),
        DataType::U32 => WriteValue::U32(raw as u32),
        DataType::I32 => WriteValue::I32(raw as i32),
        _ => unreachable!("not writable"),
    }
}

#[test]
fn the_map_and_the_policy_write_map_agree_on_every_writable_register() {
    let m = example();
    let entries = m.write_entries(&port(), 1, Trust::OperatorAttested);
    let writable: Vec<_> = m
        .registers()
        .iter()
        .filter(|r| r.write().is_some())
        .collect();
    assert_eq!(entries.len(), writable.len());
    assert_eq!(entries.len(), 4);
    let writes = m.write_map(&port(), 1, Trust::OperatorAttested).unwrap();
    let buses = buses();

    for r in writable {
        let w = r.write().unwrap();
        let e = writes
            .get(&port(), 1, &SignalId::parse(r.name()).unwrap())
            .expect("an entry per writable register");
        // The facts the entry carries are the map's.
        assert_eq!(e.register, r.address());
        assert_eq!(e.width, r.width());
        assert_eq!(e.class, w.class);
        assert_eq!(e.kind, w.kind);
        assert_eq!(e.energy_relevant, w.energy_relevant);
        assert_eq!(e.trust, Trust::OperatorAttested);
        let range = e.range.expect("every entry from a map has a range");
        let t = r.codec().data_type();
        assert_eq!(range.min(), value_of(t, w.raw_min));
        assert_eq!(range.max(), value_of(t, w.raw_max));

        // The engine's own answer is the class the map states.
        for raw in [w.raw_min, w.raw_max] {
            let d = classify(&write_request(r.name(), value_of(t, raw)), &writes, &buses);
            assert!(!d.is_refused(), "{} {raw}: {:?}", r.name(), d.requirement());
            assert_eq!(d.class(), w.class, "{}", r.name());
        }
        // Inside the map's valid range in engineering units is inside the
        // raw range, and one step outside is refused.
        let (lo, hi) = (t.min_raw(), t.max_raw());
        if w.raw_min > lo {
            let d = classify(
                &write_request(r.name(), value_of(t, w.raw_min - 1)),
                &writes,
                &buses,
            );
            assert_eq!(
                d.requirement(),
                &Requirement::Refused {
                    reason: RefusalReason::ValueOutOfRange
                }
            );
        }
        if w.raw_max < hi {
            let d = classify(
                &write_request(r.name(), value_of(t, w.raw_max + 1)),
                &writes,
                &buses,
            );
            assert!(d.is_refused());
        }
    }
}

#[test]
fn the_raw_range_is_exactly_the_engineering_range() {
    let m = example();
    // 40.0 to 52.0 V at 0.1 V a step.
    let w = m.register("battery_low_cutoff").unwrap().write().unwrap();
    assert_eq!((w.raw_min, w.raw_max), (400, 520));
    assert_eq!(w.class, HazardClass::Class3);
    assert_eq!(w.kind, SettingKind::BatteryProtectionLimit);
    // 48.0 to 58.4.
    let w = m.register("charge_voltage_limit").unwrap().write().unwrap();
    assert_eq!((w.raw_min, w.raw_max), (480, 584));
    // Every raw value in the range decodes inside it; the next ones do not.
    let c = m.register("battery_low_cutoff").unwrap().codec();
    for raw in 380u16..=540 {
        let ok = c.decode(&[raw]).is_ok();
        assert_eq!(ok, (400..=520).contains(&raw), "{raw}");
    }
}

#[test]
fn a_negative_scale_still_gives_the_right_raw_range() {
    let m = load(&reg(
        "x",
        0,
        "holding",
        1,
        "i16",
        "access = \"read_write\"\nscale = \"-0.5\"\noffset = \"10\"\nmin = \"0\"\nmax = \"20\"\nclass = \"class2\"\nkind = \"config\"\nenergy_relevant = false",
    ))
    .unwrap();
    // value = raw * -0.5 + 10: 0 at raw 20, 20 at raw -20.
    let w = m.register("x").unwrap().write().unwrap();
    assert_eq!((w.raw_min, w.raw_max), (-20, 20));
}

#[test]
fn a_map_writable_register_is_refused_by_the_policy_at_a_lower_trust_tier() {
    // The map states the facts; the tier is the caller's, and a documented
    // tier is not acted on.
    let m = example();
    let writes = m.write_map(&port(), 1, Trust::Documented).unwrap();
    let d = classify(
        &write_request("display_brightness", WriteValue::U16(50)),
        &writes,
        &buses(),
    );
    assert!(d.is_refused());
}

#[test]
fn neighbours_of_a_two_register_value_both_resolve_to_it() {
    let m = example();
    let sp = m.register("output_power_setpoint").unwrap();
    for a in [sp.address(), sp.address() + 1] {
        assert_eq!(m.register_at(Table::Holding, a).unwrap().name(), sp.name());
    }
    // One word either side is another register or nothing, never the same.
    for a in [sp.address() - 1, sp.address() + 2] {
        if let Some(r) = m.register_at(Table::Holding, a) {
            assert_ne!(r.name(), sp.name());
        }
    }
}
