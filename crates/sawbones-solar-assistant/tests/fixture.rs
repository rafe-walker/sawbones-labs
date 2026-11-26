//! Runs the parser, the summary and the ratio monitor over the synthetic
//! fixture, and the `sa-tap` binary over files made from it. No broker and
//! no network are involved.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sawbones_reading::{
    Reading, TimestampQuality, Transport, Trust, UnitBasis, FENCE_CLOSE, FENCE_OPEN, READING_SCHEMA,
};
use sawbones_solar_assistant::host_reading;
use sawbones_solar_assistant::parser::{IgnoreReason, Outcome, ParseStats, Parser, ParserConfig};
use sawbones_solar_assistant::ratio::{twin_input_ratio, RatioConfig, SampleCollector, StreamKey};
use sawbones_solar_assistant::raw::{parse_raw_line, RawMessage};
use sawbones_solar_assistant::summary::SummaryBuilder;
use sawbones_solar_assistant::topic::is_command_topic;

const NANOS_PER_SEC: u64 = 1_000_000_000;

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/synthetic-raw-capture.jsonl")
}

fn fixture_text() -> String {
    fs::read_to_string(fixture_path()).expect("the fixture is readable")
}

fn raw_messages() -> Vec<RawMessage> {
    fixture_text()
        .lines()
        .filter_map(parse_raw_line)
        .map(|r| r.expect("every fixture line is a raw-capture record"))
        .collect()
}

fn parser() -> Parser {
    Parser::new(ParserConfig::new("solar-assistant.local:1883")).expect("default config is valid")
}

fn parse_all() -> (Vec<Reading>, Vec<(RawMessage, IgnoreReason)>, ParseStats) {
    let mut p = parser();
    let mut readings = Vec::new();
    let mut ignored = Vec::new();
    for raw in raw_messages() {
        match p.handle(&raw.as_message()) {
            Outcome::Observation(o) => readings.push(host_reading(*o)),
            Outcome::Discovery(_) => {}
            Outcome::Ignored(i) => ignored.push((raw, i.reason)),
        }
    }
    (readings, ignored, p.stats().clone())
}

fn series<'a>(readings: &'a [Reading], source: &str, quantity: &str) -> Vec<&'a Reading> {
    readings
        .iter()
        .filter(|r| r.source_id().as_str() == source && r.quantity().as_str() == quantity)
        .collect()
}

#[test]
fn the_fixture_says_it_is_synthetic() {
    assert!(fixture_text().starts_with("# SYNTHETIC FIXTURE. Not captured from any real device."));
}

#[test]
fn every_message_is_accounted_for() {
    let (readings, ignored, stats) = parse_all();
    assert_eq!(stats.messages, 1068);
    assert_eq!(stats.readings, 937);
    assert_eq!(readings.len(), 937);
    assert_eq!(stats.discovery_changes, 6);
    assert_eq!(ignored.len() as u64, stats.ignored_total());
    assert_eq!(
        stats.readings + stats.discovery_changes + stats.ignored_total(),
        stats.messages
    );
    let want: BTreeMap<IgnoreReason, u64> = [
        (IgnoreReason::OutsidePrefix, 1),
        (IgnoreReason::UnknownTopicShape, 2),
        (IgnoreReason::CommandTopic, 2),
        (IgnoreReason::SetResponse, 1),
        (IgnoreReason::StatusTopic, 1),
        (IgnoreReason::UnusableName, 1),
        (IgnoreReason::EmptyPayload, 1),
        (IgnoreReason::NonNumericPayload, 114),
        (IgnoreReason::DiscoveryMalformed, 1),
        (IgnoreReason::DiscoveryUnrelated, 1),
    ]
    .into_iter()
    .collect();
    assert_eq!(stats.ignored, want);
}

#[test]
fn every_reading_carries_provenance_and_is_unverified() {
    let (readings, _, _) = parse_all();
    for r in &readings {
        assert_eq!(r.schema(), READING_SCHEMA);
        assert_eq!(r.trust(), Trust::Unverified);
        assert_eq!(r.provenance().transport, Transport::Mqtt);
        assert_eq!(r.provenance().broker, "solar-assistant.local:1883");
        assert_eq!(r.source_timestamp(), None);
        let topic = r.provenance().topic.as_untrusted_str();
        assert_eq!(topic, format!("{}/{}/state", r.source_id(), r.quantity()));
        assert!(r.value().as_f64().is_finite());
        assert_eq!(
            r.provenance().raw_payload.as_untrusted_str().trim(),
            r.value().text()
        );
        match r.timestamp_quality() {
            TimestampQuality::HostReceive { uncertainty_ns } => {
                assert!(!r.provenance().retained);
                assert_eq!(uncertainty_ns, 5 * NANOS_PER_SEC);
            }
            TimestampQuality::HostReceiveRetained => assert!(r.provenance().retained),
            TimestampQuality::SourceClock { .. } => panic!("MQTT gives no source clock"),
        }
    }
}

#[test]
fn every_reading_is_one_json_line_and_round_trips() {
    let (readings, _, _) = parse_all();
    for r in &readings {
        let line = serde_json::to_string(r).expect("a reading serialises");
        assert!(!line.contains('\n'));
        let back: Reading = serde_json::from_str(&line).expect("a reading deserialises");
        assert_eq!(&back, r);
    }
}

#[test]
fn units_come_from_discovery_when_declared_and_from_the_name_otherwise() {
    let (readings, _, _) = parse_all();
    let unit_of = |source: &str, quantity: &str| {
        let s = series(&readings, source, quantity);
        assert!(!s.is_empty(), "{source} {quantity} is in the fixture");
        let first = s[0];
        assert!(s
            .iter()
            .all(|r| r.unit() == first.unit() && r.unit_basis() == first.unit_basis()));
        (
            first.unit().map(|u| u.as_str().to_owned()),
            first.unit_basis(),
        )
    };
    let declared = |u: &str| (Some(u.to_owned()), UnitBasis::DeclaredBySource);
    let inferred = |u: &str| (Some(u.to_owned()), UnitBasis::InferredFromName);

    assert_eq!(
        unit_of("solar_assistant/inverter_1", "pv_power"),
        declared("W")
    );
    assert_eq!(
        unit_of("solar_assistant/inverter_2", "pv_power"),
        declared("W")
    );
    assert_eq!(
        unit_of("solar_assistant/inverter_1", "temperature"),
        declared("°C")
    );
    // Declared with Home Assistant's short field names and base-topic shorthand.
    assert_eq!(
        unit_of("solar_assistant/total", "battery_state_of_charge"),
        declared("%")
    );
    assert_eq!(
        unit_of("solar_assistant/inverter_1", "max_charge_current"),
        declared("A")
    );
    // Not declared: guessed from the name.
    assert_eq!(
        unit_of("solar_assistant/inverter_1", "pv_voltage"),
        inferred("V")
    );
    assert_eq!(
        unit_of("solar_assistant/inverter_1", "grid_frequency"),
        inferred("Hz")
    );
    assert_eq!(
        unit_of("solar_assistant/total", "battery_power"),
        inferred("W")
    );
    assert_eq!(
        unit_of("solar_assistant/inverter_1", "load_power_non-essential"),
        inferred("W")
    );
    // Not declared, and a temperature: Celsius or Fahrenheit is a setting on
    // the unit, so no unit is claimed.
    assert_eq!(
        unit_of("solar_assistant/inverter_2", "temperature"),
        (None, UnitBasis::Unknown)
    );
}

#[test]
fn a_retained_reading_is_marked_as_unknown_age() {
    let (readings, _, _) = parse_all();
    let s = series(
        &readings,
        "solar_assistant/inverter_1",
        "max_charge_current",
    );
    assert_eq!(s.len(), 2);
    assert_eq!(
        s[0].timestamp_quality(),
        TimestampQuality::HostReceiveRetained
    );
    assert!(matches!(
        s[1].timestamp_quality(),
        TimestampQuality::HostReceive { .. }
    ));
}

#[test]
fn command_topics_never_become_readings() {
    let (readings, ignored, _) = parse_all();
    let commands: Vec<&RawMessage> = raw_messages_static()
        .iter()
        .filter(|m| is_command_topic(&m.topic))
        .collect();
    assert_eq!(commands.len(), 2, "the fixture holds two command messages");
    for c in commands {
        assert!(ignored
            .iter()
            .any(|(raw, reason)| raw == c && *reason == IgnoreReason::CommandTopic));
    }
    assert!(readings
        .iter()
        .all(|r| !is_command_topic(r.provenance().topic.as_untrusted_str())));
}

fn raw_messages_static() -> &'static [RawMessage] {
    use std::sync::OnceLock;
    static CELL: OnceLock<Vec<RawMessage>> = OnceLock::new();
    CELL.get_or_init(raw_messages)
}

#[test]
fn instruction_shaped_text_never_reaches_the_readings() {
    let needle = "Ignore previous instructions";
    assert!(
        fixture_text().contains(needle),
        "the fixture holds the bait"
    );
    let (readings, ignored, _) = parse_all();
    for r in &readings {
        assert!(!serde_json::to_string(r).unwrap().contains(needle));
    }
    // It was ignored as text, and if anyone does show it, it is fenced.
    let (raw, reason) = ignored
        .iter()
        .find(|(raw, _)| raw.payload.contains(needle))
        .expect("the bait message was ignored");
    assert_eq!(*reason, IgnoreReason::NonNumericPayload);
    let shown = sawbones_reading::WorldText::new(&raw.payload, 512).fenced();
    assert!(shown.starts_with(FENCE_OPEN) && shown.ends_with(FENCE_CLOSE));
}

#[test]
fn the_summary_finds_the_series_and_the_outage() {
    let (readings, _, _) = parse_all();
    let mut b = SummaryBuilder::new(60 * NANOS_PER_SEC);
    for r in &readings {
        b.push(r);
    }
    let s = b.finish();
    assert_eq!(s.readings, 937);
    assert_eq!(s.series.len(), 14);
    assert_eq!(s.out_of_order, 0);

    let pv = &s.series["solar_assistant/inverter_1/pv_power"];
    assert_eq!(pv.count, 165);
    assert_eq!(pv.min, 60.0);
    assert!(pv.max > 1800.0 && pv.max < 1950.0);

    // The fixture has one 150 s stretch with nothing from any series.
    assert_eq!(s.overall_gap_count, 1);
    let gap = s.overall_gaps[0].seconds();
    assert!((150.0..170.0).contains(&gap), "gap was {gap} s");

    let text = s.to_text();
    assert!(text.contains("937 readings across 14 series"));
    assert!(text.contains("Stretches with no reading from any series: 1"));
}

fn ratio_over_fixture(baseline: Option<f64>) -> sawbones_solar_assistant::ratio::RatioReport {
    let (readings, _, _) = parse_all();
    let mut c = SampleCollector::new(
        StreamKey::parse("solar_assistant/inverter_1/pv_power").unwrap(),
        StreamKey::parse("solar_assistant/inverter_2/pv_power").unwrap(),
    );
    for r in &readings {
        c.push(r);
    }
    assert_eq!((c.a.len(), c.b.len()), (165, 165));
    assert_eq!((c.skipped_retained, c.skipped_not_watts), (0, 0));
    let config = RatioConfig {
        min_power_w: 200.0,
        window_ns: 300 * NANOS_PER_SEC,
        step_ns: 60 * NANOS_PER_SEC,
        max_pair_skew_ns: 10 * NANOS_PER_SEC,
        min_pairs: 10,
        baseline,
    };
    twin_input_ratio(&c.a, &c.b, &config).expect("the settings are valid")
}

#[test]
fn the_ratio_monitor_reports_the_shift_as_drift_and_nothing_more() {
    // The fixture holds the two inputs at about 1.25 for 20 minutes, then
    // moves them to about 1.39.
    let r = ratio_over_fixture(Some(1.25));
    assert_eq!(r.unpaired, 0);
    assert!(
        r.pairs_below_min_power > 0,
        "the morning ramp starts below 200 W"
    );
    assert_eq!(r.pairs_used + r.pairs_below_min_power, r.pairs_in_time);

    let first = r.windows.first().expect("there are windows");
    let last = r.windows.last().expect("there are windows");
    assert!(first.drift_pct.unwrap().abs() < 1.0, "{first:?}");
    assert!((1.37..1.41).contains(&last.median_ratio), "{last:?}");
    assert!((9.5..12.5).contains(&last.drift_pct.unwrap()), "{last:?}");

    let text = r.describe();
    assert!(text.contains("ratio moved by +"), "{text}");
    assert!(text.contains("from baseline 1.2500"), "{text}");
    assert!(text.contains("observation, not a diagnosis"), "{text}");
    let lower = text.to_lowercase();
    for word in ["fault", "fail", "defect", "broken", "damage"] {
        assert!(!lower.contains(word), "{text:?} contains {word:?}");
    }
}

#[test]
fn without_a_baseline_no_drift_is_claimed() {
    let r = ratio_over_fixture(None);
    assert!(r.windows.iter().all(|w| w.drift_pct.is_none()));
    assert_eq!(r.overall_drift_pct, None);
    assert!(r.describe().contains("no baseline stored"));
}

// ---- the binary -----------------------------------------------------------

fn sa_tap(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sa-tap"))
        .args(args)
        .env_remove("SA_TAP_MQTT_USERNAME")
        .env_remove("SA_TAP_MQTT_PASSWORD")
        .output()
        .expect("sa-tap runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn temp_file(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    fs::create_dir_all(dir).expect("the test temp directory exists");
    dir.join(format!("{}-{name}", std::process::id()))
}

/// Writes the fixture's readings as `sa-tap record` would, plus some damage.
fn write_readings_file(name: &str) -> PathBuf {
    let (readings, _, _) = parse_all();
    let mut text = String::new();
    for r in &readings {
        text.push_str(&serde_json::to_string(r).unwrap());
        text.push('\n');
    }
    text.push_str("this line is not JSON\n");
    text.push_str("{\"schema\":\"something else\"}\n");
    text.push('\n');
    let path = temp_file(name);
    fs::write(&path, text).expect("the readings file is writable");
    path
}

#[test]
fn replay_summarises_a_raw_capture() {
    let fixture = fixture_path();
    let (ok, stdout, stderr) = sa_tap(&["replay", "--raw", fixture.to_str().unwrap()]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("937 readings across 14 series"), "{stdout}");
    assert!(stderr.contains("1068 messages: 937 readings, 6 discovery configs, 125 ignored"));
    assert!(stderr.contains("non_numeric_payload"));
}

#[test]
fn replay_summarises_a_readings_file_and_survives_bad_lines() {
    let path = write_readings_file("replay.jsonl");
    let (ok, stdout, stderr) = sa_tap(&["replay", path.to_str().unwrap(), "--gap-secs", "60"]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("937 readings across 14 series"), "{stdout}");
    assert!(stderr.contains("2 could not be read"), "{stderr}");
    let _ = fs::remove_file(path);
}

#[test]
fn ratio_runs_over_a_readings_file_and_stores_a_baseline() {
    let path = write_readings_file("ratio.jsonl");
    let baseline = temp_file("baseline.json");
    let common = [
        "--a",
        "solar_assistant/inverter_1/pv_power",
        "--b",
        "solar_assistant/inverter_2/pv_power",
        "--window-secs",
        "300",
    ];

    let mut args = vec!["ratio", path.to_str().unwrap()];
    args.extend(common);
    args.extend(["--save-baseline", baseline.to_str().unwrap()]);
    let (ok, stdout, stderr) = sa_tap(&args);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("no baseline stored"), "{stdout}");

    let mut args = vec!["ratio", path.to_str().unwrap()];
    args.extend(common);
    args.extend(["--baseline-file", baseline.to_str().unwrap()]);
    let (ok, stdout, stderr) = sa_tap(&args);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("ratio moved by"), "{stdout}");
    assert!(stdout.contains("It does not say why."), "{stdout}");
    assert!(!stdout.to_lowercase().contains("fault"), "{stdout}");

    // A baseline for one pair of inputs is refused for another.
    let (ok, _, stderr) = sa_tap(&[
        "ratio",
        path.to_str().unwrap(),
        "--a",
        "solar_assistant/inverter_1/pv_power",
        "--b",
        "solar_assistant/inverter_1/load_power",
        "--baseline-file",
        baseline.to_str().unwrap(),
    ]);
    assert!(!ok);
    assert!(stderr.contains("not the series asked for"), "{stderr}");

    let _ = fs::remove_file(path);
    let _ = fs::remove_file(baseline);
}

#[test]
fn record_takes_credentials_from_the_environment_only() {
    let (ok, help, _) = sa_tap(&["record", "--help"]);
    assert!(ok);
    assert!(help.contains("SA_TAP_MQTT_USERNAME") && help.contains("SA_TAP_MQTT_PASSWORD"));
    assert!(!help.contains("--password") && !help.contains("--username"));
    for flag in ["--password", "--username", "--user", "-u", "-P"] {
        let (ok, _, _) = sa_tap(&["record", flag, "x"]);
        assert!(!ok, "record accepted {flag}");
    }
}

#[test]
fn a_missing_file_is_a_clean_error() {
    let (ok, _, stderr) = sa_tap(&["replay", "/nonexistent/sawbones/recording.jsonl"]);
    assert!(!ok);
    assert!(stderr.contains("could not open"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
}
