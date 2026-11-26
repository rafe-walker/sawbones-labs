//! A streaming summary of a recording: what was seen, how often, the range
//! of each value, and where the gaps are. Pure: no I/O.

use std::collections::BTreeMap;

use sawbones_reading::{Reading, TimestampQuality, Unit, UtcNanos};

/// Most distinct series a summary will track. Readings for further series
/// are counted in [`Summary::series_not_tracked`] and otherwise left out.
pub const MAX_SERIES: usize = 10_000;

/// Most gaps a summary will list individually.
pub const MAX_GAPS_LISTED: usize = 50;

/// A stretch with no readings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gap {
    /// The last reading before the gap.
    pub from: UtcNanos,
    /// The first reading after it.
    pub to: UtcNanos,
}

impl Gap {
    /// Length of the gap in seconds.
    pub fn seconds(&self) -> f64 {
        self.to.nanos_since(self.from) as f64 / 1e9
    }
}

/// What was seen for one series (one source id and quantity).
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesSummary {
    /// Readings seen.
    pub count: u64,
    /// Of those, how many were retained replays of unknown age.
    pub retained: u64,
    /// Smallest value.
    pub min: f64,
    /// Largest value.
    pub max: f64,
    /// The unit of the first reading.
    pub unit: Option<Unit>,
    /// Whether later readings carried a different unit from the first.
    pub unit_changed: bool,
    /// Receive time of the first reading.
    pub first: UtcNanos,
    /// Receive time of the latest reading.
    pub last: UtcNanos,
    /// Gaps in this series longer than the threshold.
    pub gaps: u64,
    /// The longest gap in this series, in nanoseconds.
    pub longest_gap_ns: u64,
}

/// The summary of a whole recording.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Summary {
    /// Readings seen.
    pub readings: u64,
    /// Per-series detail, keyed by `<source id>/<quantity>`.
    pub series: BTreeMap<String, SeriesSummary>,
    /// Readings for series beyond [`MAX_SERIES`].
    pub series_not_tracked: u64,
    /// Receive time of the first reading.
    pub first: Option<UtcNanos>,
    /// Receive time of the latest reading.
    pub last: Option<UtcNanos>,
    /// Stretches where no series at all reported for longer than the
    /// threshold. Only the first [`MAX_GAPS_LISTED`] are listed.
    pub overall_gaps: Vec<Gap>,
    /// How many such stretches there were in all.
    pub overall_gap_count: u64,
    /// Readings whose receive time was earlier than the one before.
    pub out_of_order: u64,
}

/// Builds a [`Summary`] one reading at a time, in constant memory per series.
#[derive(Debug, Clone)]
pub struct SummaryBuilder {
    gap_threshold_ns: u64,
    summary: Summary,
}

impl SummaryBuilder {
    /// A builder that reports gaps longer than `gap_threshold_ns`.
    pub fn new(gap_threshold_ns: u64) -> Self {
        Self {
            gap_threshold_ns,
            summary: Summary::default(),
        }
    }

    /// Adds one reading.
    pub fn push(&mut self, r: &Reading) {
        let s = &mut self.summary;
        s.readings += 1;
        let at = r.received_at;

        match s.last {
            Some(prev) if at < prev => s.out_of_order += 1,
            Some(prev) => {
                if at.nanos_since(prev).unsigned_abs() > self.gap_threshold_ns {
                    s.overall_gap_count += 1;
                    if s.overall_gaps.len() < MAX_GAPS_LISTED {
                        s.overall_gaps.push(Gap { from: prev, to: at });
                    }
                }
                s.last = Some(at);
            }
            None => {
                s.first = Some(at);
                s.last = Some(at);
            }
        }

        let key = format!("{}/{}", r.source_id, r.quantity);
        if !s.series.contains_key(&key) && s.series.len() >= MAX_SERIES {
            s.series_not_tracked += 1;
            return;
        }
        let value = r.value.as_f64();
        let retained = u64::from(r.timestamp_quality == TimestampQuality::HostReceiveRetained);
        match s.series.get_mut(&key) {
            None => {
                s.series.insert(
                    key,
                    SeriesSummary {
                        count: 1,
                        retained,
                        min: value,
                        max: value,
                        unit: r.unit.clone(),
                        unit_changed: false,
                        first: at,
                        last: at,
                        gaps: 0,
                        longest_gap_ns: 0,
                    },
                );
            }
            Some(e) => {
                e.count += 1;
                e.retained += retained;
                e.min = e.min.min(value);
                e.max = e.max.max(value);
                e.unit_changed |= e.unit != r.unit;
                if at >= e.last {
                    let gap = at.nanos_since(e.last).unsigned_abs();
                    if gap > self.gap_threshold_ns {
                        e.gaps += 1;
                    }
                    e.longest_gap_ns = e.longest_gap_ns.max(gap);
                    e.last = at;
                }
            }
        }
    }

    /// Finishes and returns the summary.
    pub fn finish(self) -> Summary {
        self.summary
    }
}

impl Summary {
    /// Renders the summary as plain text. Every string in it is a validated
    /// identifier, a validated unit or a number; no outside text is printed.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        let (Some(first), Some(last)) = (self.first, self.last) else {
            return "No readings.\n".to_owned();
        };
        out.push_str(&format!(
            "{} readings across {} series\nfrom {} to {} (receive times, UTC)\n\n",
            self.readings,
            self.series.len(),
            first.to_rfc3339(),
            last.to_rfc3339()
        ));
        let width = self
            .series
            .keys()
            .map(String::len)
            .max()
            .unwrap_or(6)
            .max(6);
        out.push_str(&format!(
            "{:<width$}  {:>8}  {:>12}  {:>12}  {:<5}  {:>5}  {:>13}\n",
            "series", "count", "min", "max", "unit", "gaps", "longest gap s"
        ));
        for (key, e) in &self.series {
            let unit = match (&e.unit, e.unit_changed) {
                (_, true) => "mixed".to_owned(),
                (Some(u), false) => u.to_string(),
                (None, false) => "?".to_owned(),
            };
            out.push_str(&format!(
                "{key:<width$}  {:>8}  {:>12}  {:>12}  {unit:<5}  {:>5}  {:>13.1}\n",
                e.count,
                e.min,
                e.max,
                e.gaps,
                e.longest_gap_ns as f64 / 1e9
            ));
        }
        let retained: u64 = self.series.values().map(|e| e.retained).sum();
        if retained > 0 {
            out.push_str(&format!(
                "\n{retained} readings were retained replays (age unknown).\n"
            ));
        }
        if self.series_not_tracked > 0 {
            out.push_str(&format!(
                "\n{} readings belonged to series beyond the {MAX_SERIES}-series limit and are not broken out.\n",
                self.series_not_tracked
            ));
        }
        if self.out_of_order > 0 {
            out.push_str(&format!(
                "\n{} readings had a receive time earlier than the one before.\n",
                self.out_of_order
            ));
        }
        out.push_str(&format!(
            "\nStretches with no reading from any series: {}\n",
            self.overall_gap_count
        ));
        for g in &self.overall_gaps {
            out.push_str(&format!(
                "  {:>9.1} s  from {} to {}\n",
                g.seconds(),
                g.from.to_rfc3339(),
                g.to.to_rfc3339()
            ));
        }
        if self.overall_gap_count as usize > self.overall_gaps.len() {
            out.push_str("  (further gaps not listed)\n");
        }
        out.push_str(
            "\nA gap is an observation. A series may be quiet because its value did not change,\nbecause the source paused, or because this tap was disconnected.\n",
        );
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Message, Outcome, Parser, ParserConfig};

    const S: i64 = 1_000_000_000;

    fn reading(p: &mut Parser, topic: &str, payload: &str, at_s: i64, retained: bool) -> Reading {
        match p.handle(&Message {
            topic,
            payload: payload.as_bytes(),
            received_at: UtcNanos::from_unix_nanos(at_s * S),
            retained,
        }) {
            Outcome::Reading(r) => *r,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn counts_ranges_and_gaps() {
        let mut p = Parser::new(ParserConfig::new("solar-assistant.local:1883")).unwrap();
        let mut b = SummaryBuilder::new(60 * S as u64);
        let pv = "solar_assistant/inverter_1/pv_power/state";
        let soc = "solar_assistant/total/battery_state_of_charge/state";
        b.push(&reading(&mut p, pv, "100", 0, true));
        b.push(&reading(&mut p, pv, "1500", 10, false));
        b.push(&reading(&mut p, soc, "88", 11, false));
        b.push(&reading(&mut p, pv, "-5", 20, false));
        // 200 s of silence on everything.
        b.push(&reading(&mut p, pv, "900", 220, false));
        let s = b.finish();

        assert_eq!(s.readings, 5);
        assert_eq!(s.series.len(), 2);
        let e = &s.series["solar_assistant/inverter_1/pv_power"];
        assert_eq!((e.count, e.retained), (4, 1));
        assert_eq!((e.min, e.max), (-5.0, 1500.0));
        assert_eq!(e.unit.as_ref().map(Unit::as_str), Some("W"));
        assert_eq!(e.gaps, 1);
        assert_eq!(e.longest_gap_ns, 200 * S as u64);
        assert_eq!(s.overall_gap_count, 1);
        assert_eq!(s.overall_gaps[0].seconds(), 200.0);

        let text = s.to_text();
        assert!(text.contains("5 readings across 2 series"));
        assert!(text.contains("solar_assistant/total/battery_state_of_charge"));
        assert!(text.contains("1 readings were retained replays"));
    }

    #[test]
    fn an_empty_recording_summarises_cleanly() {
        let s = SummaryBuilder::new(1).finish();
        assert_eq!(s.to_text(), "No readings.\n");
    }

    #[test]
    fn out_of_order_readings_are_counted_not_treated_as_gaps() {
        let mut p = Parser::new(ParserConfig::new("solar-assistant.local:1883")).unwrap();
        let mut b = SummaryBuilder::new(60 * S as u64);
        let pv = "solar_assistant/inverter_1/pv_power/state";
        b.push(&reading(&mut p, pv, "1", 1000, false));
        b.push(&reading(&mut p, pv, "2", 10, false));
        b.push(&reading(&mut p, pv, "3", 1001, false));
        let s = b.finish();
        assert_eq!(s.out_of_order, 1);
        assert_eq!(s.overall_gap_count, 0);
        assert_eq!(s.series["solar_assistant/inverter_1/pv_power"].gaps, 0);
    }
}
