//! The raw-capture line format: one received MQTT message per JSON line.
//!
//! `sa-tap record --raw-log` writes this format, and the synthetic test
//! fixture is in it too. Keeping the messages exactly as received means a
//! capture from a real unit can be run through the parser again whenever the
//! parser changes.
//!
//! Lines that are blank or start with `#` are comments.

use std::io::{self, BufRead, Read};

use sawbones_reading::UtcNanos;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::parser::Message;

/// Most payload bytes kept per message in a raw capture.
pub const MAX_RAW_PAYLOAD_BYTES: usize = 64 * 1024;

/// One MQTT message as received.
///
/// Both strings are outside text. They are plain `String`s here because this
/// type is a file format, not something to show a reader; anything that
/// leaves this crate goes through the parser, which wraps them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawMessage {
    /// When this host received the message.
    pub received_at: UtcNanos,
    /// The topic.
    pub topic: String,
    /// The payload as text. Invalid UTF-8 was replaced on capture, and
    /// payloads longer than [`MAX_RAW_PAYLOAD_BYTES`] were cut.
    pub payload: String,
    /// Whether the broker flagged the message as retained.
    #[serde(default)]
    pub retained: bool,
}

impl RawMessage {
    /// Captures a message, replacing invalid UTF-8 and cutting an overlong
    /// payload.
    pub fn capture(topic: &str, payload: &[u8], received_at: UtcNanos, retained: bool) -> Self {
        let head = &payload[..payload.len().min(MAX_RAW_PAYLOAD_BYTES)];
        Self {
            received_at,
            topic: topic.to_owned(),
            payload: String::from_utf8_lossy(head).into_owned(),
            retained,
        }
    }

    /// Borrows this capture as a parser [`Message`].
    pub fn as_message(&self) -> Message<'_> {
        Message {
            topic: &self.topic,
            payload: self.payload.as_bytes(),
            received_at: self.received_at,
            retained: self.retained,
        }
    }
}

/// A line of a raw capture that could not be read.
#[derive(Debug, Error)]
#[error("line is not a raw-capture record: {0}")]
pub struct RawLineError(#[from] serde_json::Error);

/// Returns the content of a JSONL line, or `None` for a blank line or a
/// `#` comment.
pub fn jsonl_content(line: &str) -> Option<&str> {
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') {
        None
    } else {
        Some(t)
    }
}

/// Parses one line of a raw capture. `None` means blank or comment.
pub fn parse_raw_line(line: &str) -> Option<Result<RawMessage, RawLineError>> {
    jsonl_content(line).map(|t| serde_json::from_str(t).map_err(RawLineError))
}

/// What [`read_bounded_line`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineRead {
    /// End of input; nothing was read.
    Eof,
    /// A line is in the buffer (without its line ending).
    Line,
    /// The line was longer than the limit. It was skipped and the buffer is
    /// empty.
    TooLong,
}

/// Reads one line of at most `max_bytes` into `buf`, replacing whatever was
/// there. A longer line is skipped whole rather than read into memory, so a
/// damaged or hostile file cannot exhaust it.
pub fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    buf: &mut Vec<u8>,
    max_bytes: usize,
) -> io::Result<LineRead> {
    buf.clear();
    let limit = u64::try_from(max_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let n = reader.by_ref().take(limit).read_until(b'\n', buf)?;
    if n == 0 {
        return Ok(LineRead::Eof);
    }
    if buf.last() == Some(&b'\n') {
        buf.pop();
        if buf.last() == Some(&b'\r') {
            buf.pop();
        }
        return Ok(LineRead::Line);
    }
    if buf.len() <= max_bytes {
        // Last line of the input, with no line ending.
        return Ok(LineRead::Line);
    }
    // Too long: discard the rest of the line without storing it.
    buf.clear();
    loop {
        let chunk = reader.fill_buf()?;
        if chunk.is_empty() {
            break;
        }
        match chunk.iter().position(|b| *b == b'\n') {
            Some(i) => {
                reader.consume(i + 1);
                break;
            }
            None => {
                let len = chunk.len();
                reader.consume(len);
            }
        }
    }
    Ok(LineRead::TooLong)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_lines_are_read_and_long_ones_skipped() {
        let data = format!("short\r\n{}\nafter\nlast", "x".repeat(100));
        let mut r = io::BufReader::with_capacity(8, data.as_bytes());
        let mut buf = Vec::new();
        assert_eq!(
            read_bounded_line(&mut r, &mut buf, 10).unwrap(),
            LineRead::Line
        );
        assert_eq!(buf, b"short");
        assert_eq!(
            read_bounded_line(&mut r, &mut buf, 10).unwrap(),
            LineRead::TooLong
        );
        assert!(buf.is_empty());
        assert_eq!(
            read_bounded_line(&mut r, &mut buf, 10).unwrap(),
            LineRead::Line
        );
        assert_eq!(buf, b"after");
        assert_eq!(
            read_bounded_line(&mut r, &mut buf, 10).unwrap(),
            LineRead::Line
        );
        assert_eq!(buf, b"last");
        assert_eq!(
            read_bounded_line(&mut r, &mut buf, 10).unwrap(),
            LineRead::Eof
        );
    }

    #[test]
    fn a_line_exactly_at_the_limit_is_kept() {
        let mut r = io::BufReader::new(&b"12345\n123456\n"[..]);
        let mut buf = Vec::new();
        assert_eq!(
            read_bounded_line(&mut r, &mut buf, 5).unwrap(),
            LineRead::Line
        );
        assert_eq!(buf, b"12345");
        assert_eq!(
            read_bounded_line(&mut r, &mut buf, 5).unwrap(),
            LineRead::TooLong
        );
        assert_eq!(
            read_bounded_line(&mut r, &mut buf, 5).unwrap(),
            LineRead::Eof
        );
    }

    #[test]
    fn round_trips_one_line() {
        let m = RawMessage::capture(
            "solar_assistant/inverter_1/pv_power/state",
            b"1823",
            UtcNanos::from_unix_nanos(7),
            false,
        );
        let line = serde_json::to_string(&m).unwrap();
        assert!(!line.contains('\n'));
        assert_eq!(parse_raw_line(&line).unwrap().unwrap(), m);
    }

    #[test]
    fn comments_and_blanks_are_skipped() {
        assert!(parse_raw_line("").is_none());
        assert!(parse_raw_line("   ").is_none());
        assert!(parse_raw_line("# SYNTHETIC").is_none());
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(parse_raw_line("{").unwrap().is_err());
        assert!(parse_raw_line("[]").unwrap().is_err());
        assert!(parse_raw_line(r#"{"topic":1}"#).unwrap().is_err());
    }

    #[test]
    fn capture_bounds_the_payload() {
        let big = vec![b'x'; MAX_RAW_PAYLOAD_BYTES * 2];
        let m = RawMessage::capture("t", &big, UtcNanos::from_unix_nanos(0), true);
        assert_eq!(m.payload.len(), MAX_RAW_PAYLOAD_BYTES);
        let m = RawMessage::capture("t", &[0xff, b'1'], UtcNanos::from_unix_nanos(0), true);
        assert!(m.payload.ends_with('1'));
    }
}
