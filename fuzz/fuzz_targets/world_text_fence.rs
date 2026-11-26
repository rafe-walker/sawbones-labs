//! `WorldText` fencing: outside bytes, with a cap the input chooses, always
//! fence onto exactly one body line with no raw angle bracket or control
//! character, and wrapping is idempotent.
#![no_main]

use libfuzzer_sys::fuzz_target;
use sawbones_reading::{WorldText, FENCE_CLOSE, FENCE_OPEN};

fuzz_target!(|data: &[u8]| {
    // The first two bytes pick the cap (past the hard maximum sometimes).
    let Some((head, rest)) = data.split_first_chunk::<2>() else {
        return;
    };
    let cap = usize::from(u16::from_le_bytes(*head)) % (WorldText::HARD_MAX_BYTES + 512);
    let text = WorldText::from_bytes_lossy(rest, cap);

    let fenced = text.fenced();
    let lines: Vec<&str> = fenced.split('\n').collect();
    assert_eq!(lines.len(), 3, "open, one body line, close: {fenced:?}");
    assert_eq!(lines[0], FENCE_OPEN);
    assert_eq!(lines[2], FENCE_CLOSE);
    let body = lines[1];
    assert!(
        !body.contains(['<', '>']) && !body.chars().any(char::is_control),
        "raw bracket or control character in {body:?}"
    );
    assert!(text.len() <= cap.min(WorldText::HARD_MAX_BYTES), "cap");

    // Idempotent: wrapping already-capped text keeps it as it is.
    let again = WorldText::new(text.as_untrusted_str(), cap);
    assert_eq!(again.as_untrusted_str(), text.as_untrusted_str());
    assert_eq!(again.fenced().split('\n').count(), 3);
});
