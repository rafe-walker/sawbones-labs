//! Instruction-count benchmarks for the Sawbones safety crates (#220).
//!
//! Wall-clock time is noisy on a shared runner. The number of instructions a
//! deterministic function runs is not, so `ci/bench.py` runs this binary
//! under `valgrind --tool=callgrind` and reads the instruction count (Ir).
//! Only the work inside [`sawbones_bench_measure`] is counted, so process
//! start-up and the set-up of each case are not.
//!
//! Usage: `sawbones-bench list` prints the case names, one per line, and
//! `sawbones-bench run <case>` runs one case. Each case runs its body
//! [`ITERATIONS`] times; the harness divides by that to report Ir per
//! iteration. Run on its own (not under valgrind) the binary only checks the
//! cases still work.

use std::hint::black_box;
use std::num::NonZeroU64;
use std::process::ExitCode;

use sawbones_busguard::{crc16_modbus, parse, AllowList, Function, Port, Rule};
use sawbones_interlock::{step, Event, FaultRecord, Inputs, State, RAIL_B_PORTS};
use sawbones_module_protocol::{
    decode_request, decode_response, encode_request, encode_response, Actuate, Channel,
    HealthReport, HealthStatus, ImageDigest, Request, RequestFrame, RequestHeader, Response,
    ResponseFrame, ResponseHeader, UpdateChunk, MAX_CHUNK_LEN, MAX_FRAME_LEN,
};
use sawbones_policy::{
    classify, preflight, BusFact, BusFacts, ChainHead, EffectRequest, FactContent, FactLedger,
    FactRecord, FactScope, HazardClass, Interface, LedgerId, Mastership, Method, PortId,
    SettingKind, SignalId, Validity, WriteEntry, WriteMap, WriteValue,
};
use sawbones_reading::Trust;

/// How many times each case runs its body. Fixed: the baseline is per
/// iteration, so changing it changes nothing but the resolution.
pub const ITERATIONS: u32 = 200;

/// The cases, by name. `ci/bench_baseline.json` has one entry for each.
pub const CASES: &[&str] = &[
    "protocol_encode_request",
    "protocol_encode_response",
    "protocol_decode_request",
    "protocol_decode_response",
    "busguard_frame_check",
    "policy_classify",
    "policy_preflight",
    "interlock_step",
];

const HEADER: RequestHeader = RequestHeader {
    session: 7,
    seq: 42,
};
const REPLY_HEADER: ResponseHeader = ResponseHeader {
    session: 7,
    seq: 43,
    reply_to: 42,
};

fn chunk_request(bytes: &[u8]) -> Request<'_> {
    Request::StageUpdate(UpdateChunk {
        image_digest: ImageDigest([0x5a; 32]),
        image_len: 4096,
        offset: 512,
        bytes,
    })
}

fn health_response() -> Response<'static> {
    Response::Health(HealthReport {
        status: HealthStatus::Ok,
        uptime_ticks: 123_456,
        fault_code: 0,
        bad_frames: 1,
        refused_frames: 2,
    })
}

fn encode_to_vec(frame: &RequestFrame<'_>) -> Vec<u8> {
    let mut out = [0u8; MAX_FRAME_LEN];
    let len = encode_request(frame, &mut out).expect("sample request encodes");
    out[..len].to_vec()
}

fn modbus_frame(unit: u8, function: u8, start: u16, quantity: u16, values: &[u16]) -> Vec<u8> {
    let mut body = vec![unit, function];
    body.extend_from_slice(&start.to_be_bytes());
    body.extend_from_slice(&quantity.to_be_bytes());
    if function == 0x10 {
        body.push(u8::try_from(values.len() * 2).expect("small"));
        for v in values {
            body.extend_from_slice(&v.to_be_bytes());
        }
    }
    let crc = crc16_modbus(&body);
    body.extend_from_slice(&crc.to_le_bytes());
    body
}

fn port_id(s: &str) -> PortId {
    PortId::parse(s).expect("sample port name")
}

fn signal_id(s: &str) -> SignalId {
    SignalId::parse(s).expect("sample signal name")
}

fn write_entry() -> WriteEntry {
    WriteEntry {
        port: port_id("rs485-1"),
        unit: 1,
        signal: signal_id("charge_voltage"),
        register: 0x0120,
        width: 1,
        class: HazardClass::Class2,
        kind: SettingKind::ChargeVoltage,
        energy_relevant: true,
        trust: Trust::OperatorAttested,
        range: None,
    }
}

fn bus_fact() -> BusFact {
    BusFact {
        port: port_id("rs485-1"),
        bench_port: 1,
        mastership: Mastership::BenchOnly,
        trust: Trust::OperatorAttested,
    }
}

/// Runs `body` [`ITERATIONS`] times with an observed result, so nothing is
/// optimised away.
fn repeat<T>(mut body: impl FnMut() -> T) {
    for _ in 0..ITERATIONS {
        black_box(body());
    }
}

/// Runs one case. Everything inside this function is counted by
/// `ci/bench.py` (`--toggle-collect`), so its name is part of the harness:
/// keep `sawbones_bench_measure` in the name.
///
/// Returns `false` for an unknown case.
#[inline(never)]
pub fn sawbones_bench_measure(case: &str, input: &Input) -> bool {
    match case {
        "protocol_encode_request" => {
            let mut out = [0u8; MAX_FRAME_LEN];
            let chunk = &input.chunk[..];
            let big = RequestFrame {
                header: HEADER,
                request: chunk_request(chunk),
            };
            let small = RequestFrame {
                header: HEADER,
                request: Request::Actuate(Actuate {
                    channel: Channel(3),
                    setting: -1200,
                    authority: sawbones_module_protocol::Authority::RailA,
                }),
            };
            repeat(|| {
                let a = encode_request(black_box(&big), &mut out);
                let b = encode_request(black_box(&small), &mut out);
                (a, b)
            });
        }
        "protocol_encode_response" => {
            let mut out = [0u8; MAX_FRAME_LEN];
            let frame = ResponseFrame {
                header: REPLY_HEADER,
                response: health_response(),
            };
            repeat(|| encode_response(black_box(&frame), &mut out));
        }
        "protocol_decode_request" => {
            repeat(|| {
                let a = decode_request(black_box(&input.big_request));
                let b = decode_request(black_box(&input.small_request));
                (a.is_ok(), b.is_ok())
            });
        }
        "protocol_decode_response" => {
            repeat(|| decode_response(black_box(&input.response)).is_ok());
        }
        "busguard_frame_check" => {
            repeat(|| {
                let mut passed = 0u32;
                for frame in &input.modbus {
                    if let Ok(parsed) = parse(black_box(frame)) {
                        if input.allow.permits(Port(1), &parsed) {
                            passed += 1;
                        }
                    }
                }
                passed
            });
        }
        "policy_classify" => {
            repeat(|| {
                let w = classify(black_box(&input.write), &input.writes, &input.buses);
                let r = classify(black_box(&input.read), &input.writes, &input.buses);
                (w.class(), r.class())
            });
        }
        "policy_preflight" => {
            repeat(|| {
                preflight(
                    black_box(&input.read),
                    &input.writes,
                    &input.buses,
                    &input.ledger,
                    ChainHead([0xab; 32]),
                    input.now,
                )
                .is_ok()
            });
        }
        "interlock_step" => {
            repeat(|| {
                let restart = Event::Restart {
                    record: Some(FaultRecord::Clear),
                    nonce: NonZeroU64::new(1),
                };
                let (mut state, _) = step(State::POWER_ON, input.idle(0, true), Some(restart));
                for t in 1..=16u64 {
                    let inputs = input.idle(t, t % 4 == 0);
                    let (next, _) = step(black_box(state), inputs, None);
                    state = next;
                }
                state
            });
        }
        _ => return false,
    }
    true
}

/// Everything a case needs, built before the measured region.
pub struct Input {
    chunk: [u8; MAX_CHUNK_LEN],
    big_request: Vec<u8>,
    small_request: Vec<u8>,
    response: Vec<u8>,
    modbus: Vec<Vec<u8>>,
    allow: AllowList,
    writes: WriteMap,
    buses: BusFacts,
    ledger: FactLedger,
    write: EffectRequest,
    read: EffectRequest,
    now: u64,
}

impl Input {
    fn new() -> Self {
        let mut chunk = [0u8; MAX_CHUNK_LEN];
        for (i, b) in chunk.iter_mut().enumerate() {
            *b = u8::try_from(i % 251).expect("below 251");
        }
        let big_request = encode_to_vec(&RequestFrame {
            header: HEADER,
            request: chunk_request(&chunk),
        });
        let small_request = encode_to_vec(&RequestFrame {
            header: HEADER,
            request: Request::Actuate(Actuate {
                channel: Channel(3),
                setting: -1200,
                authority: sawbones_module_protocol::Authority::RailA,
            }),
        });
        let mut out = [0u8; MAX_FRAME_LEN];
        let len = encode_response(
            &ResponseFrame {
                header: REPLY_HEADER,
                response: health_response(),
            },
            &mut out,
        )
        .expect("sample response encodes");
        let response = out[..len].to_vec();

        let mut allow = AllowList::new();
        allow
            .try_push(Rule {
                port: Port(1),
                unit: 1,
                function: Function::WriteMultipleRegisters,
                first: 0x0100,
                last: 0x01ff,
                max_quantity: 8,
            })
            .expect("one rule fits");
        let modbus = vec![
            modbus_frame(1, 0x03, 0x0100, 10, &[]),
            modbus_frame(1, 0x10, 0x0120, 2, &[0x1234, 0x5678]),
            // Refused: a unit the list does not name.
            modbus_frame(9, 0x10, 0x0120, 2, &[1, 2]),
        ];

        let writes = WriteMap::new([write_entry()]).expect("sample write map");
        let buses = BusFacts::new([bus_fact()]).expect("sample bus facts");
        let now = 2;
        let mut ledger = FactLedger::new(LedgerId(42));
        let fact = bus_fact();
        ledger
            .record(
                FactRecord {
                    scope: FactScope {
                        device: None,
                        interface: Interface {
                            port: fact.port.clone(),
                            bench_port: fact.bench_port,
                            unit: None,
                        },
                        conditions: std::collections::BTreeSet::new(),
                        method: Method::OperatorAttested,
                    },
                    content: FactContent::Mastership(fact),
                    earned_at: 1,
                    validity: Validity::For { ticks: 10_000 },
                },
                now,
            )
            .expect("sample mastership fact");

        Self {
            chunk,
            big_request,
            small_request,
            response,
            modbus,
            allow,
            writes,
            buses,
            ledger,
            write: EffectRequest::BusWrite(sawbones_policy::BusWrite {
                port: port_id("rs485-1"),
                unit: 1,
                signal: signal_id("charge_voltage"),
                value: WriteValue::U16(5600),
            }),
            read: EffectRequest::BusRead(sawbones_policy::BusRead {
                port: port_id("rs485-1"),
                unit: 1,
                signal: signal_id("charge_voltage"),
            }),
            now,
        }
    }

    fn idle(&self, now: u64, arm_switch_on: bool) -> Inputs {
        Inputs {
            estop_pressed: false,
            arm_switch_on,
            key_switch_on: false,
            heartbeat: None,
            panel_button_pressed: false,
            rail_a_sensed: false,
            rail_b_sensed: [false; RAIL_B_PORTS],
            firmware_jumper_fitted: false,
            now,
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["list"] => {
            for case in CASES {
                println!("{case}");
            }
            ExitCode::SUCCESS
        }
        ["run", case] => {
            let input = Input::new();
            if sawbones_bench_measure(case, &input) {
                ExitCode::SUCCESS
            } else {
                eprintln!("unknown case {case}; try `list`");
                ExitCode::from(2)
            }
        }
        _ => {
            eprintln!("usage: sawbones-bench list | run <case>");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_case_runs() {
        let input = Input::new();
        for case in CASES {
            assert!(sawbones_bench_measure(case, &input), "{case} did not run");
        }
        assert!(!sawbones_bench_measure("no_such_case", &input));
    }

    #[test]
    fn the_sample_inputs_take_the_paths_the_cases_claim() {
        let input = Input::new();
        assert!(decode_request(&input.big_request).is_ok());
        assert!(decode_request(&input.small_request).is_ok());
        assert!(decode_response(&input.response).is_ok());
        let passes: Vec<bool> = input
            .modbus
            .iter()
            .map(|f| {
                parse(f)
                    .map(|p| input.allow.permits(Port(1), &p))
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(passes, [false, true, false]);
        let token = preflight(
            &input.read,
            &input.writes,
            &input.buses,
            &input.ledger,
            ChainHead([0xab; 32]),
            input.now,
        );
        assert!(token.is_ok(), "{token:?}");
    }
}
