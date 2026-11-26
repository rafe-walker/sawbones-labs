//! UTC timestamps as nanoseconds since the Unix epoch.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// A UTC instant, stored as signed nanoseconds since 1970-01-01T00:00:00Z.
///
/// An `i64` of nanoseconds covers roughly the years 1677 to 2262. It
/// serialises as a plain JSON integer. Note for consumers written in
/// languages whose only number type is an IEEE double: present-day values are
/// larger than 2^53, so read the field as a 64-bit integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UtcNanos(i64);

impl UtcNanos {
    /// Builds an instant from nanoseconds since the Unix epoch.
    pub const fn from_unix_nanos(nanos: i64) -> Self {
        Self(nanos)
    }

    /// Nanoseconds since the Unix epoch.
    pub const fn as_unix_nanos(self) -> i64 {
        self.0
    }

    /// The host clock's idea of now. Saturates rather than panicking if the
    /// host clock is outside the representable range.
    pub fn now() -> Self {
        Self::from_system_time(SystemTime::now())
    }

    /// Converts a [`SystemTime`], saturating at the ends of the range.
    pub fn from_system_time(t: SystemTime) -> Self {
        match t.duration_since(UNIX_EPOCH) {
            Ok(after) => Self(i64::try_from(after.as_nanos()).unwrap_or(i64::MAX)),
            Err(before) => {
                let n = i64::try_from(before.duration().as_nanos()).unwrap_or(i64::MAX);
                Self(n.checked_neg().unwrap_or(i64::MIN))
            }
        }
    }

    /// Nanoseconds from `earlier` to `self` (negative if `self` is earlier).
    /// Saturates instead of overflowing.
    pub const fn nanos_since(self, earlier: Self) -> i64 {
        self.0.saturating_sub(earlier.0)
    }

    /// Adds a signed number of nanoseconds, saturating at the range ends.
    pub const fn saturating_add_nanos(self, nanos: i64) -> Self {
        Self(self.0.saturating_add(nanos))
    }

    /// Formats as RFC 3339 in UTC with nanosecond precision, for example
    /// `2026-11-25T18:30:00.000000000Z`.
    pub fn to_rfc3339(self) -> String {
        const NANOS_PER_SEC: i64 = 1_000_000_000;
        let secs = self.0.div_euclid(NANOS_PER_SEC);
        let nanos = self.0.rem_euclid(NANOS_PER_SEC);
        let days = secs.div_euclid(86_400);
        let sod = secs.rem_euclid(86_400);
        let (y, m, d) = civil_from_days(days);
        format!(
            "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{nanos:09}Z",
            sod / 3600,
            (sod % 3600) / 60,
            sod % 60
        )
    }
}

/// Days since the Unix epoch to a proleptic Gregorian (year, month, day).
/// This is the well-known era-based algorithm; the arithmetic is exact for
/// the whole `i64`-nanosecond range.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_formats_as_1970() {
        assert_eq!(
            UtcNanos::from_unix_nanos(0).to_rfc3339(),
            "1970-01-01T00:00:00.000000000Z"
        );
    }

    #[test]
    fn known_instants_format_correctly() {
        // 2026-11-25T18:30:00Z is 1_795_631_400 seconds after the epoch.
        let t = UtcNanos::from_unix_nanos(1_795_631_400 * 1_000_000_000 + 123);
        assert_eq!(t.to_rfc3339(), "2026-11-25T18:30:00.000000123Z");
        // A leap day.
        let leap = UtcNanos::from_unix_nanos(1_709_208_000 * 1_000_000_000);
        assert_eq!(leap.to_rfc3339(), "2024-02-29T12:00:00.000000000Z");
    }

    #[test]
    fn negative_instants_do_not_panic() {
        let t = UtcNanos::from_unix_nanos(-1);
        assert_eq!(t.to_rfc3339(), "1969-12-31T23:59:59.999999999Z");
        let _ = UtcNanos::from_unix_nanos(i64::MIN).to_rfc3339();
        let _ = UtcNanos::from_unix_nanos(i64::MAX).to_rfc3339();
    }

    #[test]
    fn differences_saturate() {
        let a = UtcNanos::from_unix_nanos(i64::MAX);
        let b = UtcNanos::from_unix_nanos(i64::MIN);
        assert_eq!(a.nanos_since(b), i64::MAX);
        assert_eq!(b.nanos_since(a), i64::MIN);
    }

    #[test]
    fn serialises_as_a_bare_integer() {
        let t = UtcNanos::from_unix_nanos(42);
        assert_eq!(serde_json::to_string(&t).unwrap(), "42");
        let back: UtcNanos = serde_json::from_str("42").unwrap();
        assert_eq!(back, t);
    }
}
