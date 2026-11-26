# Solar Assistant's MQTT interface: what is documented, what is assumed

Research for rung 0, the read-only ingest in `crates/sawbones-solar-assistant`. Done on 25 November 2026 from public web pages only. **Nothing here has been checked against a real Solar Assistant unit.** Everything is paraphrased; topic strings are given exactly because they are identifiers.

Three levels of confidence are used:

- **Documented**: stated on Solar Assistant's own help pages, or on Home Assistant's for the discovery format.
- **Seen in the wild**: shown in listings or configs other users posted on forums. Real, but from other people's inverters and firmware, and not a specification.
- **Assumed**: not found anywhere. The code picks a behaviour and says so.

## Sources

| Ref | Page | Kind |
| --- | --- | --- |
| SA-MQTT | <https://solar-assistant.io/help/integration/mqtt> | Solar Assistant help: MQTT |
| SA-HA | <https://solar-assistant.io/help/home-assistant/setup> | Solar Assistant help: Home Assistant over MQTT |
| SA-BRIDGE | <https://solar-assistant.io/help/home-assistant/broker-setup> | Solar Assistant help: Home Assistant broker with bridge |
| HA-MQTT | <https://www.home-assistant.io/integrations/mqtt/> | Home Assistant docs: MQTT discovery format |
| F-PF | <https://powerforum.co.za/topic/27902-dont-see-entities-and-devices-after-connecting-ha-to-sa/> | Forum post with a `mosquitto_sub` listing |
| F-INFLUX | <https://community.influxdata.com/t/help-getting-mqtt-feed-from-solar-assistant-via-mqtt-talegraf-to-influxdb/29256> | Forum post with a Telegraf listing |
| F-HA-RENAME | <https://community.home-assistant.io/t/dynamically-rename-mqtt-sensors/546903> | Forum post showing two discovery configs |
| F-HA-BRIDGE | <https://community.home-assistant.io/t/solar-assistant-setup-with-mosquitto-as-broker-bridge-not-changing-settings/477138> | Forum post showing a discovery config |
| F-HA-MODE | <https://community.home-assistant.io/t/mqtt-sensor-unknown-solar-assistant-device-mode/584211> | Forum post on a text-valued topic |
| F-NODERED | <https://discourse.nodered.org/t/solar-assistant-mqtt-how-to-use-it-with-node-red/67281?page=2> | Forum post on set and response topics |
| GBB | <https://krzysztofhajdamowicz.github.io/rewrite-GbbOptimizer-manual/en/integrations/home-assistant/solar-assistant/> | Third-party product manual listing topics it reads |

The pages were read through a fetch tool that summarises, not as raw HTML. Where an exact string matters (a topic, a field name) it was asked for verbatim, but a transcription slip is possible. That is one more reason to check against a real unit.

## Connecting

| Point | Finding | Level | Source |
| --- | --- | --- | --- |
| Where MQTT is switched on | Started from the unit's Configuration tab. | Documented | SA-MQTT |
| Host | The unit's own address; `solar-assistant.local` where the network supports mDNS. | Documented | SA-MQTT |
| Port | The page's examples all use 1883. No other port, and no TLS, is mentioned. | Documented (by example) | SA-MQTT |
| Authentication | Optional. The bridge example carries commented-out username and password lines, to be enabled if authentication was configured on the unit. | Documented | SA-MQTT, SA-BRIDGE |
| Topic prefix | `solar_assistant` by default, changeable in the unit's MQTT broker settings. Users run prefixes such as `solar_assistant_8kw`. | Documented; seen in the wild | SA-MQTT, F-HA-RENAME |
| MQTT version, QoS used by the unit, session behaviour | Not stated. | Assumed: MQTT 3.1.1, and this client subscribes at QoS 0 | |

## Topic layout

| Topic | Meaning | Level | Source |
| --- | --- | --- | --- |
| `solar_assistant/inverter_1/load_power/state` | The one value topic the help page shows. | Documented | SA-MQTT |
| `solar_assistant/+/+/state` | The bridge example's inbound filter for readings. This fixes the shape: prefix, one device segment, one name segment, then `state`. | Documented | SA-MQTT, SA-BRIDGE |
| `solar_assistant/status` | Bridged inbound alongside the readings; presented as the unit's status. Its payload is not shown. | Documented (topic), assumed (payload) | SA-BRIDGE |
| `solar_assistant/total/...` | A second device segment, for system totals: `battery_power`, `battery_state_of_charge`, `battery_temperature`, and energy counters such as `pv_energy`, `load_energy`, `grid_energy_in`, `grid_energy_out`. | Seen in the wild | F-PF, F-INFLUX, GBB |
| `solar_assistant/inverter_1/<name>/state` names | `pv_power`, `pv_power_1`, `pv_power_2`, `pv_voltage_1`, `pv_current_1`, `load_power`, `grid_power`, `grid_voltage`, `grid_frequency`, `battery_voltage`, `battery_current`, `temperature`, `device_mode`, `output_source_priority`, `max_charge_current`, and many more. One name contains a hyphen: `load_power_non-essential`. The set differs by inverter make. | Seen in the wild | F-PF, F-INFLUX |
| `solar_assistant/inverter_2/...` | Not shown in any page loaded. | Assumed from the `inverter_1` numbering | |
| `solar_assistant/battery_1/...` | Not shown in any page loaded. | Assumed. The parser does not depend on it: any device segment is accepted | |

The parser treats the device and name segments as opaque identifiers. It does not need to know the list in advance, so an unfamiliar name still becomes a reading (with no unit if the name gives no clue).

## Payloads

| Point | Finding | Level | Source |
| --- | --- | --- | --- |
| Format | A bare number as text: `1823`, `-1550`, `53.2`, `0.0`. No JSON, no unit suffix. | Documented (no unit in payload); seen in the wild (examples) | SA-MQTT, F-PF, F-INFLUX |
| Negative values | Present, for example grid power while exporting. | Seen in the wild | F-INFLUX, F-PF |
| Text values | Some `state` topics carry words, not numbers: `Hybrid`, `Grid`, `Solar/Battery`, `Load first`, `Disabled`. | Seen in the wild | F-PF, F-INFLUX, F-HA-MODE |
| Timestamps | None. The payload is only the value. | Documented (by the payload format) | SA-MQTT |

Text-valued topics are not readings in this rung. The parser returns them as a typed "ignored: non-numeric payload" result and counts them.

## Units

| Point | Finding | Level | Source |
| --- | --- | --- | --- |
| Which units | Values are sent in the units the unit's own screens show; watts, volts and amps are the examples given. | Documented | SA-MQTT |
| Temperature | Follows the Celsius/Fahrenheit setting under Configuration, System. | Documented | SA-MQTT |
| Machine-readable units | With Home Assistant discovery enabled, each value's unit is advertised as `unit_of_measurement` in its discovery config. | Documented | SA-MQTT |
| Energy counters | Kilowatt-hours. | Seen in the wild (third-party manual) | GBB |
| A table of names to units | Not published. | Assumed: see below | |

What the code does: if a discovery config has declared a unit for a topic, that unit is used and the reading says `declared_by_source`. Otherwise the unit is guessed from the name (`power` gives W, `voltage` V, `current` A, `frequency` Hz, `state_of_charge` %, `energy` kWh) and the reading says `inferred_from_name`. Temperature is never guessed, because the name cannot say which scale is in use. Anything else gets no unit.

## Home Assistant discovery

| Point | Finding | Level | Source |
| --- | --- | --- | --- |
| Is it published? | Only if "Home Assistant discovery" is enabled on the Configuration tab. | Documented | SA-HA |
| Retained? | Yes: the page describes the discovery messages as retained. | Documented | SA-HA |
| Where | Under `homeassistant/#` (the bridge example's inbound filter). | Documented | SA-MQTT, SA-BRIDGE |
| Exact topic path | Home Assistant's format is `<prefix>/<component>/[<node>/]<object>/config`, default prefix `homeassistant`. The exact component and object ids Solar Assistant uses were not found. | Documented (format); assumed (ids) | HA-MQTT |
| Payload fields | Posted configs show `state_topic`, `name`, `uniq_id`, `unit_of_measurement`, `device_class`, and a `device` object with `ids`, `mf`, `mdl`, `name`. Settings also carry `command_topic`. | Seen in the wild | F-HA-RENAME, F-HA-BRIDGE |
| Missing units | Text-valued topics have been posted with `unit_of_measurement` set to null. | Seen in the wild | F-HA-BRIDGE, F-HA-MODE |
| Short field names, `~` shorthand, empty payload removes | Part of Home Assistant's format. Whether Solar Assistant uses the short names for anything beyond `uniq_id` and the `device` keys is unknown. | Documented (format) | HA-MQTT |

What the code does: it reads `state_topic`, `unit_of_measurement`, `device_class` and the presence of `command_topic` (and their documented short forms), keeps only declarations whose state topic is under the Solar Assistant prefix, and never relies on the config topic's own path. It does not handle Home Assistant's newer one-message-per-device form; such a message is counted as an unreadable discovery config.

## Update cadence

Not documented. One user's log shows messages arriving roughly 0.6 to 1.3 seconds apart across topics (F-INFLUX); another reports a setting's state taking minutes to change (F-NODERED). Whether the unit publishes every value on every poll or only on change was not found.

What the code does: it assumes nothing about cadence. Each reading's timestamp is this host's receive time, with a stated bound on how stale the value may be (default five seconds, an assumption the operator can change). A reading that arrives as a retained message is marked as being of unknown age. `sa-tap replay` reports gaps so the real cadence can be read off a capture.

## Command topics: what a read-only client must never send to

| Topic | Meaning | Level | Source |
| --- | --- | --- | --- |
| `solar_assistant/<device>/<name>/set` | Changes a setting, for example `solar_assistant/inverter_1/output_source_priority/set`. | Documented | SA-MQTT |
| `solar_assistant/<device>/<name>/set_verified` | Changes a setting, reads it back, and retries up to three times. | Documented | SA-MQTT |
| `solar_assistant/set/response_message/state` | Where the unit answers a setting change. A reply, not a command and not a measurement. | Documented | SA-MQTT |
| Any `command_topic` named in a discovery config | The same `/set` topics, listed per setting. | Documented | SA-MQTT |

Two further documented facts matter here. Setting changes are off by default and must be enabled on the unit ("Allow setting changes"); while off, the unit rejects them. And the bridge example forwards only the `set` and `set_verified` topics outbound, which suggests those two suffixes are the whole write surface. That is an inference from an example, not a statement by the vendor.

What the code does: it cannot send an application message to any topic at all. The MQTT client is wrapped in a type that exposes subscribe and receive only. It sets no last-will message and subscribes at QoS 0, so it never sends an acknowledgement either. If a command message from some other client passes by, the parser classes it as a command topic and ignores it. Leave "Allow setting changes" off on the unit regardless.

## To check on a real unit

In rough order of how much rests on each:

1. Capture everything once: `sa-tap record --discovery-prefix homeassistant --raw-log raw.jsonl --out readings.jsonl`. The raw log can be parsed again as the parser improves.
2. Is "Allow setting changes" off? It should be.
3. The device segments actually present. Is the second inverter `inverter_2`? Do batteries appear as `battery_1` and so on, or only under `total`?
4. The value names for this inverter model. Is PV power `pv_power`, or `pv_power_1` and `pv_power_2`, or both? The ratio monitor needs the right pair.
5. Does `replay --raw` show any `unknown_topic_shape` or `unusable_name` counts? Each one is a topic the layout above does not explain.
6. Are discovery configs published, under which topic paths, and do their fields match the list above? Is a unit declared for every numeric value?
7. Do the guessed units agree with the declared ones wherever both exist? In particular the energy counters (kWh assumed) and anything left without a unit.
8. Which temperature scale is the unit set to?
9. The real cadence, per topic, and whether quiet topics are quiet because values did not change.
10. Are state messages retained? (Do values arrive in a burst at the moment of subscribing?)
11. What `solar_assistant/status` carries.
12. Whether the broker needs a username and password, and whether it has any access rules that refuse a subscription.
