//! The pure parser: one MQTT message in, one typed outcome out.
//!
//! Nothing in this module does I/O or reads a clock. The caller supplies the
//! topic, the payload bytes, the receive time and the retained flag, which
//! makes every path testable from fixtures.
//!
//! The parser never panics on input. Whatever a broker sends (an unknown
//! topic, a text payload, invalid UTF-8, a megabyte of garbage) comes back as
//! an [`Outcome::Ignored`] with a typed [`IgnoreReason`], and is counted in
//! [`ParseStats`].

use std::collections::{BTreeMap, HashMap};

use sawbones_reading::{
    DecimalValue, Ident, Observation, Provenance, TimestampQuality, Transport, Unit, UnitBasis,
    UtcNanos, WorldText,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::discovery::{self, DeclaredUnit, SensorDeclaration};
use crate::topic::{classify, TopicClass};
use crate::units;

/// Longest topic the parser will look at, in bytes.
pub const MAX_TOPIC_BYTES: usize = 512;

/// How much of a payload is kept in a reading's provenance, in bytes.
pub const MAX_PROVENANCE_PAYLOAD_BYTES: usize = 256;

/// Most discovery declarations the parser will remember. A broker cannot
/// make the parser's memory grow without bound.
pub const MAX_DECLARATIONS: usize = 4096;

/// Default stated bound on the delay between Solar Assistant taking a value
/// and this host receiving it: five seconds.
///
/// This is an **assumption, not a measurement**. Solar Assistant's
/// documentation gives no update interval; captures posted by users show
/// messages about a second apart. The operator can state a different bound.
pub const DEFAULT_HOST_RECEIVE_UNCERTAINTY_NS: u64 = 5_000_000_000;

/// Settings for a [`Parser`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserConfig {
    /// Solar Assistant's topic prefix, `solar_assistant` unless changed.
    pub topic_prefix: String,
    /// Home Assistant discovery prefix, or `None` to ignore discovery.
    pub discovery_prefix: Option<String>,
    /// The broker the messages came from, as `host:port`, for provenance.
    pub broker: String,
    /// Stated bound on measurement-to-receipt delay; see
    /// [`DEFAULT_HOST_RECEIVE_UNCERTAINTY_NS`].
    pub host_receive_uncertainty_ns: u64,
}

impl ParserConfig {
    /// A config with the default prefixes and uncertainty for `broker`.
    pub fn new(broker: &str) -> Self {
        Self {
            topic_prefix: crate::DEFAULT_TOPIC_PREFIX.to_owned(),
            discovery_prefix: Some(crate::DEFAULT_DISCOVERY_PREFIX.to_owned()),
            broker: broker.to_owned(),
            host_receive_uncertainty_ns: DEFAULT_HOST_RECEIVE_UNCERTAINTY_NS,
        }
    }
}

/// Why a [`ParserConfig`] was rejected.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigError {
    /// A prefix was empty, began or ended with `/`, held an MQTT wildcard,
    /// or held a character an identifier cannot.
    #[error("topic prefix {0:?} is not usable: it must be non-empty, must not start or end with '/', and may hold only A-Z a-z 0-9 _ - . /")]
    BadPrefix(String),
    /// The broker label was empty, too long, or held control characters.
    #[error("broker label is empty, longer than 255 bytes, or holds control characters or '@'")]
    BadBroker,
}

/// Checks that a topic prefix is safe to subscribe under and to build
/// identifiers from.
pub fn validate_prefix(prefix: &str) -> Result<(), ConfigError> {
    let ok = !prefix.is_empty()
        && !prefix.starts_with('/')
        && !prefix.ends_with('/')
        && !prefix.contains("//")
        && prefix.len() <= 96
        && Ident::new(prefix).is_ok();
    if ok {
        Ok(())
    } else {
        Err(ConfigError::BadPrefix(prefix.to_owned()))
    }
}

/// One MQTT message, as the parser needs it.
#[derive(Debug, Clone, Copy)]
pub struct Message<'a> {
    /// The topic the message arrived on.
    pub topic: &'a str,
    /// The payload bytes.
    pub payload: &'a [u8],
    /// When this host received it.
    pub received_at: UtcNanos,
    /// Whether the broker flagged it as a retained (stored) message.
    pub retained: bool,
}

/// Why a message produced no reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IgnoreReason {
    /// The topic was longer than [`MAX_TOPIC_BYTES`].
    TopicTooLong,
    /// The topic was under neither configured prefix.
    OutsidePrefix,
    /// The topic was under the prefix but of no known shape.
    UnknownTopicShape,
    /// The topic was a command topic (`/set` or `/set_verified`): some other
    /// client changed, or tried to change, a setting.
    CommandTopic,
    /// The topic was Solar Assistant's reply to a setting change.
    SetResponse,
    /// The topic was Solar Assistant's status topic.
    StatusTopic,
    /// The device or value name could not be used as an identifier.
    UnusableName,
    /// The payload was empty or only whitespace.
    EmptyPayload,
    /// The payload was not valid UTF-8.
    InvalidUtf8,
    /// The payload was text, not a plain decimal number. Mode and priority
    /// values such as `Hybrid` land here.
    NonNumericPayload,
    /// A discovery config could not be read.
    DiscoveryMalformed,
    /// A discovery config described a topic outside the configured prefix,
    /// or a command topic.
    DiscoveryUnrelated,
    /// The parser already holds [`MAX_DECLARATIONS`] declarations.
    DiscoveryStoreFull,
}

/// A message that produced no reading, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ignored {
    /// Why it was ignored.
    pub reason: IgnoreReason,
    /// The topic it arrived on, as outside text.
    pub topic: WorldText,
}

/// What a discovery config message did to the parser's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryChange {
    /// A declaration was added or replaced.
    Stored,
    /// A declaration was removed (the config payload was empty).
    Removed,
}

/// The result of handing one message to the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// A numeric value on a state topic. It carries no trust tier; the host
    /// attaches one (see [`crate::host_reading`]).
    Observation(Box<Observation>),
    /// A discovery config was taken in. It will shape later readings.
    Discovery(DiscoveryChange),
    /// Nothing usable, with the reason.
    Ignored(Ignored),
}

/// Running counts of what the parser has seen.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseStats {
    /// Messages handled.
    pub messages: u64,
    /// Messages that became readings.
    pub readings: u64,
    /// Discovery configs stored or removed.
    pub discovery_changes: u64,
    /// Ignored messages, by reason.
    pub ignored: BTreeMap<IgnoreReason, u64>,
}

impl ParseStats {
    /// Total ignored messages across all reasons.
    pub fn ignored_total(&self) -> u64 {
        self.ignored.values().sum()
    }
}

/// The Solar Assistant message parser. Holds only what discovery messages
/// have declared so far, and counts.
#[derive(Debug, Clone)]
pub struct Parser {
    config: ParserConfig,
    /// State topic to its declaration.
    declarations: HashMap<String, SensorDeclaration>,
    /// Discovery config topic to the state topic it declared, so an empty
    /// config payload can remove the right declaration.
    config_topics: HashMap<String, String>,
    stats: ParseStats,
}

impl Parser {
    /// Builds a parser, checking the config first.
    pub fn new(config: ParserConfig) -> Result<Self, ConfigError> {
        validate_prefix(&config.topic_prefix)?;
        if let Some(dp) = &config.discovery_prefix {
            validate_prefix(dp)?;
        }
        if config.broker.is_empty()
            || config.broker.len() > 255
            || config.broker.chars().any(|c| c.is_control() || c == '@')
        {
            return Err(ConfigError::BadBroker);
        }
        Ok(Self {
            config,
            declarations: HashMap::new(),
            config_topics: HashMap::new(),
            stats: ParseStats::default(),
        })
    }

    /// Counts so far.
    pub fn stats(&self) -> &ParseStats {
        &self.stats
    }

    /// How many discovery declarations are held.
    pub fn declaration_count(&self) -> usize {
        self.declarations.len()
    }

    /// The declaration held for a state topic, if any.
    pub fn declaration_for(&self, state_topic: &str) -> Option<&SensorDeclaration> {
        self.declarations.get(state_topic)
    }

    /// Handles one message. Never panics; never does I/O.
    pub fn handle(&mut self, msg: &Message<'_>) -> Outcome {
        self.stats.messages += 1;
        let outcome = self.handle_inner(msg);
        match &outcome {
            Outcome::Observation(_) => self.stats.readings += 1,
            Outcome::Discovery(_) => self.stats.discovery_changes += 1,
            Outcome::Ignored(i) => *self.stats.ignored.entry(i.reason).or_insert(0) += 1,
        }
        outcome
    }

    fn handle_inner(&mut self, msg: &Message<'_>) -> Outcome {
        let ignored = |reason| {
            Outcome::Ignored(Ignored {
                reason,
                topic: WorldText::new(msg.topic, MAX_TOPIC_BYTES),
            })
        };
        if msg.topic.len() > MAX_TOPIC_BYTES {
            return ignored(IgnoreReason::TopicTooLong);
        }
        let class = classify(
            msg.topic,
            &self.config.topic_prefix,
            self.config.discovery_prefix.as_deref(),
        );
        match class {
            TopicClass::OutsidePrefix => ignored(IgnoreReason::OutsidePrefix),
            TopicClass::UnknownShape => ignored(IgnoreReason::UnknownTopicShape),
            TopicClass::Command => ignored(IgnoreReason::CommandTopic),
            TopicClass::SetResponse => ignored(IgnoreReason::SetResponse),
            TopicClass::Status => ignored(IgnoreReason::StatusTopic),
            TopicClass::DiscoveryConfig => match self.take_discovery(msg) {
                Ok(change) => Outcome::Discovery(change),
                Err(reason) => ignored(reason),
            },
            TopicClass::State { device, name } => match self.read_state(msg, device, name) {
                Ok(observation) => Outcome::Observation(Box::new(observation)),
                Err(reason) => ignored(reason),
            },
        }
    }

    fn take_discovery(&mut self, msg: &Message<'_>) -> Result<DiscoveryChange, IgnoreReason> {
        if msg.payload.is_empty() {
            // An empty config payload withdraws the declaration.
            return match self.config_topics.remove(msg.topic) {
                Some(state_topic) => {
                    self.declarations.remove(&state_topic);
                    Ok(DiscoveryChange::Removed)
                }
                None => Err(IgnoreReason::DiscoveryUnrelated),
            };
        }
        let decl =
            discovery::parse_config(msg.payload).map_err(|_| IgnoreReason::DiscoveryMalformed)?;
        // Only declarations about this unit's own value topics are kept. A
        // shared broker carries discovery for every other device too.
        let describes_our_state = matches!(
            classify(&decl.state_topic, &self.config.topic_prefix, None),
            TopicClass::State { .. }
        );
        if !describes_our_state {
            return Err(IgnoreReason::DiscoveryUnrelated);
        }
        let is_new = !self.declarations.contains_key(&decl.state_topic);
        if is_new && self.declarations.len() >= MAX_DECLARATIONS {
            return Err(IgnoreReason::DiscoveryStoreFull);
        }
        if let Some(old_state) = self
            .config_topics
            .insert(msg.topic.to_owned(), decl.state_topic.clone())
        {
            if old_state != decl.state_topic {
                self.declarations.remove(&old_state);
            }
        }
        self.declarations.insert(decl.state_topic.clone(), decl);
        Ok(DiscoveryChange::Stored)
    }

    fn read_state(
        &self,
        msg: &Message<'_>,
        device: &str,
        name: &str,
    ) -> Result<Observation, IgnoreReason> {
        let source_id = Ident::new(&format!("{}/{device}", self.config.topic_prefix))
            .map_err(|_| IgnoreReason::UnusableName)?;
        let quantity = Ident::new(name).map_err(|_| IgnoreReason::UnusableName)?;

        let text = std::str::from_utf8(msg.payload).map_err(|_| IgnoreReason::InvalidUtf8)?;
        let text = text.trim_matches(|c: char| c.is_ascii_whitespace());
        if text.is_empty() {
            return Err(IgnoreReason::EmptyPayload);
        }
        let value = DecimalValue::parse(text).map_err(|_| IgnoreReason::NonNumericPayload)?;

        let (unit, unit_basis) = self.unit_for(msg.topic, name);
        let timestamp_quality = if msg.retained {
            TimestampQuality::HostReceiveRetained
        } else {
            TimestampQuality::HostReceive {
                uncertainty_ns: self.config.host_receive_uncertainty_ns,
            }
        };

        Ok(Observation {
            source_id,
            quantity,
            value,
            unit,
            unit_basis,
            received_at: msg.received_at,
            // MQTT 3.1.1 carries no timestamp and Solar Assistant's payloads
            // are bare numbers, so there is no source time to record.
            source_timestamp: None,
            timestamp_quality,
            provenance: Provenance {
                transport: Transport::Mqtt,
                broker: self.config.broker.clone(),
                topic: WorldText::new(msg.topic, MAX_TOPIC_BYTES),
                raw_payload: WorldText::from_bytes_lossy(msg.payload, MAX_PROVENANCE_PAYLOAD_BYTES),
                retained: msg.retained,
            },
        })
    }

    /// A declared unit wins over a guess from the name. A declaration that
    /// exists but names no usable unit leaves the unit unknown: the source
    /// has spoken, and guessing over it would hide that.
    fn unit_for(&self, topic: &str, name: &str) -> (Option<Unit>, UnitBasis) {
        match self.declarations.get(topic).map(|d| &d.unit) {
            Some(DeclaredUnit::Recognised(u)) => match Unit::new(u) {
                Ok(unit) => (Some(unit), UnitBasis::DeclaredBySource),
                Err(_) => (None, UnitBasis::Unknown),
            },
            Some(DeclaredUnit::Unrecognised(_)) | Some(DeclaredUnit::Absent) => {
                (None, UnitBasis::Unknown)
            }
            None => match units::infer_unit_from_name(name).and_then(|u| Unit::new(u).ok()) {
                Some(unit) => (Some(unit), UnitBasis::InferredFromName),
                None => (None, UnitBasis::Unknown),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{host_reading, SOLAR_ASSISTANT_DEFAULT_TRUST};
    use sawbones_reading::Trust;

    const T0: UtcNanos = UtcNanos::from_unix_nanos(1_795_631_400_000_000_000);

    fn parser() -> Parser {
        Parser::new(ParserConfig::new("solar-assistant.local:1883")).unwrap()
    }

    fn msg<'a>(topic: &'a str, payload: &'a [u8]) -> Message<'a> {
        Message {
            topic,
            payload,
            received_at: T0,
            retained: false,
        }
    }

    fn reading(p: &mut Parser, topic: &str, payload: &[u8]) -> Observation {
        match p.handle(&msg(topic, payload)) {
            Outcome::Observation(r) => *r,
            other => panic!("expected a reading for {topic}, got {other:?}"),
        }
    }

    fn reason(p: &mut Parser, topic: &str, payload: &[u8]) -> IgnoreReason {
        match p.handle(&msg(topic, payload)) {
            Outcome::Ignored(i) => i.reason,
            other => panic!("expected ignored for {topic}, got {other:?}"),
        }
    }

    #[test]
    fn a_numeric_state_message_becomes_a_reading() {
        let mut p = parser();
        let r = reading(&mut p, "solar_assistant/inverter_1/pv_power/state", b"1823");
        assert_eq!(r.source_id.as_str(), "solar_assistant/inverter_1");
        assert_eq!(r.quantity.as_str(), "pv_power");
        assert_eq!(r.value.text(), "1823");
        assert_eq!(r.value.as_f64(), 1823.0);
        assert_eq!(r.unit.as_ref().map(Unit::as_str), Some("W"));
        assert_eq!(r.unit_basis, UnitBasis::InferredFromName);
        assert_eq!(r.received_at, T0);
        assert_eq!(r.source_timestamp, None);
        assert_eq!(
            r.timestamp_quality,
            TimestampQuality::HostReceive {
                uncertainty_ns: DEFAULT_HOST_RECEIVE_UNCERTAINTY_NS
            }
        );
        assert_eq!(r.provenance.transport, Transport::Mqtt);
        assert_eq!(r.provenance.broker, "solar-assistant.local:1883");
        assert_eq!(
            r.provenance.topic.as_untrusted_str(),
            "solar_assistant/inverter_1/pv_power/state"
        );
        assert_eq!(r.provenance.raw_payload.as_untrusted_str(), "1823");
    }

    #[test]
    fn solar_assistant_readings_are_unverified_by_default() {
        let mut p = parser();
        let r = reading(&mut p, "solar_assistant/total/battery_power/state", b"-420");
        assert_eq!(host_reading(r).trust(), Trust::Unverified);
        assert_eq!(SOLAR_ASSISTANT_DEFAULT_TRUST, Trust::Unverified);
    }

    #[test]
    fn payload_whitespace_is_trimmed_but_kept_in_provenance() {
        let mut p = parser();
        let r = reading(
            &mut p,
            "solar_assistant/inverter_1/grid_voltage/state",
            b" 239.6\n",
        );
        assert_eq!(r.value.text(), "239.6");
        assert_eq!(r.provenance.raw_payload.as_untrusted_str(), " 239.6\n");
    }

    #[test]
    fn retained_messages_are_marked_as_unknown_age() {
        let mut p = parser();
        let m = Message {
            retained: true,
            ..msg("solar_assistant/inverter_1/pv_power/state", b"5")
        };
        match p.handle(&m) {
            Outcome::Observation(r) => {
                assert_eq!(r.timestamp_quality, TimestampQuality::HostReceiveRetained);
                assert!(r.provenance.retained);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn text_payloads_are_ignored_and_counted() {
        let mut p = parser();
        for payload in [
            &b"Hybrid"[..],
            b"Solar/Battery",
            b"Load first",
            b"NaN",
            b"inf",
            b"12 W",
        ] {
            assert_eq!(
                reason(
                    &mut p,
                    "solar_assistant/inverter_1/device_mode/state",
                    payload
                ),
                IgnoreReason::NonNumericPayload
            );
        }
        assert_eq!(p.stats().ignored[&IgnoreReason::NonNumericPayload], 6);
        assert_eq!(p.stats().readings, 0);
    }

    #[test]
    fn empty_and_binary_payloads_are_ignored() {
        let mut p = parser();
        let t = "solar_assistant/inverter_1/pv_power/state";
        assert_eq!(reason(&mut p, t, b""), IgnoreReason::EmptyPayload);
        assert_eq!(reason(&mut p, t, b"  \n"), IgnoreReason::EmptyPayload);
        assert_eq!(
            reason(&mut p, t, &[0xff, 0xfe, 0x00]),
            IgnoreReason::InvalidUtf8
        );
        let huge = vec![b'9'; 1_000_000];
        assert_eq!(reason(&mut p, t, &huge), IgnoreReason::NonNumericPayload);
    }

    #[test]
    fn command_response_and_status_topics_never_become_readings() {
        let mut p = parser();
        assert_eq!(
            reason(
                &mut p,
                "solar_assistant/inverter_1/capacity_point_1/set",
                b"15"
            ),
            IgnoreReason::CommandTopic
        );
        assert_eq!(
            reason(
                &mut p,
                "solar_assistant/inverter_1/capacity_point_1/set_verified",
                b"15"
            ),
            IgnoreReason::CommandTopic
        );
        assert_eq!(
            reason(
                &mut p,
                "solar_assistant/set/response_message/state",
                b"Saved"
            ),
            IgnoreReason::SetResponse
        );
        // Even a numeric reply is not a measurement.
        assert_eq!(
            reason(&mut p, "solar_assistant/set/response_message/state", b"1"),
            IgnoreReason::SetResponse
        );
        assert_eq!(
            reason(&mut p, "solar_assistant/status", b"online"),
            IgnoreReason::StatusTopic
        );
    }

    #[test]
    fn unknown_and_hostile_topics_are_ignored() {
        let mut p = parser();
        assert_eq!(
            reason(&mut p, "zigbee2mqtt/lamp", b"1"),
            IgnoreReason::OutsidePrefix
        );
        assert_eq!(
            reason(&mut p, "solar_assistant/inverter_1/pv_power", b"1"),
            IgnoreReason::UnknownTopicShape
        );
        assert_eq!(
            reason(&mut p, "solar_assistant/inv 1; drop/pv_power/state", b"1"),
            IgnoreReason::UnusableName
        );
        assert_eq!(
            reason(&mut p, "solar_assistant/inverter_1/pv\npower/state", b"1"),
            IgnoreReason::UnusableName
        );
        let long = format!("solar_assistant/{}/pv_power/state", "a".repeat(600));
        assert_eq!(reason(&mut p, &long, b"1"), IgnoreReason::TopicTooLong);
        assert_eq!(p.stats().messages, 5);
        assert_eq!(p.stats().ignored_total(), 5);
    }

    #[test]
    fn a_declared_unit_overrides_the_guess() {
        let mut p = parser();
        // Without discovery, temperature has no unit.
        let r = reading(
            &mut p,
            "solar_assistant/inverter_1/temperature/state",
            b"33.9",
        );
        assert_eq!(r.unit, None);
        assert_eq!(r.unit_basis, UnitBasis::Unknown);

        let cfg = r#"{"state_topic":"solar_assistant/inverter_1/temperature/state","unit_of_measurement":"°C","device_class":"temperature"}"#.as_bytes();
        assert_eq!(
            p.handle(&msg(
                "homeassistant/sensor/sa_inverter_1/temperature/config",
                cfg
            )),
            Outcome::Discovery(DiscoveryChange::Stored)
        );
        let r = reading(
            &mut p,
            "solar_assistant/inverter_1/temperature/state",
            b"33.9",
        );
        assert_eq!(r.unit.as_ref().map(Unit::as_str), Some("°C"));
        assert_eq!(r.unit_basis, UnitBasis::DeclaredBySource);
    }

    #[test]
    fn a_declaration_without_a_unit_stops_the_guess() {
        let mut p = parser();
        let cfg = br#"{"state_topic":"solar_assistant/inverter_1/pv_power/state","unit_of_measurement":null}"#;
        p.handle(&msg("homeassistant/sensor/x/pv_power/config", cfg));
        let r = reading(&mut p, "solar_assistant/inverter_1/pv_power/state", b"10");
        assert_eq!(r.unit, None);
        assert_eq!(r.unit_basis, UnitBasis::Unknown);
    }

    #[test]
    fn an_empty_config_withdraws_a_declaration() {
        let mut p = parser();
        let topic = "homeassistant/sensor/x/pv_power/config";
        let cfg = br#"{"state_topic":"solar_assistant/inverter_1/pv_power/state","unit_of_measurement":"kW"}"#;
        p.handle(&msg(topic, cfg));
        assert_eq!(p.declaration_count(), 1);
        assert_eq!(
            p.handle(&msg(topic, b"")),
            Outcome::Discovery(DiscoveryChange::Removed)
        );
        assert_eq!(p.declaration_count(), 0);
        let r = reading(&mut p, "solar_assistant/inverter_1/pv_power/state", b"10");
        assert_eq!(r.unit_basis, UnitBasis::InferredFromName);
    }

    #[test]
    fn discovery_for_other_devices_is_not_stored() {
        let mut p = parser();
        assert_eq!(
            reason(
                &mut p,
                "homeassistant/sensor/kettle/temp/config",
                br#"{"state_topic":"kitchen/kettle/temp","unit_of_measurement":"W"}"#
            ),
            IgnoreReason::DiscoveryUnrelated
        );
        assert_eq!(
            reason(
                &mut p,
                "homeassistant/sensor/x/y/config",
                br#"{"state_topic":"solar_assistant/inverter_1/x/set"}"#
            ),
            IgnoreReason::DiscoveryUnrelated
        );
        assert_eq!(
            reason(&mut p, "homeassistant/sensor/x/y/config", b"{nope"),
            IgnoreReason::DiscoveryMalformed
        );
        assert_eq!(p.declaration_count(), 0);
    }

    #[test]
    fn the_declaration_store_is_bounded() {
        let mut p = parser();
        for i in 0..(MAX_DECLARATIONS + 10) {
            let cfg = format!(
                r#"{{"state_topic":"solar_assistant/inverter_1/v{i}/state","unit_of_measurement":"V"}}"#
            );
            p.handle(&msg(
                &format!("homeassistant/sensor/x/v{i}/config"),
                cfg.as_bytes(),
            ));
        }
        assert_eq!(p.declaration_count(), MAX_DECLARATIONS);
        assert_eq!(p.stats().ignored[&IgnoreReason::DiscoveryStoreFull], 10);
    }

    #[test]
    fn a_custom_prefix_is_honoured() {
        let mut cfg = ParserConfig::new("solar-assistant.local:1883");
        cfg.topic_prefix = "solar_assistant_8kw".to_owned();
        let mut p = Parser::new(cfg).unwrap();
        let r = reading(
            &mut p,
            "solar_assistant_8kw/inverter_1/pv_power_2/state",
            b"663",
        );
        assert_eq!(r.source_id.as_str(), "solar_assistant_8kw/inverter_1");
        assert_eq!(
            reason(
                &mut p,
                "solar_assistant/inverter_1/pv_power_2/state",
                b"663"
            ),
            IgnoreReason::OutsidePrefix
        );
    }

    #[test]
    fn bad_configs_are_rejected() {
        for bad in [
            "",
            "/x",
            "x/",
            "a//b",
            "solar_assistant/#",
            "+",
            "has space",
        ] {
            let mut cfg = ParserConfig::new("solar-assistant.local:1883");
            cfg.topic_prefix = bad.to_owned();
            assert!(Parser::new(cfg).is_err(), "{bad:?}");
        }
        assert_eq!(
            Parser::new(ParserConfig::new("user:secret@solar-assistant.local:1883")).err(),
            Some(ConfigError::BadBroker)
        );
        assert_eq!(
            Parser::new(ParserConfig::new("")).err(),
            Some(ConfigError::BadBroker)
        );
    }

    #[test]
    fn stats_add_up() {
        let mut p = parser();
        reading(&mut p, "solar_assistant/inverter_1/pv_power/state", b"1");
        reason(
            &mut p,
            "solar_assistant/inverter_1/device_mode/state",
            b"Grid",
        );
        reason(&mut p, "elsewhere", b"1");
        let s = p.stats();
        assert_eq!(s.messages, 3);
        assert_eq!(
            s.readings + s.discovery_changes + s.ignored_total(),
            s.messages
        );
    }
}
