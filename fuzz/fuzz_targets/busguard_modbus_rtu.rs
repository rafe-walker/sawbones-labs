//! The bus guard's Modbus RTU request parser: no panic, and a frame that
//! parses is rebuilt byte for byte from its fields.
#![no_main]

use libfuzzer_sys::fuzz_target;
use sawbones_busguard::{crc16_modbus, parse, Function};

fuzz_target!(|data: &[u8]| {
    let Ok(frame) = parse(data) else { return };
    let mut rebuilt = vec![frame.unit(), frame.function().code()];
    rebuilt.extend_from_slice(&frame.start().to_be_bytes());
    match frame.function() {
        Function::ReadHoldingRegisters | Function::ReadInputRegisters => {
            rebuilt.extend_from_slice(&frame.quantity().to_be_bytes());
        }
        Function::WriteSingleRegister => rebuilt.extend_from_slice(frame.payload()),
        Function::WriteMultipleRegisters => {
            rebuilt.extend_from_slice(&frame.quantity().to_be_bytes());
            rebuilt.push(
                frame
                    .byte_count()
                    .expect("a multiple write has a byte count"),
            );
            rebuilt.extend_from_slice(frame.payload());
        }
    }
    assert_eq!(crc16_modbus(&rebuilt), frame.crc(), "check value");
    rebuilt.extend_from_slice(&frame.crc().to_le_bytes());
    assert_eq!(rebuilt, data, "frame round trip");
    assert!(frame.unit() != 0, "a broadcast never parses");
    assert!(
        u32::from(frame.start()) + u32::from(frame.quantity()) - 1 <= u32::from(u16::MAX),
        "range stays in the address space"
    );
});
