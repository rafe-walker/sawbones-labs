//! Reading Home Assistant MQTT discovery config messages. Pure, no I/O.
//!
//! When its "Home Assistant discovery" option is on, Solar Assistant
//! publishes retained config messages describing each value. This module
//! reads the few fields the parser can use: which state topic a message
//! describes, the unit it declares, its device class, and whether it also
//! names a command topic.
//!
//! The field names follow Home Assistant's MQTT discovery format, including
//! its documented short forms (`stat_t`, `unit_of_meas`, `dev_cla`, `cmd_t`)
//! and the `~` base-topic shorthand. Solar Assistant's own documentation
//! confirms only that `unit_of_measurement` is present; the rest of the
//! payload shape here comes from configs users have posted, and is an
//! assumption until checked on a real unit.

use sawbones_reading::WorldText;
use thiserror::Error;

/// Largest discovery payload this module will parse.
pub const MAX_CONFIG_BYTES: usize = 64 * 1024;

/// Longest state topic accepted from a discovery payload.
pub const MAX_TOPIC_BYTES: usize = 512;

/// The unit a discovery message declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredUnit {
    /// A unit from this crate's table of known symbols.
    Recognised(&'static str),
    /// A unit string this crate does not recognise. Kept as outside text for
    /// inspection; never copied into a reading.
    Unrecognised(WorldText),
    /// No unit, or an explicit `null`.
    Absent,
}

/// What one discovery config message says about one value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorDeclaration {
    /// The state topic the declaration describes.
    pub state_topic: String,
    /// The declared unit.
    pub unit: DeclaredUnit,
    /// The declared device class (for example `power`), as outside text.
    pub device_class: Option<WorldText>,
    /// Whether the config also names a command topic, meaning the value is a
    /// setting that some client could change. This crate never does.
    pub has_command_topic: bool,
}

/// Why a discovery payload could not be used.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DiscoveryError {
    /// The payload was larger than [`MAX_CONFIG_BYTES`].
    #[error("discovery payload is larger than {MAX_CONFIG_BYTES} bytes")]
    TooLarge,
    /// The payload was not valid JSON.
    #[error("discovery payload is not valid JSON")]
    NotJson,
    /// The payload was JSON but not an object.
    #[error("discovery payload is not a JSON object")]
    NotAnObject,
    /// The payload named no state topic.
    #[error("discovery payload names no state topic")]
    NoStateTopic,
    /// The state topic was longer than [`MAX_TOPIC_BYTES`].
    #[error("discovery state topic is longer than {MAX_TOPIC_BYTES} bytes")]
    StateTopicTooLong,
}

fn get_str<'a>(
    obj: &'a serde_json::Map<String, serde_json::Value>,
    long: &str,
    short: &str,
) -> Option<&'a str> {
    obj.get(long).or_else(|| obj.get(short))?.as_str()
}

/// Applies Home Assistant's `~` base-topic shorthand to a topic value.
fn expand_base(value: &str, base: Option<&str>) -> String {
    match base {
        Some(base) => {
            if let Some(rest) = value.strip_prefix('~') {
                format!("{base}{rest}")
            } else if let Some(rest) = value.strip_suffix('~') {
                format!("{rest}{base}")
            } else {
                value.to_owned()
            }
        }
        None => value.to_owned(),
    }
}

/// Parses one discovery config payload.
pub fn parse_config(payload: &[u8]) -> Result<SensorDeclaration, DiscoveryError> {
    if payload.len() > MAX_CONFIG_BYTES {
        return Err(DiscoveryError::TooLarge);
    }
    let value: serde_json::Value =
        serde_json::from_slice(payload).map_err(|_| DiscoveryError::NotJson)?;
    let obj = value.as_object().ok_or(DiscoveryError::NotAnObject)?;
    let base = obj.get("~").and_then(|v| v.as_str());

    let state_topic = get_str(obj, "state_topic", "stat_t").ok_or(DiscoveryError::NoStateTopic)?;
    let state_topic = expand_base(state_topic, base);
    if state_topic.len() > MAX_TOPIC_BYTES {
        return Err(DiscoveryError::StateTopicTooLong);
    }

    let unit = match get_str(obj, "unit_of_measurement", "unit_of_meas") {
        None | Some("") => DeclaredUnit::Absent,
        Some(u) => match crate::units::canonical_unit(u) {
            Some(known) => DeclaredUnit::Recognised(known),
            None => DeclaredUnit::Unrecognised(WorldText::new(u, 64)),
        },
    };
    let device_class = get_str(obj, "device_class", "dev_cla").map(|s| WorldText::new(s, 64));
    let has_command_topic = obj.contains_key("command_topic") || obj.contains_key("cmd_t");

    Ok(SensorDeclaration {
        state_topic,
        unit,
        device_class,
        has_command_topic,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_sensor_config() {
        let d = parse_config(
            br#"{"device":{"ids":["sa_inverter_1"],"mf":"Example","name":"Example inverter"},
                "device_class":"power","name":"PV power",
                "state_topic":"solar_assistant/inverter_1/pv_power/state",
                "uniq_id":"inverter_1_pv_power","unit_of_measurement":"W"}"#,
        )
        .unwrap();
        assert_eq!(d.state_topic, "solar_assistant/inverter_1/pv_power/state");
        assert_eq!(d.unit, DeclaredUnit::Recognised("W"));
        assert_eq!(d.device_class.unwrap().as_untrusted_str(), "power");
        assert!(!d.has_command_topic);
    }

    #[test]
    fn reads_short_field_names_and_the_base_shorthand() {
        let d = parse_config(
            br#"{"~":"solar_assistant/inverter_1/max_charge_current",
                "stat_t":"~/state","cmd_t":"~/set","unit_of_meas":"A","dev_cla":"current"}"#,
        )
        .unwrap();
        assert_eq!(
            d.state_topic,
            "solar_assistant/inverter_1/max_charge_current/state"
        );
        assert_eq!(d.unit, DeclaredUnit::Recognised("A"));
        assert!(d.has_command_topic);
    }

    #[test]
    fn null_and_missing_units_are_absent() {
        let d = parse_config(
            br#"{"state_topic":"solar_assistant/inverter_1/device_mode/state","unit_of_measurement":null}"#,
        )
        .unwrap();
        assert_eq!(d.unit, DeclaredUnit::Absent);
        let d = parse_config(br#"{"state_topic":"solar_assistant/inverter_1/device_mode/state"}"#)
            .unwrap();
        assert_eq!(d.unit, DeclaredUnit::Absent);
    }

    #[test]
    fn unknown_units_are_kept_apart() {
        let d = parse_config(
            br#"{"state_topic":"solar_assistant/x/y/state","unit_of_measurement":"furlongs"}"#,
        )
        .unwrap();
        assert!(matches!(d.unit, DeclaredUnit::Unrecognised(_)));
    }

    #[test]
    fn malformed_payloads_are_errors_not_panics() {
        assert_eq!(parse_config(b""), Err(DiscoveryError::NotJson));
        assert_eq!(parse_config(b"not json"), Err(DiscoveryError::NotJson));
        assert_eq!(parse_config(b"[1,2]"), Err(DiscoveryError::NotAnObject));
        assert_eq!(parse_config(b"{}"), Err(DiscoveryError::NoStateTopic));
        assert_eq!(
            parse_config(br#"{"state_topic":42}"#),
            Err(DiscoveryError::NoStateTopic)
        );
        assert_eq!(parse_config(&[0xff, 0xfe]), Err(DiscoveryError::NotJson));
        let big = vec![b' '; MAX_CONFIG_BYTES + 1];
        assert_eq!(parse_config(&big), Err(DiscoveryError::TooLarge));
        let long = format!(r#"{{"state_topic":"{}"}}"#, "a".repeat(MAX_TOPIC_BYTES + 1));
        assert_eq!(
            parse_config(long.as_bytes()),
            Err(DiscoveryError::StateTopicTooLong)
        );
    }

    #[test]
    fn deeply_nested_json_does_not_overflow_the_stack() {
        let deep = "[".repeat(100_000);
        assert_eq!(parse_config(deep.as_bytes()), Err(DiscoveryError::TooLarge));
        let deep = "[".repeat(10_000);
        assert_eq!(parse_config(deep.as_bytes()), Err(DiscoveryError::NotJson));
    }
}
