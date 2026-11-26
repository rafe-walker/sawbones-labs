//! Runs `sa-tap record` against a stub that speaks just enough MQTT 3.1.1 to
//! stand in for a broker, on the loopback address, inside this test. No real
//! broker, no network, no Solar Assistant unit.
//!
//! It checks the things the pure tests cannot: that the client connects,
//! subscribes, turns received messages into JSON lines, reconnects and
//! subscribes again after the connection drops, and above all that every
//! packet it puts on the wire is a connect, a subscribe or a ping. Never an
//! application message.

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use sawbones_reading::{Reading, TimestampQuality, Trust};

const CONNECT: u8 = 1;
const PUBLISH: u8 = 3;
const SUBSCRIBE: u8 = 8;
const PINGREQ: u8 = 12;
const DISCONNECT: u8 = 14;

const STEP_TIMEOUT: Duration = Duration::from_secs(20);

struct Packet {
    kind: u8,
    body: Vec<u8>,
}

/// Reads one MQTT control packet.
fn read_packet(stream: &mut TcpStream) -> Packet {
    let mut first = [0u8; 1];
    stream.read_exact(&mut first).expect("a packet header");
    let (mut len, mut shift) = (0usize, 0u32);
    loop {
        let mut b = [0u8; 1];
        stream.read_exact(&mut b).expect("a packet length byte");
        len |= usize::from(b[0] & 0x7f) << shift;
        if b[0] & 0x80 == 0 {
            break;
        }
        shift += 7;
        assert!(shift <= 21, "malformed packet length");
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).expect("a packet body");
    Packet {
        kind: first[0] >> 4,
        body,
    }
}

/// Encodes a QoS 0 application message from the stub to the client.
fn publish_packet(topic: &str, payload: &[u8], retain: bool) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&u16::try_from(topic.len()).unwrap().to_be_bytes());
    body.extend_from_slice(topic.as_bytes());
    body.extend_from_slice(payload);
    assert!(body.len() < 128, "the stub only encodes short packets");
    let mut packet = vec![(PUBLISH << 4) | u8::from(retain), body.len() as u8];
    packet.extend_from_slice(&body);
    packet
}

/// Accepts one client, completes the handshake, and returns the stream and
/// the kinds of packet the client sent during it.
fn accept_and_handshake(
    listener: &TcpListener,
    expect_subscriptions: usize,
) -> (TcpStream, Vec<u8>) {
    listener.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + STEP_TIMEOUT;
    let mut stream = loop {
        match listener.accept() {
            Ok((s, _)) => break s,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "the client never connected");
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => panic!("accept failed: {e}"),
        }
    };
    stream.set_nonblocking(false).unwrap();
    stream.set_read_timeout(Some(STEP_TIMEOUT)).unwrap();
    let mut seen = Vec::new();

    let connect = read_packet(&mut stream);
    assert_eq!(connect.kind, CONNECT);
    seen.push(connect.kind);
    // Variable header: "MQTT" (2+4 bytes), level, connect flags.
    assert_eq!(&connect.body[2..6], b"MQTT");
    let flags = connect.body[7];
    assert_eq!(
        flags & 0b0000_0100,
        0,
        "the client must not register a will"
    );
    assert_ne!(
        flags & 0b0000_0010,
        0,
        "the client must ask for a clean session"
    );
    assert_eq!(flags & 0b1100_0000, 0, "no credentials were configured");
    stream.write_all(&[0x20, 0x02, 0x00, 0x00]).unwrap(); // CONNACK, accepted

    let mut filters = Vec::new();
    while filters.len() < expect_subscriptions {
        let p = read_packet(&mut stream);
        seen.push(p.kind);
        if p.kind == PINGREQ {
            stream.write_all(&[0xd0, 0x00]).unwrap();
            continue;
        }
        assert_eq!(
            p.kind, SUBSCRIBE,
            "after connecting the client may only subscribe"
        );
        let id = [p.body[0], p.body[1]];
        let mut i = 2;
        let mut granted = Vec::new();
        while i < p.body.len() {
            let n = usize::from(u16::from_be_bytes([p.body[i], p.body[i + 1]]));
            let filter = String::from_utf8(p.body[i + 2..i + 2 + n].to_vec()).unwrap();
            let qos = p.body[i + 2 + n];
            assert_eq!(qos, 0, "subscriptions must be QoS 0");
            filters.push(filter);
            granted.push(0u8);
            i += 3 + n;
        }
        let mut suback = vec![0x90, (2 + granted.len()) as u8, id[0], id[1]];
        suback.extend_from_slice(&granted);
        stream.write_all(&suback).unwrap();
    }
    filters.sort();
    assert_eq!(filters, ["homeassistant/#", "solar_assistant/#"]);
    (stream, seen)
}

/// Reads whatever else the client sent, until it goes quiet.
fn drain_client_packets(stream: &mut TcpStream, seen: &mut Vec<u8>) {
    stream
        .set_read_timeout(Some(Duration::from_millis(300)))
        .unwrap();
    let mut first = [0u8; 1];
    while let Ok(1) = stream.peek(&mut first) {
        stream.set_read_timeout(Some(STEP_TIMEOUT)).unwrap();
        seen.push(read_packet(stream).kind);
        stream
            .set_read_timeout(Some(Duration::from_millis(300)))
            .unwrap();
    }
}

fn wait_for_lines(path: &Path, want: usize) -> Vec<Reading> {
    let deadline = Instant::now() + STEP_TIMEOUT;
    loop {
        let text = fs::read_to_string(path).unwrap_or_default();
        // Only count lines that have been written in full.
        let whole = text.rfind('\n').map_or("", |i| &text[..i]);
        let complete: Vec<&str> = whole.lines().collect();
        if complete.len() >= want {
            return complete
                .iter()
                .map(|l| serde_json::from_str(l).expect("each line is a reading"))
                .collect();
        }
        assert!(
            Instant::now() < deadline,
            "expected {want} readings in the output, found {}",
            complete.len()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

struct KillOnDrop(Child);
impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn record_subscribes_receives_reconnects_and_never_sends_a_message() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let port = listener.local_addr().unwrap().port();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    fs::create_dir_all(dir).unwrap();
    let out: PathBuf = dir.join(format!("{}-wire.jsonl", std::process::id()));
    let raw: PathBuf = dir.join(format!("{}-wire-raw.jsonl", std::process::id()));
    let _ = fs::remove_file(&out);
    let _ = fs::remove_file(&raw);

    let child = Command::new(env!("CARGO_BIN_EXE_sa-tap"))
        .args(["record", "--host", "127.0.0.1", "--port", &port.to_string()])
        .args(["--discovery-prefix", "homeassistant"])
        .args(["--out", out.to_str().unwrap()])
        .args(["--raw-log", raw.to_str().unwrap()])
        .env_remove("SA_TAP_MQTT_USERNAME")
        .env_remove("SA_TAP_MQTT_PASSWORD")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("sa-tap starts");
    let _child = KillOnDrop(child);

    // First connection.
    let (mut stream, mut seen) = accept_and_handshake(&listener, 2);
    for (topic, payload, retain) in [
        (
            "solar_assistant/inverter_1/pv_power/state",
            &b"1823"[..],
            false,
        ),
        (
            "solar_assistant/inverter_1/device_mode/state",
            b"Solar/Battery",
            false,
        ),
        (
            "solar_assistant/inverter_1/max_charge_current/set",
            b"50",
            false,
        ),
        (
            "solar_assistant/inverter_1/battery_voltage/state",
            b"53.2",
            true,
        ),
    ] {
        stream
            .write_all(&publish_packet(topic, payload, retain))
            .unwrap();
    }
    let readings = wait_for_lines(&out, 2);
    assert_eq!(
        readings.len(),
        2,
        "text and command messages produce no reading"
    );
    assert_eq!(readings[0].quantity.as_str(), "pv_power");
    assert_eq!(readings[0].value.text(), "1823");
    assert_eq!(readings[0].trust, Trust::Unverified);
    assert_eq!(readings[0].provenance.broker, format!("127.0.0.1:{port}"));
    assert!(matches!(
        readings[0].timestamp_quality,
        TimestampQuality::HostReceive { .. }
    ));
    assert_eq!(readings[1].quantity.as_str(), "battery_voltage");
    assert_eq!(
        readings[1].timestamp_quality,
        TimestampQuality::HostReceiveRetained
    );
    drain_client_packets(&mut stream, &mut seen);

    // Drop the connection. The client must come back and subscribe again.
    drop(stream);
    let (mut stream, seen_again) = accept_and_handshake(&listener, 2);
    seen.extend(seen_again);
    stream
        .write_all(&publish_packet(
            "solar_assistant/inverter_2/pv_power/state",
            b"1460",
            false,
        ))
        .unwrap();
    let readings = wait_for_lines(&out, 3);
    assert_eq!(readings[2].source_id.as_str(), "solar_assistant/inverter_2");
    drain_client_packets(&mut stream, &mut seen);

    // The raw log holds every message, including the ones that made no reading.
    let raw_text = fs::read_to_string(&raw).unwrap();
    assert_eq!(raw_text.lines().count(), 5);
    assert!(raw_text.contains("max_charge_current/set"));

    // The whole point: nothing the client sent was an application message.
    assert!(seen.contains(&CONNECT) && seen.contains(&SUBSCRIBE));
    for kind in &seen {
        assert!(
            [CONNECT, SUBSCRIBE, PINGREQ, DISCONNECT].contains(kind),
            "the client sent MQTT packet type {kind}; only connect, subscribe, ping and disconnect are allowed"
        );
        assert_ne!(*kind, PUBLISH);
    }

    let _ = fs::remove_file(&out);
    let _ = fs::remove_file(&raw);
}
