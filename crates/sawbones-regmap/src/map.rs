//! The strict TOML register map, its decoder front end and the policy
//! write-map entries it produces.

use std::collections::BTreeMap;

use sawbones_policy::{
    class_rank, FactError, HazardClass, PortId, SettingKind, SignalId, ValueRange, WriteEntry,
    WriteMap, WriteValue,
};
use sawbones_reading::{
    DecimalValue, Ident, Observation, Provenance, TimestampQuality, Trust, Unit, UnitBasis,
    UtcNanos,
};
use serde::Deserialize;
use thiserror::Error;

use crate::decimal::Decimal;
use crate::decode::{ByteOrder, Codec, CodecError, DataType, DecodeError, WordOrder};

/// The schema label a map must carry.
pub const MAP_SCHEMA: &str = "sawbones.regmap/0-provisional";
/// Largest map file accepted, in bytes. A map is untrusted input.
pub const MAX_MAP_BYTES: usize = 256 * 1024;
/// Most registers a map may describe.
pub const MAX_REGISTERS: usize = 2048;

/// Which Modbus table a register is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Table {
    /// Holding registers: readable, and writable if the map says so.
    Holding,
    /// Input registers: read only.
    Input,
}

/// What the bench may do with a register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    /// Read only.
    Read,
    /// Write only.
    Write,
    /// Read and write.
    ReadWrite,
}

impl Access {
    /// Whether the register can be read.
    pub const fn can_read(self) -> bool {
        matches!(self, Self::Read | Self::ReadWrite)
    }

    /// Whether the register can be written.
    pub const fn can_write(self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMap {
    schema: String,
    device: RawDevice,
    register: Vec<RawRegister>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDevice {
    id: String,
    address_base: u8,
    #[serde(default)]
    input_aliases_holding: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRegister {
    name: String,
    address: u32,
    table: Table,
    width: u8,
    #[serde(rename = "type")]
    data_type: DataType,
    access: Access,
    scale: Option<String>,
    offset: Option<String>,
    unit: Option<String>,
    byte_order: Option<ByteOrder>,
    word_order: Option<WordOrder>,
    min: Option<String>,
    max: Option<String>,
    class: Option<HazardClass>,
    kind: Option<SettingKind>,
    energy_relevant: Option<bool>,
}

/// Why a map was refused. Every variant is a rule; a refused map yields
/// nothing.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MapError {
    /// The file is larger than [`MAX_MAP_BYTES`].
    #[error("map is larger than {MAX_MAP_BYTES} bytes")]
    TooLarge,
    /// Not valid TOML, or a missing, unknown or wrongly typed key.
    #[error("map does not parse: {0}")]
    Parse(String),
    /// The `schema` key is not [`MAP_SCHEMA`].
    #[error("unsupported schema {0:?}")]
    UnsupportedSchema(String),
    /// The device id is not a valid identifier.
    #[error("device id is not a valid identifier")]
    BadDeviceId,
    /// `address_base` is not 0 or 1.
    #[error("address_base must be 0 or 1, got {0}")]
    BadAddressBase(u8),
    /// The map has no registers.
    #[error("map has no registers")]
    NoRegisters,
    /// The map has more than [`MAX_REGISTERS`] registers.
    #[error("map has more than {MAX_REGISTERS} registers")]
    TooManyRegisters,
    /// A register name is not 1 to 64 bytes of `a-z 0-9 _ -` starting with a
    /// letter (the policy engine's signal name rule).
    #[error("register name {name:?} is not a valid signal name")]
    BadName {
        /// The name given.
        name: String,
    },
    /// Two registers have the same name (an aliased name).
    #[error("two registers are named {name}")]
    DuplicateName {
        /// The name.
        name: String,
    },
    /// The address is below the device's address base or above 65535.
    #[error("register {name} has address {address} outside the device's address range")]
    BadAddress {
        /// The register.
        name: String,
        /// The address as written.
        address: u32,
    },
    /// The register would run past the last register address, 65535.
    #[error("register {name} runs past the last register address")]
    SpanPastEnd {
        /// The register.
        name: String,
    },
    /// The width is not exactly the number of registers the type occupies.
    #[error("register {name} has width {width} but type {data_type:?} occupies {words}")]
    WidthMismatch {
        /// The register.
        name: String,
        /// The width given.
        width: u8,
        /// The type.
        data_type: DataType,
        /// Registers the type occupies.
        words: u8,
    },
    /// A scale, offset, minimum or maximum is not a plain decimal.
    #[error("register {name}: {field} is not a plain decimal number")]
    BadNumber {
        /// The register.
        name: String,
        /// The key.
        field: &'static str,
    },
    /// The unit is not a valid unit symbol.
    #[error("register {name}: unit is not a valid unit symbol")]
    BadUnit {
        /// The register.
        name: String,
    },
    /// The scale, range or arithmetic of the register is unsound.
    #[error("register {name}: {error}")]
    Codec {
        /// The register.
        name: String,
        /// The problem.
        error: CodecError,
    },
    /// `word_order` set to little on a one-register value, where it means
    /// nothing.
    #[error("register {name}: word_order is meaningless on a one-register value")]
    WordOrderOnSingle {
        /// The register.
        name: String,
    },
    /// Two registers share an address, wholly or in part. This covers an
    /// aliased register, a two-register value running into its neighbour and,
    /// when the device says input registers alias holding registers, a
    /// register in one table landing on the other.
    #[error("registers {first} and {second} overlap")]
    Overlap {
        /// The register with the lower start.
        first: String,
        /// The other.
        second: String,
    },
    /// A writable register is in the input table, which is read only.
    #[error("register {name} is writable but is in the input table")]
    WritableInputTable {
        /// The register.
        name: String,
    },
    /// A writable register is wider than the two registers the policy engine
    /// can write.
    #[error("register {name} is writable but is wider than two registers")]
    WritableTooWide {
        /// The register.
        name: String,
    },
    /// A writable register does not state `class`, `kind` or
    /// `energy_relevant`.
    #[error("writable register {name} does not state {field}")]
    MissingWriteField {
        /// The register.
        name: String,
        /// The key that is missing.
        field: &'static str,
    },
    /// A writable register has no `min` and `max`: an unlimited write is not
    /// allowed.
    #[error("writable register {name} must state both min and max")]
    WritableNeedsRange {
        /// The register.
        name: String,
    },
    /// The stated class is below what the policy engine would assign: at
    /// least class 2 for any write, the setting kind's minimum, and class 2
    /// if energy-relevant. The map would otherwise say less than the policy
    /// does.
    #[error("writable register {name} states a class below the policy's floor for it")]
    ClassBelowFloor {
        /// The register.
        name: String,
    },
    /// `class`, `kind` or `energy_relevant` on a register that cannot be
    /// written.
    #[error("register {name} is read only but states {field}")]
    WriteFieldOnReadOnly {
        /// The register.
        name: String,
        /// The key.
        field: &'static str,
    },
    /// No raw value of the type lies inside the valid range.
    #[error("writable register {name}: no raw value of its type is inside its range")]
    EmptyRawRange {
        /// The register.
        name: String,
    },
}

/// What a writable register states for the policy engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteInfo {
    /// The hazard class of writing it.
    pub class: HazardClass,
    /// What kind of setting it is.
    pub kind: SettingKind,
    /// Whether writing it can change an energy path.
    pub energy_relevant: bool,
    /// Smallest raw value inside the valid range, as written on the wire.
    pub raw_min: i128,
    /// Largest raw value inside the valid range.
    pub raw_max: i128,
}

/// One validated register.
#[derive(Debug, Clone)]
pub struct Register {
    name: String,
    quantity: Ident,
    address: u16,
    table: Table,
    width: u8,
    access: Access,
    unit: Option<Unit>,
    codec: Codec,
    write: Option<WriteInfo>,
}

impl Register {
    /// The register's name; also the policy signal name and the quantity of
    /// its observations.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The first register address as on the wire (zero-based).
    pub const fn address(&self) -> u16 {
        self.address
    }

    /// The table.
    pub const fn table(&self) -> Table {
        self.table
    }

    /// How many 16-bit registers it occupies.
    pub const fn width(&self) -> u8 {
        self.width
    }

    /// Read or write access.
    pub const fn access(&self) -> Access {
        self.access
    }

    /// The unit symbol, if the map states one.
    pub fn unit(&self) -> Option<&Unit> {
        self.unit.as_ref()
    }

    /// How its words become a number.
    pub const fn codec(&self) -> &Codec {
        &self.codec
    }

    /// What the policy engine needs to know about writing it, if it can be
    /// written.
    pub const fn write(&self) -> Option<&WriteInfo> {
        self.write.as_ref()
    }

    /// One past the last register address it occupies.
    fn end(&self) -> u32 {
        u32::from(self.address) + u32::from(self.width)
    }
}

/// Why a read or an observation was refused.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReadError {
    /// The map has no register of that name.
    #[error("no register {0:?} in the map")]
    UnknownRegister(String),
    /// The register is write only.
    #[error("register {0:?} is not readable")]
    NotReadable(String),
    /// The words were refused.
    #[error("{0}")]
    Decode(DecodeError),
    /// The register name or value could not form an observation (cannot
    /// happen for a loaded map).
    #[error("observation could not be built")]
    Observation,
}

/// What the caller knows about a read and must supply for an
/// [`Observation`]: the decoder knows the value, the unit and the quantity,
/// not who read it or when.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationContext {
    /// Which source produced the words.
    pub source_id: Ident,
    /// When the host received them.
    pub received_at: UtcNanos,
    /// When the source says it took them, if it says.
    pub source_timestamp: Option<UtcNanos>,
    /// How far `received_at` can be trusted.
    pub timestamp_quality: TimestampQuality,
    /// Where the words came from, with the raw words as the payload.
    pub provenance: Provenance,
}

/// A loaded, validated register map.
#[derive(Debug, Clone)]
pub struct RegisterMap {
    device_id: Ident,
    input_aliases_holding: bool,
    registers: Vec<Register>,
    by_name: BTreeMap<String, usize>,
}

impl RegisterMap {
    /// Loads a map from TOML text, refusing anything the rules do not allow.
    /// All rules are checked before anything is returned.
    pub fn from_toml(text: &str) -> Result<Self, MapError> {
        if text.len() > MAX_MAP_BYTES {
            return Err(MapError::TooLarge);
        }
        let raw: RawMap = toml::from_str(text).map_err(|e| MapError::Parse(e.to_string()))?;
        if raw.schema != MAP_SCHEMA {
            return Err(MapError::UnsupportedSchema(raw.schema));
        }
        let device_id = Ident::new(&raw.device.id).map_err(|_| MapError::BadDeviceId)?;
        let base = match raw.device.address_base {
            b @ (0 | 1) => u32::from(b),
            other => return Err(MapError::BadAddressBase(other)),
        };
        if raw.register.is_empty() {
            return Err(MapError::NoRegisters);
        }
        if raw.register.len() > MAX_REGISTERS {
            return Err(MapError::TooManyRegisters);
        }
        let aliased = raw.device.input_aliases_holding;
        let mut registers = Vec::with_capacity(raw.register.len());
        let mut by_name = BTreeMap::new();
        for r in raw.register {
            let reg = validate_register(r, base)?;
            if by_name.insert(reg.name.clone(), registers.len()).is_some() {
                return Err(MapError::DuplicateName { name: reg.name });
            }
            registers.push(reg);
        }
        check_overlaps(&registers, aliased)?;
        Ok(Self {
            device_id,
            input_aliases_holding: aliased,
            registers,
            by_name,
        })
    }

    /// The device id.
    pub fn device_id(&self) -> &Ident {
        &self.device_id
    }

    /// Every register, in the order the file gives them.
    pub fn registers(&self) -> &[Register] {
        &self.registers
    }

    /// The register of that name.
    pub fn register(&self, name: &str) -> Option<&Register> {
        self.by_name.get(name).map(|&i| &self.registers[i])
    }

    /// The register that occupies a wire address (any of its words) in a
    /// table. With `input_aliases_holding`, the two tables are one space.
    pub fn register_at(&self, table: Table, address: u16) -> Option<&Register> {
        let a = u32::from(address);
        self.registers
            .iter()
            .find(|r| self.same_space(r.table, table) && u32::from(r.address) <= a && a < r.end())
    }

    fn same_space(&self, a: Table, b: Table) -> bool {
        a == b || self.input_aliases_holding
    }

    /// Words to an exact value, checked against the register's valid range.
    pub fn decode(&self, name: &str, words: &[u16]) -> Result<Decimal, ReadError> {
        let reg = self.readable(name)?;
        reg.codec.decode(words).map_err(ReadError::Decode)
    }

    fn readable(&self, name: &str) -> Result<&Register, ReadError> {
        let reg = self
            .register(name)
            .ok_or_else(|| ReadError::UnknownRegister(name.to_owned()))?;
        if !reg.access.can_read() {
            return Err(ReadError::NotReadable(name.to_owned()));
        }
        Ok(reg)
    }

    /// Words to an [`Observation`]: exact decimal text, the map's unit, and
    /// the context the caller supplies. It carries no trust tier; the host
    /// decides that.
    pub fn observe(
        &self,
        name: &str,
        words: &[u16],
        ctx: &ObservationContext,
    ) -> Result<Observation, ReadError> {
        let reg = self.readable(name)?;
        let value = reg.codec.decode(words).map_err(ReadError::Decode)?;
        let value =
            DecimalValue::parse(value.to_text().as_str()).map_err(|_| ReadError::Observation)?;
        Ok(Observation {
            source_id: ctx.source_id.clone(),
            quantity: reg.quantity.clone(),
            value,
            unit: reg.unit.clone(),
            unit_basis: if reg.unit.is_some() {
                UnitBasis::DeclaredBySource
            } else {
                UnitBasis::Unknown
            },
            received_at: ctx.received_at,
            source_timestamp: ctx.source_timestamp,
            timestamp_quality: ctx.timestamp_quality,
            provenance: ctx.provenance.clone(),
        })
    }

    /// Decodes every readable register that lies wholly inside a block of
    /// words read from `start` (a wire address) of a table. One result per
    /// register, in address order; a register that fails is reported, and
    /// does not stop the others.
    pub fn decode_block(
        &self,
        table: Table,
        start: u16,
        words: &[u16],
    ) -> Vec<(&Register, Result<Decimal, ReadError>)> {
        let lo = u32::from(start);
        let hi = lo + words.len() as u32;
        let mut hits: Vec<&Register> = self
            .registers
            .iter()
            .filter(|r| {
                r.access.can_read()
                    && self.same_space(r.table, table)
                    && u32::from(r.address) >= lo
                    && r.end() <= hi
            })
            .collect();
        hits.sort_by_key(|r| r.address);
        hits.into_iter()
            .map(|r| {
                let from = usize::from(r.address - start);
                let slice = &words[from..from + usize::from(r.width)];
                (r, r.codec.decode(slice).map_err(ReadError::Decode))
            })
            .collect()
    }

    /// The policy engine's write-map entries for this map on one port and
    /// unit, at the trust tier the caller vouches for. One per writable
    /// register, each with its address, width, class, kind, energy flag and
    /// raw value range taken from the map, so the two cannot drift apart.
    pub fn write_entries(&self, port: &PortId, unit: u8, trust: Trust) -> Vec<WriteEntry> {
        self.registers
            .iter()
            .filter_map(|r| {
                let w = r.write.as_ref()?;
                let signal = SignalId::parse(&r.name).ok()?;
                let min = write_value(r.codec.data_type(), w.raw_min)?;
                let max = write_value(r.codec.data_type(), w.raw_max)?;
                let range = ValueRange::new(min, max).ok()?;
                Some(WriteEntry {
                    port: port.clone(),
                    unit,
                    signal,
                    register: r.address,
                    width: r.width,
                    class: w.class,
                    kind: w.kind,
                    energy_relevant: w.energy_relevant,
                    trust,
                    range: Some(range),
                })
            })
            .collect()
    }

    /// [`RegisterMap::write_entries`] as a policy [`WriteMap`], which checks
    /// the unit and width again.
    pub fn write_map(&self, port: &PortId, unit: u8, trust: Trust) -> Result<WriteMap, FactError> {
        WriteMap::new(self.write_entries(port, unit, trust))
    }
}

/// A raw register value as the policy's typed write value, for the types a
/// policy write can carry.
fn write_value(t: DataType, raw: i128) -> Option<WriteValue> {
    Some(match t {
        DataType::U16 => WriteValue::U16(u16::try_from(raw).ok()?),
        DataType::I16 => WriteValue::I16(i16::try_from(raw).ok()?),
        DataType::U32 => WriteValue::U32(u32::try_from(raw).ok()?),
        DataType::I32 => WriteValue::I32(i32::try_from(raw).ok()?),
        DataType::U64 | DataType::I64 => return None,
    })
}

fn decimal_field(
    name: &str,
    field: &'static str,
    text: &Option<String>,
    default: Decimal,
) -> Result<Decimal, MapError> {
    match text {
        None => Ok(default),
        Some(s) => Decimal::parse(s).map_err(|_| MapError::BadNumber {
            name: name.to_owned(),
            field,
        }),
    }
}

fn optional_decimal(
    name: &str,
    field: &'static str,
    text: &Option<String>,
) -> Result<Option<Decimal>, MapError> {
    text.as_ref()
        .map(|s| {
            Decimal::parse(s).map_err(|_| MapError::BadNumber {
                name: name.to_owned(),
                field,
            })
        })
        .transpose()
}

fn validate_register(r: RawRegister, base: u32) -> Result<Register, MapError> {
    let name = r.name.clone();
    if SignalId::parse(&name).is_err() {
        return Err(MapError::BadName { name });
    }
    let words = r.data_type.words();
    if r.width != words {
        return Err(MapError::WidthMismatch {
            name,
            width: r.width,
            data_type: r.data_type,
            words,
        });
    }
    let wire = r
        .address
        .checked_sub(base)
        .filter(|a| *a <= u32::from(u16::MAX))
        .ok_or_else(|| MapError::BadAddress {
            name: name.clone(),
            address: r.address,
        })?;
    if wire + u32::from(r.width) > u32::from(u16::MAX) + 1 {
        return Err(MapError::SpanPastEnd { name });
    }
    if r.word_order == Some(WordOrder::Little) && words == 1 {
        return Err(MapError::WordOrderOnSingle { name });
    }
    let scale = decimal_field(&name, "scale", &r.scale, Decimal::ONE)?;
    let offset = decimal_field(&name, "offset", &r.offset, Decimal::ZERO)?;
    let min = optional_decimal(&name, "min", &r.min)?;
    let max = optional_decimal(&name, "max", &r.max)?;
    let unit = match &r.unit {
        None => None,
        Some(u) => Some(Unit::new(u).map_err(|_| MapError::BadUnit { name: name.clone() })?),
    };
    let codec = Codec::new(
        r.data_type,
        r.byte_order.unwrap_or_default(),
        r.word_order.unwrap_or_default(),
        scale,
        offset,
        min,
        max,
    )
    .map_err(|error| MapError::Codec {
        name: name.clone(),
        error,
    })?;
    let write = if r.access.can_write() {
        Some(validate_write(&name, &r, &codec, min, max)?)
    } else {
        for (field, present) in [
            ("class", r.class.is_some()),
            ("kind", r.kind.is_some()),
            ("energy_relevant", r.energy_relevant.is_some()),
        ] {
            if present {
                return Err(MapError::WriteFieldOnReadOnly { name, field });
            }
        }
        None
    };
    let quantity = Ident::new(&name).map_err(|_| MapError::BadName { name: name.clone() })?;
    Ok(Register {
        name,
        quantity,
        // `wire` was checked to fit a u16 above.
        address: wire as u16,
        table: r.table,
        width: r.width,
        access: r.access,
        unit,
        codec,
        write,
    })
}

fn validate_write(
    name: &str,
    r: &RawRegister,
    codec: &Codec,
    min: Option<Decimal>,
    max: Option<Decimal>,
) -> Result<WriteInfo, MapError> {
    let owned = || name.to_owned();
    if r.table != Table::Holding {
        return Err(MapError::WritableInputTable { name: owned() });
    }
    if r.width > 2 {
        return Err(MapError::WritableTooWide { name: owned() });
    }
    let missing = |field| MapError::MissingWriteField {
        name: owned(),
        field,
    };
    let class = r.class.ok_or_else(|| missing("class"))?;
    let kind = r.kind.ok_or_else(|| missing("kind"))?;
    let energy_relevant = r
        .energy_relevant
        .ok_or_else(|| missing("energy_relevant"))?;
    // What the policy engine will assign: it raises, never lowers.
    let mut floor = class_rank(HazardClass::Class2).max(class_rank(kind.minimum_class()));
    if energy_relevant {
        floor = floor.max(class_rank(HazardClass::Class2));
    }
    if class_rank(class) < floor {
        return Err(MapError::ClassBelowFloor { name: owned() });
    }
    let (Some(min), Some(max)) = (min, max) else {
        return Err(MapError::WritableNeedsRange { name: owned() });
    };
    let overflow = |error| MapError::Codec {
        name: owned(),
        error,
    };
    let at_min = codec
        .raw_floor_ceil(min)
        .map_err(|_| overflow(CodecError::CanOverflow))?;
    let at_max = codec
        .raw_floor_ceil(max)
        .map_err(|_| overflow(CodecError::CanOverflow))?;
    // The reachable raw values inside [min, max]: round inwards. A negative
    // scale reverses the order.
    let (mut lo, mut hi) = if codec.scale().mantissa() > 0 {
        (at_min.1, at_max.0)
    } else {
        (at_max.1, at_min.0)
    };
    lo = lo.max(codec.data_type().min_raw());
    hi = hi.min(codec.data_type().max_raw());
    if lo > hi {
        return Err(MapError::EmptyRawRange { name: owned() });
    }
    Ok(WriteInfo {
        class,
        kind,
        energy_relevant,
        raw_min: lo,
        raw_max: hi,
    })
}

/// Refuses two registers that share any address of one address space.
fn check_overlaps(registers: &[Register], input_aliases_holding: bool) -> Result<(), MapError> {
    let space = |t: Table| -> u8 {
        if input_aliases_holding {
            0
        } else {
            match t {
                Table::Holding => 0,
                Table::Input => 1,
            }
        }
    };
    let mut order: Vec<&Register> = registers.iter().collect();
    order.sort_by_key(|r| (space(r.table), r.address));
    // Sorted by start, so with no overlap so far each register ends at or
    // after the one before it, and comparing neighbours is enough.
    let mut previous: Option<(u8, &Register)> = None;
    for r in order {
        let s = space(r.table);
        if let Some((ps, prev)) = previous {
            if ps == s && u32::from(r.address) < prev.end() {
                return Err(MapError::Overlap {
                    first: prev.name.clone(),
                    second: r.name.clone(),
                });
            }
        }
        previous = Some((s, r));
    }
    Ok(())
}
