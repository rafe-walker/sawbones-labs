//! Approval message decoding: any bytes decode or give a typed error, and a
//! message that decodes re-encodes to the same bytes.
#![no_main]

use libfuzzer_sys::fuzz_target;
use sawbones_approvals::Message;

fuzz_target!(|data: &[u8]| {
    if let Ok(message) = Message::decode(data) {
        assert_eq!(message.encode().as_slice(), data, "message round trip");
    }
});
