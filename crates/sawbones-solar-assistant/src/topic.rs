//! Classifying an MQTT topic. Pure string work, no I/O.
//!
//! The layout this module knows (see `docs/research/solar-assistant-mqtt.md`
//! for which parts are documented and which are assumed):
//!
//! | Topic | Class |
//! | --- | --- |
//! | `<prefix>/<device>/<name>/state` | [`TopicClass::State`] |
//! | anything ending in `/set` or `/set_verified` | [`TopicClass::Command`] |
//! | `<prefix>/set/response_message/state` | [`TopicClass::SetResponse`] |
//! | `<prefix>/status` | [`TopicClass::Status`] |
//! | `<discovery prefix>/…/config` | [`TopicClass::DiscoveryConfig`] |

/// What kind of topic a string is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopicClass<'a> {
    /// A value topic: `<prefix>/<device>/<name>/state`.
    State {
        /// The device segment, for example `inverter_1` or `total`.
        device: &'a str,
        /// The value's name, for example `pv_power`.
        name: &'a str,
    },
    /// A command topic. A read-only client must never send to one. Seeing a
    /// message on one means some *other* client changed a setting.
    Command,
    /// Solar Assistant's reply to a setting change.
    SetResponse,
    /// Solar Assistant's own status topic.
    Status,
    /// A Home Assistant discovery config message.
    DiscoveryConfig,
    /// Under neither the Solar Assistant prefix nor the discovery prefix.
    OutsidePrefix,
    /// Under the Solar Assistant prefix but not a shape this module knows.
    UnknownShape,
}

/// Whether a topic is a command topic: its last segment is `set` or
/// `set_verified`. This holds whatever the prefix is.
pub fn is_command_topic(topic: &str) -> bool {
    matches!(topic.rsplit('/').next(), Some("set" | "set_verified"))
}

/// Strips `prefix` plus the following `/` from `topic`.
fn strip_topic_prefix<'a>(topic: &'a str, prefix: &str) -> Option<&'a str> {
    topic.strip_prefix(prefix)?.strip_prefix('/')
}

/// Classifies `topic`. `prefix` is the Solar Assistant topic prefix (which
/// may itself contain `/`); `discovery_prefix` is the Home Assistant
/// discovery prefix, or `None` to treat discovery topics as outside.
pub fn classify<'a>(
    topic: &'a str,
    prefix: &str,
    discovery_prefix: Option<&str>,
) -> TopicClass<'a> {
    if let Some(rest) = strip_topic_prefix(topic, prefix) {
        // Commands first: nothing that ends in a command suffix is ever
        // treated as data, whatever else it looks like.
        if is_command_topic(topic) {
            return TopicClass::Command;
        }
        let mut parts = rest.split('/');
        let segs = [parts.next(), parts.next(), parts.next(), parts.next()];
        return match segs {
            [Some("status"), None, None, None] => TopicClass::Status,
            [Some("set"), Some("response_message"), Some("state"), None] => TopicClass::SetResponse,
            [Some(device), Some(name), Some("state"), None]
                if !device.is_empty() && !name.is_empty() =>
            {
                TopicClass::State { device, name }
            }
            _ => TopicClass::UnknownShape,
        };
    }
    if let Some(dp) = discovery_prefix {
        if let Some(rest) = strip_topic_prefix(topic, dp) {
            return if rest.rsplit('/').next() == Some("config") && rest.contains('/') {
                TopicClass::DiscoveryConfig
            } else {
                TopicClass::OutsidePrefix
            };
        }
    }
    TopicClass::OutsidePrefix
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: &str = "solar_assistant";
    const D: Option<&str> = Some("homeassistant");

    #[test]
    fn state_topics() {
        assert_eq!(
            classify("solar_assistant/inverter_1/pv_power/state", P, D),
            TopicClass::State {
                device: "inverter_1",
                name: "pv_power"
            }
        );
        assert_eq!(
            classify("solar_assistant/total/battery_state_of_charge/state", P, D),
            TopicClass::State {
                device: "total",
                name: "battery_state_of_charge"
            }
        );
    }

    #[test]
    fn command_topics_are_never_state() {
        for t in [
            "solar_assistant/inverter_1/output_source_priority/set",
            "solar_assistant/inverter_1/capacity_point_1/set_verified",
            "solar_assistant/set",
            "solar_assistant/a/b/c/d/set",
        ] {
            assert_eq!(classify(t, P, D), TopicClass::Command, "{t}");
            assert!(is_command_topic(t));
        }
        assert!(!is_command_topic(
            "solar_assistant/inverter_1/pv_power/state"
        ));
        // A value whose *name* merely contains "set" is not a command.
        assert!(!is_command_topic("solar_assistant/inverter_1/offset/state"));
    }

    #[test]
    fn response_and_status_topics() {
        assert_eq!(
            classify("solar_assistant/set/response_message/state", P, D),
            TopicClass::SetResponse
        );
        assert_eq!(classify("solar_assistant/status", P, D), TopicClass::Status);
    }

    #[test]
    fn discovery_topics() {
        assert_eq!(
            classify("homeassistant/sensor/sa_inverter_1/pv_power/config", P, D),
            TopicClass::DiscoveryConfig
        );
        assert_eq!(
            classify("homeassistant/sensor/pv_power/config", P, D),
            TopicClass::DiscoveryConfig
        );
        assert_eq!(
            classify("homeassistant/status", P, D),
            TopicClass::OutsidePrefix
        );
        assert_eq!(
            classify("homeassistant/sensor/x/config", P, None),
            TopicClass::OutsidePrefix
        );
    }

    #[test]
    fn odd_shapes_do_not_panic_and_are_not_state() {
        for t in [
            "",
            "/",
            "solar_assistant",
            "solar_assistant/",
            "solar_assistant//",
            "solar_assistant///state",
            "solar_assistant/inverter_1/pv_power",
            "solar_assistant/inverter_1/pv_power/state/extra",
            "solar_assistant/inverter_1//state",
            "solar_assistantx/inverter_1/pv_power/state",
            "other/inverter_1/pv_power/state",
        ] {
            assert!(
                !matches!(classify(t, P, D), TopicClass::State { .. }),
                "{t:?} was classed as state"
            );
        }
        assert_eq!(
            classify("solar_assistantx/inverter_1/pv_power/state", P, D),
            TopicClass::OutsidePrefix
        );
        assert_eq!(
            classify("solar_assistant/inverter_1/pv_power", P, D),
            TopicClass::UnknownShape
        );
    }

    #[test]
    fn a_prefix_containing_slashes_works() {
        assert_eq!(
            classify(
                "site-a/solar_assistant/total/pv_energy/state",
                "site-a/solar_assistant",
                D
            ),
            TopicClass::State {
                device: "total",
                name: "pv_energy"
            }
        );
    }
}
