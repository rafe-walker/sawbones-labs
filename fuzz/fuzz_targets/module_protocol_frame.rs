//! The module protocol frame decoders: any bytes decode or give a typed
//! error, and anything that decodes re-encodes to exactly the same bytes (one
//! encoding per message).
#![no_main]

use libfuzzer_sys::fuzz_target;
use sawbones_module_protocol::{
    decode_request, decode_response, encode_request, encode_response, frame_len, MAX_FRAME_LEN,
};

fuzz_target!(|data: &[u8]| {
    let _ = frame_len(data);
    let mut out = [0u8; MAX_FRAME_LEN];
    if let Ok(frame) = decode_request(data) {
        let n = encode_request(&frame, &mut out).expect("a decoded request re-encodes");
        assert_eq!(&out[..n], data, "request round trip");
    }
    if let Ok(frame) = decode_response(data) {
        let frame = frame.into_claim();
        let n = encode_response(&frame, &mut out).expect("a decoded response re-encodes");
        assert_eq!(&out[..n], data, "response round trip");
    }
});
