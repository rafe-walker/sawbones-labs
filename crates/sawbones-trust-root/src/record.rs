//! The fault record as it is kept in non-volatile memory, with a check value.
//!
//! Four 32-bit words: a magic word that also names the layout version, the
//! payload, the payload inverted, and a CRC-32 over the first two. A torn,
//! erased, zeroed or otherwise corrupt record fails one of these checks and
//! decodes as `None`, which the interlock treats as "fault state unknown".
//! Only an intact [`FaultRecord::Clear`] lets a restart come up unfaulted, so
//! every kind of corruption errs on the safe side.

use sawbones_interlock::{FaultReason, FaultRecord, Port, Rail, RAIL_B_PORTS};

/// How many 32-bit words a stored record takes.
pub const RECORD_WORDS: usize = 4;

/// A fault record as stored: see the module docs for the layout.
pub type RecordWords = [u32; RECORD_WORDS];

/// "SBF" and layout version 1. A record from another layout fails the check.
const MAGIC: u32 = 0x5342_4601;

const TAG_CLEAR: u32 = 1;
const TAG_ARMED: u32 = 2;
const TAG_LATCHED: u32 = 3;

const REASON_EMERGENCY_STOP: u32 = 1;
const REASON_HEARTBEAT_LOST: u32 = 2;
const REASON_OUTPUT_MISMATCH: u32 = 3;
const REASON_JUMPER_WHILE_ARMED: u32 = 4;
const REASON_FAULT_STATE_UNKNOWN: u32 = 5;

const RAIL_NONE: u32 = 0;
const RAIL_A: u32 = 1;
const RAIL_B: u32 = 2;

const SENSED_LIVE: u32 = 0x80;

/// Encodes a record for storage.
#[must_use]
pub const fn encode(record: FaultRecord) -> RecordWords {
    let payload = payload(record);
    [MAGIC, payload, !payload, check(MAGIC, payload)]
}

/// Decodes a stored record. Returns `None` unless every check passes and the
/// payload names a record this build knows.
#[must_use]
pub fn decode(words: &RecordWords) -> Option<FaultRecord> {
    let [magic, payload, inverted, crc] = *words;
    if magic != MAGIC || inverted != !payload || crc != check(magic, payload) {
        return None;
    }
    let record = from_payload(payload)?;
    // Only the one canonical encoding is accepted.
    if self::payload(record) != payload {
        return None;
    }
    Some(record)
}

/// Payload bytes, least significant first: tag, reason, rail, then the port
/// with bit 7 set when the rail was sensed live.
const fn payload(record: FaultRecord) -> u32 {
    let (tag, reason, rail, detail) = match record {
        FaultRecord::Clear => (TAG_CLEAR, 0, RAIL_NONE, 0),
        FaultRecord::Armed => (TAG_ARMED, 0, RAIL_NONE, 0),
        FaultRecord::Latched(reason) => match reason {
            FaultReason::EmergencyStop => (TAG_LATCHED, REASON_EMERGENCY_STOP, RAIL_NONE, 0),
            FaultReason::HeartbeatLost => (TAG_LATCHED, REASON_HEARTBEAT_LOST, RAIL_NONE, 0),
            FaultReason::FirmwareJumperWhileArmed => {
                (TAG_LATCHED, REASON_JUMPER_WHILE_ARMED, RAIL_NONE, 0)
            }
            FaultReason::FaultStateUnknown => {
                (TAG_LATCHED, REASON_FAULT_STATE_UNKNOWN, RAIL_NONE, 0)
            }
            FaultReason::OutputMismatch { rail, sensed_live } => {
                let live = if sensed_live { SENSED_LIVE } else { 0 };
                match rail {
                    Rail::A => (TAG_LATCHED, REASON_OUTPUT_MISMATCH, RAIL_A, live),
                    Rail::B(Port(port)) => (
                        TAG_LATCHED,
                        REASON_OUTPUT_MISMATCH,
                        RAIL_B,
                        live | port as u32,
                    ),
                }
            }
        },
    };
    tag | (reason << 8) | (rail << 16) | (detail << 24)
}

fn from_payload(payload: u32) -> Option<FaultRecord> {
    let [tag, reason, rail, detail] = payload.to_le_bytes();
    match u32::from(tag) {
        TAG_CLEAR => Some(FaultRecord::Clear),
        TAG_ARMED => Some(FaultRecord::Armed),
        TAG_LATCHED => {
            let reason = match u32::from(reason) {
                REASON_EMERGENCY_STOP => FaultReason::EmergencyStop,
                REASON_HEARTBEAT_LOST => FaultReason::HeartbeatLost,
                REASON_JUMPER_WHILE_ARMED => FaultReason::FirmwareJumperWhileArmed,
                REASON_FAULT_STATE_UNKNOWN => FaultReason::FaultStateUnknown,
                REASON_OUTPUT_MISMATCH => {
                    let sensed_live = u32::from(detail) & SENSED_LIVE != 0;
                    let port = detail & 0x7f;
                    let rail = match u32::from(rail) {
                        RAIL_A => Rail::A,
                        RAIL_B if usize::from(port) < RAIL_B_PORTS => Rail::B(Port(port)),
                        _ => return None,
                    };
                    FaultReason::OutputMismatch { rail, sensed_live }
                }
                _ => return None,
            };
            Some(FaultRecord::Latched(reason))
        }
        _ => None,
    }
}

/// CRC-32 (IEEE 802.3, reflected) over the little-endian bytes of both words.
const fn check(magic: u32, payload: u32) -> u32 {
    let a = magic.to_le_bytes();
    let b = payload.to_le_bytes();
    let bytes = [a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]];
    crc32(&bytes)
}

/// CRC-32 (IEEE 802.3, reflected, polynomial 0xEDB88320), bit by bit. Small
/// and slow, which is fine for eight bytes per tick at most.
#[must_use]
pub const fn crc32(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xffff_ffff;
    let mut i = 0;
    while i < bytes.len() {
        crc ^= bytes[i] as u32;
        let mut bit = 0;
        while bit < 8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
            bit += 1;
        }
        i += 1;
    }
    !crc
}
