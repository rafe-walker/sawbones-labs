# sawbones-solar-assistant

A read-only tap on the MQTT output of a [Solar Assistant](https://solar-assistant.io/) unit. It turns each numeric message into a typed reading that says where it came from, and writes one JSON object per line.

**Status: provisional, and not yet run against a real Solar Assistant unit.** It was written from Solar Assistant's public documentation and from topic listings other users have posted. The tests use a synthetic fixture and a stub broker inside the test process. What was verified against documentation and what is assumed is listed in [`docs/research/solar-assistant-mqtt.md`](../../docs/research/solar-assistant-mqtt.md). The wider Sawbones design is still under review, so the reading format here may change.

## It cannot write

Solar Assistant accepts setting changes on topics ending in `/set` and `/set_verified`. This crate has no way to send to them, or to any topic:

- The MQTT client lives in a private field of `ReadOnlyMqtt`, which offers `subscribe` and `poll` and nothing else.
- No last-will message is set, so the broker never sends anything on the client's behalf.
- Subscriptions are QoS 0, so the client never owes an acknowledgement.
- A test scans the source for any call that could send a message, and another watches the wire against a stub broker.

Keep "Allow setting changes" switched off on the unit as well. That is its default.

## Build

```
cargo build --release
```

The binary is `target/release/sa-tap`.

## Use

Record. Readings go to standard output, or to a file with `--out`:

```
sa-tap record --host solar-assistant.local --port 1883 --out readings.jsonl
```

For a first run against a real unit, also keep every message exactly as received and take units from Home Assistant discovery if the unit publishes it:

```
sa-tap record --host solar-assistant.local \
  --discovery-prefix homeassistant \
  --raw-log raw.jsonl --out readings.jsonl
```

Stop with Ctrl-C. Counts of what was read and what was ignored, and why, are printed on exit. If the connection drops, the tap reconnects with a growing delay (1 s doubling to 60 s) and subscribes again.

If the broker needs a login, set both of these in the environment. There is no flag for them, on purpose, so they cannot land in shell history:

```
SA_TAP_MQTT_USERNAME=<username> SA_TAP_MQTT_PASSWORD=<password> sa-tap record ...
```

Summarise a recording: the series seen, counts, minimum and maximum, and gaps longer than a threshold:

```
sa-tap replay readings.jsonl --gap-secs 60
sa-tap replay --raw raw.jsonl        # parse a raw capture first
```

Watch the ratio between two inputs that see the same sky:

```
sa-tap ratio readings.jsonl \
  --a solar_assistant/inverter_1/pv_power \
  --b solar_assistant/inverter_2/pv_power \
  --min-power 200 --window-secs 600 --save-baseline baseline.json

sa-tap ratio later.jsonl \
  --a solar_assistant/inverter_1/pv_power \
  --b solar_assistant/inverter_2/pv_power \
  --baseline-file baseline.json
```

`--a` and `--b` are a state topic without its `/state` ending. Run `replay` first to see which series a recording holds.

## What a reading says

```json
{"schema":"sawbones.reading/0-provisional",
 "source_id":"solar_assistant/inverter_1","quantity":"pv_power",
 "value":{"text":"1823","parsed":1823.0},
 "unit":"W","unit_basis":"inferred_from_name",
 "received_at":1782064801001276516,"source_timestamp":null,
 "timestamp_quality":{"kind":"host_receive","uncertainty_ns":5000000000},
 "trust":"unverified",
 "provenance":{"transport":"mqtt","broker":"solar-assistant.local:1883",
  "topic":{"untrusted":"solar_assistant/inverter_1/pv_power/state","truncated":false},
  "raw_payload":{"untrusted":"1823","truncated":false},"retained":false}}
```

(Shown wrapped; in a file it is one line.)

- **`trust` is always `unverified`.** Solar Assistant is a third party's reading of the inverter, and this project has not yet cross-checked it against an instrument of its own.
- **`received_at` is when this host got the message.** MQTT carries no measurement time. `uncertainty_ns` is a stated bound on how stale the value may be, five seconds unless `--timestamp-uncertainty-ms` says otherwise. It is an assumption, not a measurement. A retained message is marked `host_receive_retained`: its age is unknown.
- **`unit_basis` says how the unit was arrived at**: `declared_by_source` (a discovery config said so), `inferred_from_name` (a guess from the topic name) or `unknown`. Temperature is never guessed, because the unit has a Celsius/Fahrenheit setting.
- **Strings from outside are labelled.** Topic and payload text sit under an `untrusted` key. In Rust they are `WorldText`, which has no `Display` and must be rendered with `fenced()`.

## What is ignored

Anything that is not a number on a `<prefix>/<device>/<name>/state` topic is returned by the parser as a typed "ignored" result and counted, never dropped silently and never a panic: text values such as `Hybrid`, empty payloads, command topics, the unit's set-response and status topics, topics of an unknown shape, and discovery configs that are damaged or about other devices.

## The ratio monitor reports observations, not diagnoses

Two arrays can differ in panel count, tilt, orientation, shading and clipping, and all of that changes through the day and the year. So the monitor says "ratio moved by X% from baseline" and nothing more. It never says "panel fault". What a drift means is for a person, or a further measurement, to decide.

## Tests

```
cargo test
```

No broker, network or hardware is needed. The fixture in [`tests/fixtures/`](tests/fixtures/README.md) is synthetic.
