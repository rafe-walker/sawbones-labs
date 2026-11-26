//! Reconnect delays: exponential, capped, reset on success.

use std::time::Duration;

/// Exponential backoff between reconnect attempts.
///
/// The first delay is `base`; each later one doubles, up to `max`. Call
/// [`Backoff::reset`] once a connection is established.
#[derive(Debug, Clone)]
pub struct Backoff {
    base: Duration,
    max: Duration,
    next: Duration,
}

impl Backoff {
    /// A backoff from `base` up to `max`. A zero `base` is raised to one
    /// millisecond so a dead broker can never cause a busy loop, and `max`
    /// is raised to at least `base`.
    pub fn new(base: Duration, max: Duration) -> Self {
        let base = base.max(Duration::from_millis(1));
        let max = max.max(base);
        Self {
            base,
            max,
            next: base,
        }
    }

    /// The delay to wait now. Each call lengthens the next one.
    pub fn next_delay(&mut self) -> Duration {
        let d = self.next;
        self.next = self.next.saturating_mul(2).min(self.max);
        d
    }

    /// Goes back to the base delay. Call after a successful connection.
    pub fn reset(&mut self) {
        self.next = self.base;
    }
}

impl Default for Backoff {
    /// One second, doubling to a ceiling of one minute.
    fn default() -> Self {
        Self::new(Duration::from_secs(1), Duration::from_secs(60))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_to_the_cap_and_resets() {
        let mut b = Backoff::new(Duration::from_secs(1), Duration::from_secs(10));
        let seq: Vec<u64> = (0..6).map(|_| b.next_delay().as_secs()).collect();
        assert_eq!(seq, [1, 2, 4, 8, 10, 10]);
        b.reset();
        assert_eq!(b.next_delay(), Duration::from_secs(1));
    }

    #[test]
    fn never_yields_a_zero_delay() {
        let mut b = Backoff::new(Duration::ZERO, Duration::ZERO);
        assert!(b.next_delay() > Duration::ZERO);
    }

    #[test]
    fn huge_values_do_not_overflow() {
        let mut b = Backoff::new(Duration::MAX, Duration::MAX);
        assert_eq!(b.next_delay(), Duration::MAX);
        assert_eq!(b.next_delay(), Duration::MAX);
    }
}
