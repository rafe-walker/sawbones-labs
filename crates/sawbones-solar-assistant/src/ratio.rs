//! The "twin input" ratio monitor. Pure: no I/O, no clock.
//!
//! Two PV inputs that see the same sky (two inverters fed by similar arrays,
//! or two MPPT inputs on one inverter) tend to hold a steady power ratio
//! while both are producing. This module watches that ratio and reports how
//! far it has moved from a stored baseline.

use sawbones_reading::{Ident, Reading, TimestampQuality, UtcNanos};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One power sample: a time and a number of watts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerSample {
    /// When the sample was received.
    pub at: UtcNanos,
    /// Power in watts.
    pub watts: f64,
}

/// Settings for [`twin_input_ratio`].
#[derive(Debug, Clone, PartialEq)]
pub struct RatioConfig {
    /// Both inputs must be at or above this many watts for a pair to count.
    /// Near zero the ratio is mostly noise. Must be finite and above zero.
    pub min_power_w: f64,
    /// Length of the sliding window, in nanoseconds.
    pub window_ns: u64,
    /// How far the window slides between reports, in nanoseconds.
    pub step_ns: u64,
    /// Two samples are paired only if received within this many nanoseconds
    /// of each other.
    pub max_pair_skew_ns: u64,
    /// A window reports only if it holds at least this many pairs.
    pub min_pairs: usize,
    /// The stored baseline ratio to compare against, if there is one.
    pub baseline: Option<f64>,
}

impl Default for RatioConfig {
    /// 200 W minimum, 10-minute window sliding by 1 minute, samples paired
    /// within 10 seconds, at least 10 pairs per window, no baseline. These
    /// are starting points, not tuned values.
    fn default() -> Self {
        Self {
            min_power_w: 200.0,
            window_ns: 600 * NANOS_PER_SEC,
            step_ns: 60 * NANOS_PER_SEC,
            max_pair_skew_ns: 10 * NANOS_PER_SEC,
            min_pairs: 10,
            baseline: None,
        }
    }
}

const NANOS_PER_SEC: u64 = 1_000_000_000;

/// Most windows one call will report. Guards against a tiny step over a
/// long recording.
pub const MAX_WINDOWS: usize = 100_000;

/// Why [`twin_input_ratio`] refused its settings.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum RatioError {
    /// `min_power_w` was not a finite number above zero.
    #[error("minimum power must be a finite number above zero, got {0}")]
    BadMinPower(f64),
    /// `window_ns` or `step_ns` was zero.
    #[error("window and step must both be longer than zero")]
    ZeroWindowOrStep,
    /// `min_pairs` was zero.
    #[error("minimum pairs per window must be at least 1")]
    ZeroMinPairs,
    /// The baseline was not a finite number above zero.
    #[error("baseline ratio must be a finite number above zero, got {0}")]
    BadBaseline(f64),
    /// The step is so small that the run would report more than
    /// [`MAX_WINDOWS`] windows.
    #[error("step is too small for this recording: more than {MAX_WINDOWS} windows")]
    TooManyWindows,
}

/// The ratio over one position of the sliding window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatioWindow {
    /// The end of the window. It covers the `window_ns` before this instant.
    pub window_end: UtcNanos,
    /// How many pairs the window held.
    pub pairs: usize,
    /// The median of input A's power divided by input B's.
    pub median_ratio: f64,
    /// How far the median has moved from the baseline, in percent. `None`
    /// when no baseline was given.
    pub drift_pct: Option<f64>,
}

/// Everything [`twin_input_ratio`] observed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatioReport {
    /// One entry per window position that held enough pairs.
    pub windows: Vec<RatioWindow>,
    /// Samples of A that found a partner in B close enough in time.
    pub pairs_in_time: u64,
    /// Of those, pairs where both inputs met the minimum power.
    pub pairs_used: u64,
    /// Of those, pairs dropped because an input was below the minimum.
    pub pairs_below_min_power: u64,
    /// Samples of A with no partner in B close enough in time.
    pub unpaired: u64,
    /// The median ratio over every used pair, if there were at least
    /// `min_pairs` of them.
    pub overall_median: Option<f64>,
    /// The baseline that was compared against, if any.
    pub baseline: Option<f64>,
    /// How far the overall median has moved from the baseline, in percent.
    pub overall_drift_pct: Option<f64>,
}

impl RatioReport {
    /// States the observation in plain words, in the only terms this monitor
    /// is entitled to use: the latest window first, then the whole recording.
    pub fn describe(&self) -> String {
        let Some(overall) = self.overall_median else {
            return format!(
                "Not enough paired samples above the minimum power to state a ratio ({} usable pairs).",
                self.pairs_used
            );
        };
        let moved = |drift: Option<f64>| match (drift, self.baseline) {
            (Some(d), Some(b)) => format!("ratio moved by {d:+.1}% from baseline {b:.4}"),
            _ => "no baseline stored, so no drift is stated".to_owned(),
        };
        let mut text = String::new();
        if let Some(w) = self.windows.last() {
            text.push_str(&format!(
                "Latest window ({} pairs): median ratio A/B {:.4}; {}. ",
                w.pairs,
                w.median_ratio,
                moved(w.drift_pct)
            ));
        }
        text.push_str(&format!(
            "Whole recording ({} pairs): median ratio A/B {overall:.4}; {}. This is an observation, not a diagnosis.",
            self.pairs_used,
            moved(self.overall_drift_pct)
        ));
        text
    }
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    Some(if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) / 2.0
    })
}

fn drift_pct(median: f64, baseline: Option<f64>) -> Option<f64> {
    baseline.map(|b| (median / b - 1.0) * 100.0)
}

/// Watches the power ratio between two inputs that see the same sky.
///
/// `a` and `b` are two series of power samples, for example
/// `inverter_1 pv_power` and `inverter_2 pv_power`. They need not be sorted
/// or aligned. Each sample of `a` is paired with the nearest-in-time sample
/// of `b` (each `b` sample is used once, and only if within
/// `max_pair_skew_ns`). A pair counts only when **both** inputs are at or
/// above `min_power_w`. The ratio A/B is then taken over a sliding window:
/// each window (closed at both ends) with at least `min_pairs` pairs reports
/// its **median** ratio
/// and, if a baseline is given, the drift from it in percent.
///
/// # A ratio difference is an observation, not a diagnosis
///
/// Two arrays can differ in panel count, tilt, orientation, shading, soiling,
/// string voltage, cable loss and clipping, and each of those changes
/// through the day and the year. Passing cloud moves the ratio for seconds
/// or minutes. A battery reaching full makes an inverter throttle one input
/// before the other. None of that is a fault.
///
/// So the only thing this function ever says is of the form **"the ratio
/// moved by X% from baseline"**. It never says "panel fault", "string
/// failure" or anything like it, and callers must not turn its output into
/// such a statement. What a drift *means* is a question for a person, or
/// for further measurement, with the context this function does not have.
///
/// The inputs are also only as good as their source. Readings from Solar
/// Assistant are `Unverified` third-party values, and their timestamps are
/// arrival times.
pub fn twin_input_ratio(
    a: &[PowerSample],
    b: &[PowerSample],
    config: &RatioConfig,
) -> Result<RatioReport, RatioError> {
    if !(config.min_power_w.is_finite() && config.min_power_w > 0.0) {
        return Err(RatioError::BadMinPower(config.min_power_w));
    }
    if config.window_ns == 0 || config.step_ns == 0 {
        return Err(RatioError::ZeroWindowOrStep);
    }
    if config.min_pairs == 0 {
        return Err(RatioError::ZeroMinPairs);
    }
    if let Some(base) = config.baseline {
        if !(base.is_finite() && base > 0.0) {
            return Err(RatioError::BadBaseline(base));
        }
    }

    let sorted = |s: &[PowerSample]| {
        let mut v: Vec<PowerSample> = s.iter().copied().filter(|p| p.watts.is_finite()).collect();
        v.sort_by_key(|p| p.at);
        v
    };
    let (a, b) = (sorted(a), sorted(b));
    let skew = |x: &PowerSample, y: &PowerSample| x.at.nanos_since(y.at).unsigned_abs();

    // Pair each A sample with the nearest B sample in time.
    let mut pairs: Vec<(i64, f64)> = Vec::new();
    let (mut pairs_in_time, mut below_min, mut unpaired) = (0u64, 0u64, 0u64);
    let mut j = 0usize;
    let mut last_used: Option<usize> = None;
    for sa in &a {
        if b.is_empty() {
            unpaired += 1;
            continue;
        }
        while j + 1 < b.len() && skew(&b[j + 1], sa) <= skew(&b[j], sa) {
            j += 1;
        }
        if last_used == Some(j) || skew(&b[j], sa) > config.max_pair_skew_ns {
            unpaired += 1;
            continue;
        }
        last_used = Some(j);
        pairs_in_time += 1;
        let sb = &b[j];
        if sa.watts >= config.min_power_w && sb.watts >= config.min_power_w {
            let at = sa.at.max(sb.at).as_unix_nanos();
            pairs.push((at, sa.watts / sb.watts));
        } else {
            below_min += 1;
        }
    }
    pairs.sort_by_key(|p| p.0);

    // Slide the window over the pairs.
    let mut windows = Vec::new();
    if let (Some(first), Some(last)) = (pairs.first().map(|p| p.0), pairs.last().map(|p| p.0)) {
        let window = i64::try_from(config.window_ns).unwrap_or(i64::MAX);
        let step = i64::try_from(config.step_ns).unwrap_or(i64::MAX);
        let mut end = first.saturating_add(window).min(last);
        loop {
            // Closed at both ends: [end - window, end].
            let start = end.saturating_sub(window);
            let lo = pairs.partition_point(|p| p.0 < start);
            let hi = pairs.partition_point(|p| p.0 <= end);
            if hi - lo >= config.min_pairs {
                let mut ratios: Vec<f64> = pairs[lo..hi].iter().map(|p| p.1).collect();
                if let Some(m) = median(&mut ratios) {
                    if windows.len() >= MAX_WINDOWS {
                        return Err(RatioError::TooManyWindows);
                    }
                    windows.push(RatioWindow {
                        window_end: UtcNanos::from_unix_nanos(end),
                        pairs: hi - lo,
                        median_ratio: m,
                        drift_pct: drift_pct(m, config.baseline),
                    });
                }
            }
            if end >= last {
                break;
            }
            end = end.saturating_add(step).min(last);
        }
    }

    let overall_median = if pairs.len() >= config.min_pairs {
        let mut all: Vec<f64> = pairs.iter().map(|p| p.1).collect();
        median(&mut all)
    } else {
        None
    };

    Ok(RatioReport {
        windows,
        pairs_in_time,
        pairs_used: pairs.len() as u64,
        pairs_below_min_power: below_min,
        unpaired,
        overall_median,
        baseline: config.baseline,
        overall_drift_pct: overall_median.and_then(|m| drift_pct(m, config.baseline)),
    })
}

/// Names one series of readings: a source id and a quantity.
///
/// Written as `<source id>/<quantity>`, which for Solar Assistant is the
/// state topic without its `/state` ending, for example
/// `solar_assistant/inverter_1/pv_power`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamKey {
    /// The source id, for example `solar_assistant/inverter_1`.
    pub source_id: Ident,
    /// The quantity, for example `pv_power`.
    pub quantity: Ident,
}

/// A string that is not a usable [`StreamKey`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("stream must be written as <source id>/<quantity>, for example solar_assistant/inverter_1/pv_power")]
pub struct StreamKeyError;

impl StreamKey {
    /// Parses `<source id>/<quantity>`, splitting at the last `/`.
    pub fn parse(s: &str) -> Result<Self, StreamKeyError> {
        let (source, quantity) = s.rsplit_once('/').ok_or(StreamKeyError)?;
        Ok(Self {
            source_id: Ident::new(source).map_err(|_| StreamKeyError)?,
            quantity: Ident::new(quantity).map_err(|_| StreamKeyError)?,
        })
    }

    /// Whether a reading belongs to this series.
    pub fn matches(&self, r: &Reading) -> bool {
        *r.source_id() == self.source_id && *r.quantity() == self.quantity
    }

    /// The key in its written form.
    pub fn to_key_string(&self) -> String {
        format!("{}/{}", self.source_id, self.quantity)
    }
}

/// Picks the two series out of a stream of readings.
#[derive(Debug, Clone)]
pub struct SampleCollector {
    a_key: StreamKey,
    b_key: StreamKey,
    /// Samples for input A.
    pub a: Vec<PowerSample>,
    /// Samples for input B.
    pub b: Vec<PowerSample>,
    /// Matching readings left out because they were retained messages, whose
    /// age is unknown.
    pub skipped_retained: u64,
    /// Matching readings left out because their unit was not `W`.
    pub skipped_not_watts: u64,
}

impl SampleCollector {
    /// A collector for the two named series.
    pub fn new(a_key: StreamKey, b_key: StreamKey) -> Self {
        Self {
            a_key,
            b_key,
            a: Vec::new(),
            b: Vec::new(),
            skipped_retained: 0,
            skipped_not_watts: 0,
        }
    }

    /// Offers one reading. It is kept if it belongs to either series, is
    /// in watts and is not a retained replay.
    pub fn push(&mut self, r: &Reading) {
        let target = if self.a_key.matches(r) {
            &mut self.a
        } else if self.b_key.matches(r) {
            &mut self.b
        } else {
            return;
        };
        if r.timestamp_quality() == TimestampQuality::HostReceiveRetained {
            self.skipped_retained += 1;
            return;
        }
        if r.unit().map(|u| u.as_str()) != Some("W") {
            self.skipped_not_watts += 1;
            return;
        }
        target.push(PowerSample {
            at: r.received_at(),
            watts: r.value().as_f64(),
        });
    }
}

/// Schema label for a stored [`RatioBaseline`].
pub const BASELINE_SCHEMA: &str = "sawbones.ratio-baseline/0-provisional";

/// A stored baseline ratio for one pair of inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatioBaseline {
    /// Schema label; [`BASELINE_SCHEMA`].
    pub schema: String,
    /// Input A.
    pub a: StreamKey,
    /// Input B.
    pub b: StreamKey,
    /// The baseline median ratio A/B.
    pub ratio: f64,
    /// How many pairs the baseline was taken over.
    pub pairs: u64,
    /// The minimum power that applied when it was taken.
    pub min_power_w: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: i64 = 1_000_000_000;

    fn series(start_s: i64, every_s: i64, watts: &[f64]) -> Vec<PowerSample> {
        watts
            .iter()
            .enumerate()
            .map(|(i, w)| PowerSample {
                at: UtcNanos::from_unix_nanos((start_s + every_s * i as i64) * S),
                watts: *w,
            })
            .collect()
    }

    fn cfg() -> RatioConfig {
        RatioConfig {
            min_power_w: 100.0,
            window_ns: 60 * NANOS_PER_SEC,
            step_ns: 30 * NANOS_PER_SEC,
            max_pair_skew_ns: 3 * NANOS_PER_SEC,
            min_pairs: 3,
            baseline: None,
        }
    }

    #[test]
    fn a_steady_ratio_is_reported_as_steady() {
        let a = series(0, 5, &[1000.0; 40]);
        let b = series(1, 5, &[800.0; 40]);
        let mut c = cfg();
        c.baseline = Some(1.25);
        let r = twin_input_ratio(&a, &b, &c).unwrap();
        assert_eq!(r.pairs_used, 40);
        assert_eq!(r.unpaired, 0);
        assert!(!r.windows.is_empty());
        for w in &r.windows {
            assert!((w.median_ratio - 1.25).abs() < 1e-12);
            assert!(w.drift_pct.unwrap().abs() < 1e-9);
        }
        assert!((r.overall_median.unwrap() - 1.25).abs() < 1e-12);
    }

    #[test]
    fn a_shift_shows_up_as_drift_from_baseline() {
        let a = series(0, 5, &[1000.0; 60]);
        let mut bw = vec![800.0; 30];
        bw.extend(vec![720.0; 30]);
        let b = series(0, 5, &bw);
        let mut c = cfg();
        c.baseline = Some(1.25);
        let r = twin_input_ratio(&a, &b, &c).unwrap();
        let first = r.windows.first().unwrap();
        let last = r.windows.last().unwrap();
        assert!(first.drift_pct.unwrap().abs() < 1e-9);
        // 1000/720 against 1.25 is +11.1%.
        assert!((last.drift_pct.unwrap() - 11.111).abs() < 0.01);
    }

    #[test]
    fn low_power_pairs_are_left_out() {
        // The first ten samples of B are below the 100 W minimum.
        let a = series(0, 5, &[500.0; 30]);
        let mut bw = vec![50.0; 10];
        bw.extend(vec![400.0; 20]);
        let b = series(0, 5, &bw);
        let r = twin_input_ratio(&a, &b, &cfg()).unwrap();
        assert_eq!(r.pairs_in_time, 30);
        assert_eq!(r.pairs_below_min_power, 10);
        assert_eq!(r.pairs_used, 20);
        assert!((r.overall_median.unwrap() - 1.25).abs() < 1e-12);
    }

    #[test]
    fn the_median_resists_a_passing_cloud() {
        let a = series(0, 5, &[1000.0; 13]);
        let mut bw = vec![800.0; 13];
        bw[6] = 200.0; // one sample of cloud over B
        let b = series(0, 5, &bw);
        let mut c = cfg();
        c.window_ns = 600 * NANOS_PER_SEC;
        let r = twin_input_ratio(&a, &b, &c).unwrap();
        assert!((r.overall_median.unwrap() - 1.25).abs() < 1e-12);
    }

    #[test]
    fn samples_too_far_apart_in_time_are_not_paired() {
        let a = series(0, 10, &[1000.0; 10]);
        let b = series(5, 10, &[800.0; 10]); // always 5 s away; limit is 3 s
        let r = twin_input_ratio(&a, &b, &cfg()).unwrap();
        assert_eq!(r.pairs_in_time, 0);
        assert_eq!(r.unpaired, 10);
        assert!(r.windows.is_empty());
        assert_eq!(r.overall_median, None);
    }

    #[test]
    fn each_b_sample_is_used_at_most_once() {
        // A reports four times for every B report.
        let a = series(0, 1, &[1000.0; 40]);
        let b = series(0, 4, &[800.0; 10]);
        let r = twin_input_ratio(&a, &b, &cfg()).unwrap();
        assert_eq!(r.pairs_in_time, 10);
        assert_eq!(r.unpaired, 30);
    }

    #[test]
    fn unsorted_input_gives_the_same_answer() {
        let a = series(0, 5, &[900.0, 950.0, 1000.0, 1050.0, 1100.0, 1150.0]);
        let b = series(1, 5, &[800.0, 810.0, 820.0, 830.0, 840.0, 850.0]);
        let want = twin_input_ratio(&a, &b, &cfg()).unwrap();
        let (mut ar, mut br) = (a.clone(), b.clone());
        ar.reverse();
        br.swap(0, 4);
        assert_eq!(twin_input_ratio(&ar, &br, &cfg()).unwrap(), want);
    }

    #[test]
    fn empty_and_odd_input_does_not_panic() {
        let c = cfg();
        let some = series(0, 5, &[1000.0; 5]);
        assert_eq!(twin_input_ratio(&[], &[], &c).unwrap().pairs_used, 0);
        assert_eq!(twin_input_ratio(&some, &[], &c).unwrap().unpaired, 5);
        assert_eq!(twin_input_ratio(&[], &some, &c).unwrap().pairs_used, 0);
        let weird = [
            PowerSample {
                at: UtcNanos::from_unix_nanos(i64::MIN),
                watts: f64::NAN,
            },
            PowerSample {
                at: UtcNanos::from_unix_nanos(i64::MAX),
                watts: f64::INFINITY,
            },
            PowerSample {
                at: UtcNanos::from_unix_nanos(0),
                watts: -5.0,
            },
            PowerSample {
                at: UtcNanos::from_unix_nanos(i64::MAX),
                watts: 1e300,
            },
        ];
        let r = twin_input_ratio(&weird, &weird, &c).unwrap();
        assert!(r.windows.iter().all(|w| w.median_ratio.is_finite()));
    }

    #[test]
    fn bad_settings_are_refused() {
        let s = series(0, 5, &[1000.0; 5]);
        let run = |f: fn(&mut RatioConfig)| {
            let mut c = cfg();
            f(&mut c);
            twin_input_ratio(&s, &s, &c)
        };
        assert!(matches!(
            run(|c| c.min_power_w = 0.0),
            Err(RatioError::BadMinPower(_))
        ));
        assert!(matches!(
            run(|c| c.min_power_w = -1.0),
            Err(RatioError::BadMinPower(_))
        ));
        assert!(matches!(
            run(|c| c.min_power_w = f64::NAN),
            Err(RatioError::BadMinPower(_))
        ));
        assert_eq!(run(|c| c.window_ns = 0), Err(RatioError::ZeroWindowOrStep));
        assert_eq!(run(|c| c.step_ns = 0), Err(RatioError::ZeroWindowOrStep));
        assert_eq!(run(|c| c.min_pairs = 0), Err(RatioError::ZeroMinPairs));
        assert!(matches!(
            run(|c| c.baseline = Some(0.0)),
            Err(RatioError::BadBaseline(_))
        ));
        assert!(matches!(
            run(|c| c.baseline = Some(f64::INFINITY)),
            Err(RatioError::BadBaseline(_))
        ));
    }

    #[test]
    fn a_tiny_step_over_a_long_recording_is_refused() {
        let a = series(0, 1, &[1000.0; 2000]);
        let b = series(0, 1, &[800.0; 2000]);
        let mut c = cfg();
        c.step_ns = 1_000_000; // 1 ms steps over 2000 s
        assert_eq!(
            twin_input_ratio(&a, &b, &c),
            Err(RatioError::TooManyWindows)
        );
    }

    #[test]
    fn the_description_never_diagnoses() {
        let a = series(0, 5, &[1000.0; 40]);
        let b = series(0, 5, &[500.0; 40]);
        let mut c = cfg();
        for baseline in [None, Some(1.25)] {
            c.baseline = baseline;
            let text = twin_input_ratio(&a, &b, &c)
                .unwrap()
                .describe()
                .to_lowercase();
            for word in ["fault", "fail", "defect", "broken", "damage", "degrad"] {
                assert!(!text.contains(word), "{text:?} contains {word:?}");
            }
        }
        c.baseline = Some(1.25);
        let text = twin_input_ratio(&a, &b, &c).unwrap().describe();
        assert!(
            text.contains("ratio moved by +60.0% from baseline"),
            "{text}"
        );
        let none = twin_input_ratio(&[], &[], &c).unwrap().describe();
        assert!(none.starts_with("Not enough paired samples"));
    }

    #[test]
    fn stream_keys_parse_and_reject() {
        let k = StreamKey::parse("solar_assistant/inverter_1/pv_power").unwrap();
        assert_eq!(k.source_id.as_str(), "solar_assistant/inverter_1");
        assert_eq!(k.quantity.as_str(), "pv_power");
        assert_eq!(k.to_key_string(), "solar_assistant/inverter_1/pv_power");
        for bad in ["", "pv_power", "/pv_power", "a/", "a b/c"] {
            assert!(StreamKey::parse(bad).is_err(), "{bad:?}");
        }
    }
}
