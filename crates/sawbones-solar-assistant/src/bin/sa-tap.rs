//! `sa-tap`: a read-only tap on Solar Assistant's MQTT output.
//!
//! PROVISIONAL, and not yet run against a real Solar Assistant unit. See the
//! crate README.
//!
//! - `record` connects, subscribes and writes one JSON reading per line.
//! - `replay` summarises a recording.
//! - `ratio` runs the twin-input ratio monitor over a recording.
//!
//! This program cannot change a setting on the unit: the only MQTT handle it
//! has is `ReadOnlyMqtt`, which can subscribe and receive and nothing else.

#![forbid(unsafe_code)]

use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Args, Parser as ClapParser, Subcommand};
use sawbones_reading::{Reading, UtcNanos};
use sawbones_solar_assistant::backoff::Backoff;
use sawbones_solar_assistant::client::{ConnectParams, Credentials, Inbound, ReadOnlyMqtt};
use sawbones_solar_assistant::parser::{Message, Outcome, ParseStats, Parser, ParserConfig};
use sawbones_solar_assistant::ratio::{
    twin_input_ratio, RatioBaseline, RatioConfig, SampleCollector, StreamKey, BASELINE_SCHEMA,
};
use sawbones_solar_assistant::raw::{
    jsonl_content, parse_raw_line, read_bounded_line, LineRead, RawMessage,
};
use sawbones_solar_assistant::summary::SummaryBuilder;
use sawbones_solar_assistant::{DEFAULT_DISCOVERY_PREFIX, DEFAULT_TOPIC_PREFIX};

/// Environment variable holding the broker username, if the broker needs one.
const ENV_USERNAME: &str = "SA_TAP_MQTT_USERNAME";
/// Environment variable holding the broker password, if the broker needs one.
const ENV_PASSWORD: &str = "SA_TAP_MQTT_PASSWORD";

/// Longest line read from a recording. Longer lines are skipped and counted.
const MAX_LINE_BYTES: usize = 1024 * 1024;

#[derive(ClapParser)]
#[command(
    name = "sa-tap",
    version,
    about = "Read-only tap on Solar Assistant's MQTT output (provisional; not yet run against a real unit)",
    long_about = "Read-only tap on Solar Assistant's MQTT output.\n\n\
        PROVISIONAL: written from documentation, not yet run against a real unit.\n\
        This tool subscribes and listens. It has no code path that can send a\n\
        message to the broker, so it cannot change an inverter setting."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Connect to the broker and write each reading as one JSON line.
    Record(RecordArgs),
    /// Summarise a recording: series seen, counts, ranges and gaps.
    Replay(ReplayArgs),
    /// Run the twin-input ratio monitor over a recording.
    Ratio(RatioArgs),
}

#[derive(Args)]
#[command(
    after_help = "Broker credentials, if needed, are read from the environment variables\n\
    SA_TAP_MQTT_USERNAME and SA_TAP_MQTT_PASSWORD. There is deliberately no flag\n\
    for them, so they cannot end up in shell history or a process listing."
)]
struct RecordArgs {
    /// Broker host name or address.
    #[arg(long, default_value = "solar-assistant.local")]
    host: String,
    /// Broker TCP port.
    #[arg(long, default_value_t = 1883)]
    port: u16,
    /// Solar Assistant's topic prefix. Subscribes to `<prefix>/#`.
    #[arg(long, default_value = DEFAULT_TOPIC_PREFIX)]
    topic_prefix: String,
    /// Also subscribe to Home Assistant discovery configs under
    /// `<PREFIX>/#` (usually `homeassistant`) and take units from them.
    #[arg(long, value_name = "PREFIX")]
    discovery_prefix: Option<String>,
    /// Append readings to this file instead of writing to standard output.
    #[arg(long, value_name = "FILE")]
    out: Option<PathBuf>,
    /// Also append every message exactly as received to this file, so the
    /// capture can be parsed again later (`replay --raw`).
    #[arg(long, value_name = "FILE")]
    raw_log: Option<PathBuf>,
    /// Stated bound, in milliseconds, on the delay between Solar Assistant
    /// taking a value and this host receiving it. An assumption recorded
    /// with every reading, not something this tool measures.
    #[arg(long, default_value_t = 5000, value_name = "MS")]
    timestamp_uncertainty_ms: u64,
    /// MQTT client id. Defaults to one built from the process id.
    #[arg(long)]
    client_id: Option<String>,
}

#[derive(Args)]
struct InputArgs {
    /// The recording to read (JSON lines).
    file: PathBuf,
    /// The file is a raw capture (`record --raw-log`), not readings. It is
    /// run through the parser first.
    #[arg(long)]
    raw: bool,
    /// With --raw: Solar Assistant's topic prefix.
    #[arg(long, default_value = DEFAULT_TOPIC_PREFIX)]
    topic_prefix: String,
    /// With --raw: Home Assistant discovery prefix.
    #[arg(long, default_value = DEFAULT_DISCOVERY_PREFIX)]
    discovery_prefix: String,
}

#[derive(Args)]
struct ReplayArgs {
    #[command(flatten)]
    input: InputArgs,
    /// Report stretches with no readings longer than this many seconds.
    #[arg(long, default_value_t = 60.0, value_name = "SECONDS")]
    gap_secs: f64,
}

#[derive(Args)]
struct RatioArgs {
    #[command(flatten)]
    input: InputArgs,
    /// Input A, as `<source id>/<quantity>`, for example
    /// `solar_assistant/inverter_1/pv_power`.
    #[arg(long, value_name = "SERIES")]
    a: String,
    /// Input B, in the same form.
    #[arg(long, value_name = "SERIES")]
    b: String,
    /// Both inputs must be at or above this many watts for a pair to count.
    #[arg(long, default_value_t = 200.0, value_name = "WATTS")]
    min_power: f64,
    /// Sliding window length.
    #[arg(long, default_value_t = 600.0, value_name = "SECONDS")]
    window_secs: f64,
    /// How far the window slides between reports.
    #[arg(long, default_value_t = 60.0, value_name = "SECONDS")]
    step_secs: f64,
    /// Pair two samples only if received within this many seconds.
    #[arg(long, default_value_t = 10.0, value_name = "SECONDS")]
    max_skew_secs: f64,
    /// A window reports only with at least this many pairs.
    #[arg(long, default_value_t = 10)]
    min_pairs: usize,
    /// Baseline ratio A/B to compare against.
    #[arg(long, value_name = "RATIO", conflicts_with = "baseline_file")]
    baseline: Option<f64>,
    /// Read the baseline from a file written by --save-baseline.
    #[arg(long, value_name = "FILE")]
    baseline_file: Option<PathBuf>,
    /// Store this run's overall median ratio as a baseline file.
    #[arg(long, value_name = "FILE")]
    save_baseline: Option<PathBuf>,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // One line, with the chain of causes, and no backtrace.
            eprintln!("sa-tap: error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Record(args) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .context("could not start the async runtime")?;
            runtime.block_on(record(args))
        }
        Command::Replay(args) => replay(args),
        Command::Ratio(args) => ratio(args),
    }
}

/// Reads optional broker credentials from the environment. Both variables
/// or neither; the values are never printed.
fn credentials_from_env() -> Result<Option<Credentials>> {
    let get = |name: &str| match std::env::var(name) {
        Ok(v) if !v.is_empty() => Ok(Some(v)),
        Ok(_) | Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            bail!("environment variable {name} is not valid UTF-8")
        }
    };
    match (get(ENV_USERNAME)?, get(ENV_PASSWORD)?) {
        (Some(u), Some(p)) => Ok(Some(Credentials::new(u, p))),
        (None, None) => Ok(None),
        (Some(_), None) => bail!("{ENV_USERNAME} is set but {ENV_PASSWORD} is not"),
        (None, Some(_)) => bail!("{ENV_PASSWORD} is set but {ENV_USERNAME} is not"),
    }
}

fn open_append(path: &Path) -> Result<BufWriter<File>> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("could not open {} for appending", path.display()))?;
    Ok(BufWriter::new(file))
}

fn write_json_line<W: Write, T: serde::Serialize>(out: &mut W, value: &T) -> Result<()> {
    serde_json::to_writer(&mut *out, value).context("could not write a JSON line")?;
    out.write_all(b"\n")
        .and_then(|()| out.flush())
        .context("could not write a JSON line")
}

fn print_parse_stats(stats: &ParseStats) {
    eprintln!(
        "sa-tap: {} messages: {} readings, {} discovery configs, {} ignored",
        stats.messages,
        stats.readings,
        stats.discovery_changes,
        stats.ignored_total()
    );
    for (reason, count) in &stats.ignored {
        // The reason is one of this program's own enum names, not outside text.
        let name = serde_json::to_string(reason).unwrap_or_default();
        eprintln!("sa-tap:   ignored {:<22} {count}", name.trim_matches('"'));
    }
}

async fn record(args: RecordArgs) -> Result<()> {
    let credentials = credentials_from_env()?;
    let broker = format!("{}:{}", args.host, args.port);
    let mut parser = Parser::new(ParserConfig {
        topic_prefix: args.topic_prefix.clone(),
        discovery_prefix: args.discovery_prefix.clone(),
        broker: broker.clone(),
        host_receive_uncertainty_ns: args.timestamp_uncertainty_ms.saturating_mul(1_000_000),
    })
    .context("unusable settings")?;

    let mut filters = vec![format!("{}/#", args.topic_prefix)];
    if let Some(dp) = &args.discovery_prefix {
        filters.push(format!("{dp}/#"));
    }

    let mut out: Box<dyn Write> = match &args.out {
        Some(path) => Box::new(open_append(path)?),
        None => Box::new(io::stdout()),
    };
    let mut raw_log = match &args.raw_log {
        Some(path) => Some(open_append(path)?),
        None => None,
    };

    let mut mqtt = ReadOnlyMqtt::new(ConnectParams {
        host: args.host.clone(),
        port: args.port,
        client_id: args
            .client_id
            .clone()
            .unwrap_or_else(|| format!("sawbones-sa-tap-{}", std::process::id())),
        credentials,
        keep_alive: Duration::from_secs(30),
    });
    let mut backoff = Backoff::default();

    eprintln!("sa-tap: read-only tap. Connecting to {broker}. Ctrl-C to stop.");
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    'run: loop {
        tokio::select! {
            biased;
            signal = &mut shutdown => {
                signal.context("could not listen for Ctrl-C")?;
                break 'run;
            }
            event = mqtt.poll() => match event {
                Ok(Inbound::Connected) => {
                    backoff.reset();
                    eprintln!("sa-tap: connected to {broker}");
                    // The session is clean, so subscribe afresh every time.
                    for filter in &filters {
                        mqtt.subscribe(filter).await?;
                        eprintln!("sa-tap: subscribing to {filter}");
                    }
                }
                Ok(Inbound::SubscribeAck { granted: false }) => {
                    eprintln!("sa-tap: the broker refused a subscription (check its access rules)");
                }
                Ok(Inbound::SubscribeAck { granted: true }) | Ok(Inbound::Other) => {}
                Ok(Inbound::Message { topic, payload, retained }) => {
                    let received_at = UtcNanos::now();
                    if let Some(raw) = raw_log.as_mut() {
                        write_json_line(raw, &RawMessage::capture(&topic, &payload, received_at, retained))?;
                    }
                    let message = Message { topic: &topic, payload: &payload, received_at, retained };
                    if let Outcome::Reading(reading) = parser.handle(&message) {
                        write_json_line(&mut out, &*reading)?;
                    }
                }
                Err(error) => {
                    let delay = backoff.next_delay();
                    eprintln!("sa-tap: {error}; trying again in {} s", delay.as_secs_f64());
                    tokio::select! {
                        biased;
                        signal = &mut shutdown => {
                            signal.context("could not listen for Ctrl-C")?;
                            break 'run;
                        }
                        () = tokio::time::sleep(delay) => {}
                    }
                }
            }
        }
    }

    out.flush().context("could not flush the output")?;
    if let Some(raw) = raw_log.as_mut() {
        raw.flush().context("could not flush the raw log")?;
    }
    eprintln!("sa-tap: stopped.");
    print_parse_stats(parser.stats());
    Ok(())
}

/// Counts from reading a recording file.
#[derive(Default)]
struct InputStats {
    lines: u64,
    malformed_lines: u64,
    overlong_lines: u64,
    parse: Option<ParseStats>,
}

/// Reads a recording and hands each reading to `each`. Lines that cannot be
/// used are counted, never fatal.
fn for_each_reading(input: &InputArgs, mut each: impl FnMut(&Reading)) -> Result<InputStats> {
    let file = File::open(&input.file)
        .with_context(|| format!("could not open {}", input.file.display()))?;
    let mut reader = BufReader::new(file);
    let mut parser = if input.raw {
        let mut config = ParserConfig::new("raw-capture");
        config.topic_prefix = input.topic_prefix.clone();
        config.discovery_prefix = Some(input.discovery_prefix.clone());
        Some(Parser::new(config).context("unusable settings")?)
    } else {
        None
    };

    let mut stats = InputStats::default();
    let mut buf = Vec::new();
    loop {
        match read_bounded_line(&mut reader, &mut buf, MAX_LINE_BYTES)
            .with_context(|| format!("could not read {}", input.file.display()))?
        {
            LineRead::Eof => break,
            LineRead::TooLong => {
                stats.lines += 1;
                stats.overlong_lines += 1;
                continue;
            }
            LineRead::Line => {}
        }
        let line = String::from_utf8_lossy(&buf);
        match parser.as_mut() {
            Some(parser) => match parse_raw_line(&line) {
                None => {}
                Some(Err(_)) => {
                    stats.lines += 1;
                    stats.malformed_lines += 1;
                }
                Some(Ok(raw)) => {
                    stats.lines += 1;
                    if let Outcome::Reading(reading) = parser.handle(&raw.as_message()) {
                        each(&reading);
                    }
                }
            },
            None => {
                let Some(content) = jsonl_content(&line) else {
                    continue;
                };
                stats.lines += 1;
                match serde_json::from_str::<Reading>(content) {
                    Ok(reading) => each(&reading),
                    Err(_) => stats.malformed_lines += 1,
                }
            }
        }
    }
    stats.parse = parser.map(|p| p.stats().clone());
    Ok(stats)
}

fn print_input_stats(stats: &InputStats) {
    if stats.malformed_lines > 0 || stats.overlong_lines > 0 {
        eprintln!(
            "sa-tap: of {} lines, {} could not be read and {} were too long; they were skipped",
            stats.lines, stats.malformed_lines, stats.overlong_lines
        );
    }
    if let Some(parse) = &stats.parse {
        print_parse_stats(parse);
    }
}

fn seconds_to_nanos(name: &str, seconds: f64) -> Result<u64> {
    // `contains` is false for NaN, so this also rejects it.
    if !(0.0..=1e9).contains(&seconds) {
        bail!("{name} must be between 0 and 1e9 seconds, got {seconds}");
    }
    // The range check above keeps the product well inside u64.
    Ok((seconds * 1e9).round() as u64)
}

fn replay(args: ReplayArgs) -> Result<()> {
    let mut builder = SummaryBuilder::new(seconds_to_nanos("--gap-secs", args.gap_secs)?);
    let stats = for_each_reading(&args.input, |r| builder.push(r))?;
    print!("{}", builder.finish().to_text());
    print_input_stats(&stats);
    Ok(())
}

fn ratio(args: RatioArgs) -> Result<()> {
    let a_key = StreamKey::parse(&args.a).context("--a")?;
    let b_key = StreamKey::parse(&args.b).context("--b")?;
    if a_key == b_key {
        bail!("--a and --b name the same series");
    }

    let baseline = match (&args.baseline, &args.baseline_file) {
        (Some(ratio), _) => Some(*ratio),
        (None, Some(path)) => {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("could not read {}", path.display()))?;
            let stored: RatioBaseline = serde_json::from_str(&text)
                .with_context(|| format!("{} is not a baseline file", path.display()))?;
            if stored.a != a_key || stored.b != b_key {
                bail!(
                    "the baseline in {} is for {} against {}, not the series asked for",
                    path.display(),
                    stored.a.to_key_string(),
                    stored.b.to_key_string()
                );
            }
            Some(stored.ratio)
        }
        (None, None) => None,
    };

    let config = RatioConfig {
        min_power_w: args.min_power,
        window_ns: seconds_to_nanos("--window-secs", args.window_secs)?,
        step_ns: seconds_to_nanos("--step-secs", args.step_secs)?,
        max_pair_skew_ns: seconds_to_nanos("--max-skew-secs", args.max_skew_secs)?,
        min_pairs: args.min_pairs,
        baseline,
    };

    let mut collector = SampleCollector::new(a_key.clone(), b_key.clone());
    let stats = for_each_reading(&args.input, |r| collector.push(r))?;
    print_input_stats(&stats);

    let report = twin_input_ratio(&collector.a, &collector.b, &config)?;

    println!("A = {}", a_key.to_key_string());
    println!("B = {}", b_key.to_key_string());
    println!(
        "samples: A {}, B {}; left out: {} retained, {} not in watts",
        collector.a.len(),
        collector.b.len(),
        collector.skipped_retained,
        collector.skipped_not_watts
    );
    println!(
        "pairs: {} close enough in time, {} used, {} below {} W; {} samples of A unpaired",
        report.pairs_in_time,
        report.pairs_used,
        report.pairs_below_min_power,
        config.min_power_w,
        report.unpaired
    );
    if !report.windows.is_empty() {
        println!();
        println!(
            "{:<30}  {:>5}  {:>10}  {:>9}",
            "window end (UTC)", "pairs", "median A/B", "drift %"
        );
        for w in &report.windows {
            let drift = w
                .drift_pct
                .map_or_else(|| "-".to_owned(), |d| format!("{d:+.1}"));
            println!(
                "{:<30}  {:>5}  {:>10.4}  {:>9}",
                w.window_end.to_rfc3339(),
                w.pairs,
                w.median_ratio,
                drift
            );
        }
    }
    println!();
    println!("{}", report.describe());
    println!(
        "Two arrays can differ in panel count, tilt, shading and clipping. A changed ratio says\n\
         the ratio changed. It does not say why."
    );

    if let Some(path) = &args.save_baseline {
        let Some(median) = report.overall_median else {
            bail!("no baseline saved: not enough usable pairs to state a ratio");
        };
        let stored = RatioBaseline {
            schema: BASELINE_SCHEMA.to_owned(),
            a: a_key,
            b: b_key,
            ratio: median,
            pairs: report.pairs_used,
            min_power_w: config.min_power_w,
        };
        let text = serde_json::to_string_pretty(&stored).context("could not encode baseline")?;
        std::fs::write(path, text + "\n")
            .with_context(|| format!("could not write {}", path.display()))?;
        eprintln!("sa-tap: baseline saved to {}", path.display());
    }
    Ok(())
}
