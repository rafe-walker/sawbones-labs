//! The stored fault record: every record round-trips, and any single-bit
//! corruption of a stored record reads back as no record.

use sawbones_interlock::{FaultReason, FaultRecord, Port, Rail, RAIL_B_PORTS};
use sawbones_trust_root::record::{crc32, decode, encode};

fn every_record() -> Vec<FaultRecord> {
    let mut all = vec![
        FaultRecord::Clear,
        FaultRecord::Armed,
        FaultRecord::Latched(FaultReason::EmergencyStop),
        FaultRecord::Latched(FaultReason::HeartbeatLost),
        FaultRecord::Latched(FaultReason::FirmwareJumperWhileArmed),
        FaultRecord::Latched(FaultReason::FaultStateUnknown),
    ];
    for sensed_live in [false, true] {
        all.push(FaultRecord::Latched(FaultReason::OutputMismatch {
            rail: Rail::A,
            sensed_live,
        }));
        for port in 0..RAIL_B_PORTS {
            all.push(FaultRecord::Latched(FaultReason::OutputMismatch {
                rail: Rail::B(Port(port as u8)),
                sensed_live,
            }));
        }
    }
    all
}

#[test]
fn crc32_matches_the_standard_check_value() {
    assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
}

#[test]
fn every_record_round_trips() {
    let all = every_record();
    for record in &all {
        assert_eq!(decode(&encode(*record)), Some(*record), "{record:?}");
    }
    // And no two records share an encoding.
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_ne!(encode(*a), encode(*b));
        }
    }
}

#[test]
fn any_single_bit_flip_reads_as_no_record() {
    for record in every_record() {
        let words = encode(record);
        for word in 0..words.len() {
            for bit in 0..32 {
                let mut torn = words;
                torn[word] ^= 1 << bit;
                assert_eq!(decode(&torn), None, "{record:?} word {word} bit {bit}");
            }
        }
    }
}

#[test]
fn erased_zeroed_and_torn_stores_read_as_no_record() {
    let clear = encode(FaultRecord::Clear);
    let armed = encode(FaultRecord::Armed);
    let mixed = [clear[0], clear[1], armed[2], armed[3]];
    for words in [[0; 4], [u32::MAX; 4], mixed, [clear[0], 0, 0, 0]] {
        assert_eq!(decode(&words), None, "{words:x?}");
    }
}

#[test]
fn a_port_rail_b_does_not_have_is_refused() {
    let words = encode(FaultRecord::Latched(FaultReason::OutputMismatch {
        rail: Rail::B(Port(0)),
        sensed_live: false,
    }));
    // Rewrite the port byte to one past the last port, with a valid check.
    let payload = (words[1] & 0x00ff_ffff) | ((RAIL_B_PORTS as u32) << 24);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&words[0].to_le_bytes());
    bytes.extend_from_slice(&payload.to_le_bytes());
    let forged = [words[0], payload, !payload, crc32(&bytes)];
    assert_eq!(decode(&forged), None);
}
