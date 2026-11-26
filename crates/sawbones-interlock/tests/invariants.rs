//! Safety invariants, checked two ways: exhaustively over every single step
//! from a set of representative states, and over long random input sequences
//! against an independent record of what the sequence so far permits. A third
//! test pins the whole single-step transition table to [`RULES_VERSION`].
//!
//! The random sequences play the firmware and the supervisor around the
//! interlock: restarts that keep RAM or lose it, fault records that are kept,
//! torn or wrong, and a supervisor that answers heartbeat challenges late,
//! wrongly, twice, stale, or not at all, or freezes.

use sawbones_interlock::{
    step, ActionDigest, Approval, ClearSource, Event, FaultReason, FaultRecord, HazardClass,
    HeartbeatChallenge, HeartbeatReply, HeartbeatWatch, Inputs, LeaseTarget, Mode, Outputs,
    PanelState, PanelToken, Port, Rail, Refusal, State, Tick, HEARTBEAT_PERIOD_TICKS,
    LEASE_TTL_TICKS, OUTPUT_SETTLE_TICKS, RAIL_B_PORTS, REQUEST_TTL_TICKS, RULES_VERSION,
};

const CLASSES: [HazardClass; 5] = [
    HazardClass::Class0,
    HazardClass::Class1,
    HazardClass::Class2,
    HazardClass::Class3,
    HazardClass::Class4,
];

const DIGESTS: [ActionDigest; 2] = [ActionDigest([0xa1; 32]), ActionDigest([0xb2; 32])];

const PANEL_CLEAR: Event = Event::ClearFault {
    source: ClearSource::Panel(PanelToken::from_panel_button()),
};
const SUPERVISOR_CLEAR: Event = Event::ClearFault {
    source: ClearSource::Supervisor,
};
const UNKNOWN: FaultReason = FaultReason::FaultStateUnknown;
const P: Tick = HEARTBEAT_PERIOD_TICKS;

#[allow(clippy::cast_possible_truncation)]
const LAST_PORT: u8 = (RAIL_B_PORTS - 1) as u8;
#[allow(clippy::cast_possible_truncation)]
const NO_SUCH_PORT: u8 = RAIL_B_PORTS as u8;

/// Targets on the first port, the last port and a port with no rail B, with
/// each of two actions.
fn targets() -> Vec<LeaseTarget> {
    let mut out = Vec::new();
    for port in [0, LAST_PORT, NO_SUCH_PORT] {
        for action_digest in DIGESTS {
            out.push(LeaseTarget {
                port: Port(port),
                action_digest,
            });
        }
    }
    out
}

/// Every kind of event, and no event: restarts with each kind of record, and
/// clears from the panel and from the supervisor.
fn all_events() -> Vec<Option<Event>> {
    let mut events = vec![
        None,
        Some(Event::RequestArm),
        Some(PANEL_CLEAR),
        Some(SUPERVISOR_CLEAR),
    ];
    for record in [
        None,
        Some(FaultRecord::Clear),
        Some(FaultRecord::Armed),
        Some(FaultRecord::Latched(FaultReason::EmergencyStop)),
    ] {
        events.push(Some(Event::Restart { record, nonce: 77 }));
    }
    for target in targets() {
        events.extend(CLASSES.map(|class| Some(Event::RequestLease { class, target })));
        events.push(Some(Event::LeaseUsed { target }));
    }
    events
}

/// Rail B feedback patterns: none, the first port, the last port, every port.
fn rail_b_patterns() -> [[bool; RAIL_B_PORTS]; 4] {
    let mut first = [false; RAIL_B_PORTS];
    first[0] = true;
    let mut last = [false; RAIL_B_PORTS];
    last[RAIL_B_PORTS - 1] = true;
    [[false; RAIL_B_PORTS], first, last, [true; RAIL_B_PORTS]]
}

/// The heartbeat replies the single-step grid tries, as the watch in the
/// state would see them: none, the right one, a wrong one, and the one for
/// the challenge before.
fn replies(watch: HeartbeatWatch) -> [Option<HeartbeatReply>; 4] {
    let stale = HeartbeatWatch {
        seq: watch.seq.wrapping_sub(1),
        ..watch
    };
    let right = watch.challenge().reply();
    [
        None,
        Some(right),
        Some(HeartbeatReply(right.0 ^ 0x8000)),
        Some(stale.challenge().reply()),
    ]
}

/// All 64 combinations of the six sampled booleans, with each heartbeat reply
/// and each rail B feedback pattern.
fn all_inputs(now: Tick, watch: HeartbeatWatch) -> impl Iterator<Item = Inputs> {
    (0u8..64).flat_map(move |bits| {
        replies(watch).into_iter().flat_map(move |heartbeat| {
            rail_b_patterns().map(move |rail_b_sensed| Inputs {
                estop_pressed: bits & 1 != 0,
                arm_switch_on: bits & 2 != 0,
                key_switch_on: bits & 4 != 0,
                panel_button_pressed: bits & 8 != 0,
                rail_a_sensed: bits & 16 != 0,
                firmware_jumper_fitted: bits & 32 != 0,
                heartbeat,
                rail_b_sensed,
                now,
            })
        })
    })
}

/// What the rails were commanded to before this tick, worked out from the
/// outputs alone: rail A's on-tick and rail B's port and on-tick.
#[derive(Clone, Copy, Default)]
struct Commanded {
    rail_a_since: Option<Tick>,
    rail_b: Option<(Port, Tick)>,
}

impl Commanded {
    /// From a mode, for the single-step test.
    fn of(mode: Mode) -> Self {
        match mode {
            Mode::Armed { since, approval } => Commanded {
                rail_a_since: Some(since),
                rail_b: match approval {
                    Approval::Live { granted_at, target } => Some((target.port, granted_at)),
                    _ => None,
                },
            },
            _ => Commanded::default(),
        }
    }

    /// True if the feedback on this tick disagrees with the command: a rail
    /// live while commanded off, or dead while commanded on past its settle
    /// window (or with the clock gone backwards).
    fn disagrees(self, inputs: &Inputs) -> bool {
        let settled = |since: Tick| inputs.now < since || inputs.now - since >= OUTPUT_SETTLE_TICKS;
        let bad = |since: Option<Tick>, sensed: bool| match since {
            None => sensed,
            Some(since) => !sensed && settled(since),
        };
        bad(self.rail_a_since, inputs.rail_a_sensed)
            || (0..RAIL_B_PORTS).any(|p| {
                let since = self
                    .rail_b
                    .filter(|(port, _)| usize::from(port.0) == p)
                    .map(|(_, at)| at);
                bad(since, inputs.rail_b_sensed[p])
            })
    }
}

/// The record and nonce of a restart event.
fn restart_of(event: Option<Event>) -> Option<(Option<FaultRecord>, u64)> {
    match event {
        Some(Event::Restart { record, nonce }) => Some((record, nonce)),
        _ => None,
    }
}

/// The heartbeat rules, written out again from the spec: whether this tick
/// loses the heartbeat, and whether it is alive afterwards.
fn judge_beat(before: State, inputs: &Inputs, event: Option<Event>) -> (bool, bool) {
    let watch = match restart_of(event) {
        Some((_, nonce)) => HeartbeatWatch::start(nonce, inputs.now),
        None => before.heartbeat,
    };
    let age = inputs.now.checked_sub(watch.issued_at);
    let good = !watch.answered
        && inputs.heartbeat == Some(watch.challenge().reply())
        && age.is_some_and(|a| (1..P).contains(&a));
    let bad_reply = inputs.heartbeat.is_some() && !good;
    let over = age.is_none_or(|a| a >= P);
    let stalled = age.is_none_or(|a| a >= 2 * P);
    let missed = over && (stalled || !(watch.answered || good));
    let lost = bad_reply || missed;
    (lost, !lost && (watch.alive || good))
}

/// True if a restart from `kept` with `record` cannot come up unfaulted.
fn restart_is_dirty(kept: Mode, record: Option<FaultRecord>) -> bool {
    let kept_dirty = match kept {
        Mode::Faulted(reason) => reason != UNKNOWN,
        Mode::Armed { .. } => true,
        _ => false,
    };
    kept_dirty || record != Some(FaultRecord::Clear)
}

/// The invariants that can be judged from one tick alone.
fn check_tick(
    before: State,
    inputs: Inputs,
    event: Option<Event>,
    next: State,
    out: Outputs,
    context: &dyn Fn() -> String,
) {
    let restart = restart_of(event);
    let driven = if restart.is_some() {
        Commanded::default()
    } else {
        Commanded::of(before.mode)
    };
    let armed_before = restart.is_none() && matches!(before.mode, Mode::Armed { .. });
    let disagrees = driven.disagrees(&inputs);
    let (lost, alive) = judge_beat(before, &inputs, event);
    let rail_on = out.rail_a || out.rail_b.is_some();

    if let Some(port) = out.rail_b {
        assert!(out.rail_a, "rail B without rail A: {}", context());
        assert!(
            inputs.key_switch_on,
            "rail B without the key: {}",
            context()
        );
        assert!(
            port.exists(),
            "rail B on a port that does not exist: {}",
            context()
        );
    }
    if rail_on {
        assert_eq!(out.fault, None, "a rail on with a fault: {}", context());
        assert!(
            !inputs.estop_pressed,
            "a rail on with the stop pressed: {}",
            context()
        );
        assert!(
            !lost && alive,
            "a rail on without a correct heartbeat: {}",
            context()
        );
        assert!(
            inputs.arm_switch_on,
            "a rail on with the arm switch off: {}",
            context()
        );
        assert!(
            !inputs.firmware_jumper_fitted,
            "a rail on with the firmware jumper fitted: {}",
            context()
        );
        assert!(
            !disagrees,
            "a rail on with an output mismatch: {}",
            context()
        );
        assert!(
            !out.firmware_update_allowed,
            "firmware allowed with a rail on: {}",
            context()
        );
    }
    if inputs.estop_pressed || lost || disagrees {
        assert!(
            out.fault.is_some(),
            "a hazard that did not latch: {}",
            context()
        );
    }
    if inputs.firmware_jumper_fitted && armed_before {
        assert!(
            out.fault.is_some(),
            "the jumper fitted while armed did not latch: {}",
            context()
        );
    }
    if out.firmware_update_allowed {
        assert!(
            inputs.firmware_jumper_fitted,
            "firmware allowed without the jumper: {}",
            context()
        );
        assert!(!rail_on, "firmware allowed with a rail on: {}", context());
        assert_eq!(
            out.fault,
            None,
            "firmware allowed with a fault: {}",
            context()
        );
        assert_eq!(out.panel, PanelState::FirmwareUpdate, "{}", context());
    }
    assert_eq!(
        out.fault.is_some(),
        out.panel == PanelState::Fault,
        "panel and fault disagree: {}",
        context()
    );
    if let Some((record, _)) = restart {
        assert!(!rail_on, "a rail on at a restart: {}", context());
        if restart_is_dirty(before.mode, record) {
            assert!(
                out.fault.is_some(),
                "a restart that could not prove a clean record came up unfaulted: {}",
                context()
            );
        }
    }
    if event == Some(SUPERVISOR_CLEAR) {
        assert_eq!(
            out.refusal,
            Some(Refusal::ClearNotFromPanel),
            "a supervisor clear not refused: {}",
            context()
        );
    }
    if let Some(Event::RequestLease { class, target }) = event {
        let want = match class {
            HazardClass::Class4 => Some(Some(Refusal::ClassNeverPermitted)),
            HazardClass::Class3 if !target.port.exists() => Some(Some(Refusal::UnknownPort)),
            HazardClass::Class3 => None,
            _ => Some(Some(Refusal::NoLeaseForClass)),
        };
        if let Some(want) = want {
            assert_eq!(out.refusal, want, "class or port refusal: {}", context());
        }
    }
    // The outputs are a function of the state alone, apart from the refusal.
    let from_state = Outputs {
        refusal: out.refusal,
        ..next.outputs()
    };
    assert_eq!(
        out,
        from_state,
        "outputs not derived from the state: {}",
        context()
    );
}

/// The heartbeat watches the single-step grid starts from, at `now`: alive
/// and answered, alive and waiting, not alive, at the deadline answered and
/// unanswered, two periods on (a stalled loop), just issued, and issued in
/// the future (the clock went back).
fn watches(now: Tick) -> [HeartbeatWatch; 8] {
    let w = |issued_at, answered, alive| HeartbeatWatch {
        nonce: 0x5eed,
        seq: 3,
        issued_at,
        answered,
        alive,
    };
    [
        w(now - 3, true, true),
        w(now - 3, false, true),
        w(now - 3, false, false),
        w(now - P, true, true),
        w(now - P, false, true),
        w(now - 2 * P, true, true),
        w(now, false, true),
        w(now + 1, true, true),
    ]
}

/// The representative states for the single-step tests. Every mode that is
/// not armed is paired with every watch; each armed mode with one live watch
/// in turn, since an armed mode is only reached with the heartbeat alive.
fn representative_states(now: Tick) -> Vec<State> {
    let stamps = [
        now + 1,
        now,
        now - 1,
        now - (LEASE_TTL_TICKS - 1),
        now - LEASE_TTL_TICKS,
        now - (REQUEST_TTL_TICKS - 1),
        now - REQUEST_TTL_TICKS,
        0,
    ];
    let idle = [
        Mode::AwaitArmOff,
        Mode::Disarmed,
        Mode::FirmwareUpdate,
        Mode::Faulted(FaultReason::EmergencyStop),
        Mode::Faulted(FaultReason::HeartbeatLost),
        Mode::Faulted(FaultReason::FirmwareJumperWhileArmed),
        Mode::Faulted(FaultReason::OutputMismatch {
            rail: Rail::A,
            sensed_live: true,
        }),
        Mode::Faulted(FaultReason::OutputMismatch {
            rail: Rail::B(Port(LAST_PORT)),
            sensed_live: false,
        }),
        Mode::Faulted(UNKNOWN),
    ];
    let mut states = Vec::new();
    for mode in idle {
        for heartbeat in watches(now) {
            states.push(State { mode, heartbeat });
        }
    }

    let leased = [
        LeaseTarget {
            port: Port(0),
            action_digest: DIGESTS[0],
        },
        LeaseTarget {
            port: Port(LAST_PORT),
            action_digest: DIGESTS[0],
        },
    ];
    let mut armed = Vec::new();
    for since in [
        now,
        now - (OUTPUT_SETTLE_TICKS - 1),
        now - OUTPUT_SETTLE_TICKS,
    ] {
        armed.push(Mode::Armed {
            since,
            approval: Approval::Idle,
        });
        for at in stamps {
            for target in leased {
                for approval in [
                    Approval::AwaitRelease {
                        requested_at: at,
                        target,
                    },
                    Approval::AwaitPress {
                        requested_at: at,
                        target,
                    },
                    Approval::Live {
                        granted_at: at,
                        target,
                    },
                ] {
                    armed.push(Mode::Armed { since, approval });
                }
            }
        }
    }
    let live: Vec<HeartbeatWatch> = watches(now).into_iter().filter(|w| w.alive).collect();
    for (i, mode) in armed.into_iter().enumerate() {
        states.push(State {
            mode,
            heartbeat: live[i % live.len()],
        });
    }
    states
}

/// Calls `visit` for every step from every representative state, with every
/// input and every event. Returns how many steps it visited.
fn for_every_single_step(
    mut visit: impl FnMut(State, Inputs, Option<Event>, State, Outputs),
) -> u64 {
    let now: Tick = 1_000;
    let events = all_events();
    let mut count = 0;
    for state in representative_states(now) {
        for inputs in all_inputs(now, state.heartbeat) {
            for &event in &events {
                let (next, out) = step(state, inputs, event);
                visit(state, inputs, event, next, out);
                count += 1;
            }
        }
    }
    count
}

#[test]
fn every_single_step_from_representative_states_is_safe() {
    let now: Tick = 1_000;
    let within = |since: Tick, ttl: Tick| since <= now && now - since < ttl;
    let checked = for_every_single_step(|state, inputs, event, next, out| {
        let context = || format!("{state:?} + {inputs:?} + {event:?} -> {next:?}");
        check_tick(state, inputs, event, next, out, &context);
        assert_eq!(step(state, inputs, event), (next, out), "not deterministic");
        let restart = restart_of(event);
        let (_, alive) = judge_beat(state, &inputs, event);

        // Rail A needs the switch to have been seen off already, and no
        // restart on this tick.
        if out.rail_a {
            assert!(
                restart.is_none() && matches!(state.mode, Mode::Disarmed | Mode::Armed { .. }),
                "armed without a sequence: {}",
                context()
            );
        }
        // Rail B needs a lease still in its window, or a fresh press
        // answering a request still in its window, for that very port.
        if let Some(port) = out.rail_b {
            let permitted = match state.mode {
                Mode::Armed {
                    approval: Approval::Live { granted_at, target },
                    ..
                } => {
                    within(granted_at, LEASE_TTL_TICKS)
                        && !matches!(event, Some(Event::LeaseUsed { .. }))
                        && target.port == port
                }
                Mode::Armed {
                    approval:
                        Approval::AwaitPress {
                            requested_at,
                            target,
                        },
                    ..
                } => {
                    inputs.panel_button_pressed
                        && within(requested_at, REQUEST_TTL_TICKS)
                        && target.port == port
                }
                _ => false,
            };
            assert!(permitted, "rail B without a lease: {}", context());
        }
        // Any use, right or wrong, ends a live lease.
        if let (
            Mode::Armed {
                approval: Approval::Live { target, .. },
                ..
            },
            Some(Event::LeaseUsed { target: named }),
        ) = (state.mode, event)
        {
            assert_eq!(out.rail_b, None, "a use left rail B on: {}", context());
            let live = inputs.key_switch_on
                && inputs.arm_switch_on
                && out.fault.is_none()
                && matches!(next.mode, Mode::Armed { .. });
            if live && named != target && out.refusal != Some(Refusal::NoLiveLease) {
                assert_eq!(
                    out.refusal,
                    Some(Refusal::LeaseMismatch),
                    "a wrong use not refused: {}",
                    context()
                );
            }
        }
        // A latched fault leaves only by a panel clear, or by a restart when
        // the fault was only "unknown" and the record settles it.
        if let Mode::Faulted(reason) = state.mode {
            let clear = event == Some(PANEL_CLEAR)
                && inputs.panel_button_pressed
                && !inputs.estop_pressed
                && !inputs.arm_switch_on
                && alive
                && !inputs.rail_a_sensed
                && !inputs.rail_b_sensed.contains(&true);
            if !(restart.is_some() && reason == UNKNOWN) {
                let want = if clear {
                    Mode::Disarmed
                } else {
                    Mode::Faulted(reason)
                };
                assert_eq!(next.mode, want, "latch: {}", context());
            }
        }
        // A restart comes up exactly as the worse of RAM and record says.
        if let Some((record, _)) = restart {
            let want = match (state.mode, record) {
                (Mode::Faulted(reason), _) if reason != UNKNOWN => Some(reason),
                (_, Some(FaultRecord::Latched(reason))) => Some(reason),
                _ if restart_is_dirty(state.mode, record) => Some(UNKNOWN),
                _ => None,
            };
            if let Some(reason) = want {
                assert_eq!(next.mode, Mode::Faulted(reason), "restart: {}", context());
            }
        }
        // A supervisor clear changes nothing at all.
        if event == Some(SUPERVISOR_CLEAR) {
            assert_eq!(
                step(state, inputs, None).0,
                next,
                "a supervisor clear had an effect: {}",
                context()
            );
        }
        // The firmware state is reached only from a disarmed state or a
        // clean restart.
        if out.firmware_update_allowed {
            let clean_restart =
                restart.is_some_and(|(record, _)| !restart_is_dirty(state.mode, record));
            assert!(
                (restart.is_none()
                    && matches!(
                        state.mode,
                        Mode::AwaitArmOff | Mode::Disarmed | Mode::FirmwareUpdate
                    ))
                    || clean_restart,
                "firmware allowed from an armed or faulted state: {}",
                context()
            );
        }
    });
    let states = representative_states(now).len() as u64;
    assert_eq!(checked, states * 1024 * all_events().len() as u64);
}

/// FNV-1a over an explicit byte encoding of states and outputs. The encoding
/// is written out here, not taken from `Debug` or `Hash`, so a toolchain
/// change cannot move the hash.
struct Fnv(u64);

impl Fnv {
    fn byte(&mut self, b: u8) {
        self.0 ^= u64::from(b);
        self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
    }
    fn flag(&mut self, b: bool) {
        self.byte(u8::from(b));
    }
    fn word(&mut self, w: u64) {
        for b in w.to_le_bytes() {
            self.byte(b);
        }
    }
    fn target(&mut self, t: LeaseTarget) {
        self.byte(t.port.0);
        for b in t.action_digest.0 {
            self.byte(b);
        }
    }
    fn rail(&mut self, rail: Rail) {
        match rail {
            Rail::A => self.byte(0),
            Rail::B(port) => {
                self.byte(1);
                self.byte(port.0);
            }
        }
    }
    fn fault(&mut self, reason: FaultReason) {
        match reason {
            FaultReason::EmergencyStop => self.byte(0),
            FaultReason::HeartbeatLost => self.byte(1),
            FaultReason::OutputMismatch { rail, sensed_live } => {
                self.byte(2);
                self.rail(rail);
                self.flag(sensed_live);
            }
            FaultReason::FirmwareJumperWhileArmed => self.byte(3),
            FaultReason::FaultStateUnknown => self.byte(4),
        }
    }
    fn mode(&mut self, mode: Mode) {
        match mode {
            Mode::AwaitArmOff => self.byte(0),
            Mode::Disarmed => self.byte(1),
            Mode::FirmwareUpdate => self.byte(2),
            Mode::Armed { since, approval } => {
                self.byte(3);
                self.word(since);
                match approval {
                    Approval::Idle => self.byte(0),
                    Approval::AwaitRelease {
                        requested_at,
                        target,
                    } => {
                        self.byte(1);
                        self.word(requested_at);
                        self.target(target);
                    }
                    Approval::AwaitPress {
                        requested_at,
                        target,
                    } => {
                        self.byte(2);
                        self.word(requested_at);
                        self.target(target);
                    }
                    Approval::Live { granted_at, target } => {
                        self.byte(3);
                        self.word(granted_at);
                        self.target(target);
                    }
                }
            }
            Mode::Faulted(reason) => {
                self.byte(4);
                self.fault(reason);
            }
        }
    }
    fn state(&mut self, state: State) {
        self.mode(state.mode);
        let w = state.heartbeat;
        self.word(w.nonce);
        self.word(w.seq);
        self.word(w.issued_at);
        self.flag(w.answered);
        self.flag(w.alive);
    }
    fn outputs(&mut self, out: Outputs) {
        self.flag(out.rail_a);
        match out.rail_b {
            None => self.byte(0xff),
            Some(port) => self.byte(port.0),
        }
        self.flag(out.firmware_update_allowed);
        self.byte(match out.panel {
            PanelState::AwaitingArmOff => 0,
            PanelState::Disarmed => 1,
            PanelState::FirmwareUpdate => 2,
            PanelState::Armed => 3,
            PanelState::ApprovalRequested => 4,
            PanelState::LeaseLive => 5,
            PanelState::Fault => 6,
        });
        match out.fault {
            None => self.byte(0xff),
            Some(reason) => self.fault(reason),
        }
        self.word(out.challenge.0);
        self.byte(match out.refusal {
            None => 0xff,
            Some(Refusal::ClassNeverPermitted) => 0,
            Some(Refusal::NoLeaseForClass) => 1,
            Some(Refusal::UnknownPort) => 2,
            Some(Refusal::Faulted) => 3,
            Some(Refusal::NotArmed) => 4,
            Some(Refusal::FirmwareJumperFitted) => 5,
            Some(Refusal::KeyOff) => 6,
            Some(Refusal::LeaseBusy) => 7,
            Some(Refusal::NoLiveLease) => 8,
            Some(Refusal::LeaseMismatch) => 9,
            Some(Refusal::EmergencyStopPressed) => 10,
            Some(Refusal::HeartbeatUnhealthy) => 11,
            Some(Refusal::RailSensedLive) => 12,
            Some(Refusal::ArmSwitchOn) => 13,
            Some(Refusal::ClearNotFromPanel) => 14,
            Some(Refusal::PanelButtonNotPressed) => 15,
        });
    }
}

/// Every rule-set version and the hash of its single-step transition table,
/// oldest first. Version 1 (#150) predates this pin. Version 3 (#197, #198)
/// changed the grid itself (heartbeat watches, restart records, clear
/// sources), as well as the rules.
const TRANSITION_TABLES: &[(u32, u64)] = &[(2, 0xa1ce_7c5f_feda_0b92), (3, 0x8291_075e_8d7c_df85)];

/// The trust root reports [`sawbones_interlock::rules_id`], which is built
/// from [`RULES_VERSION`]. This test hashes the next state and outputs of
/// every step in the single-step grid, so a change to what [`step`] does
/// fails here until a new row is added with a bumped [`RULES_VERSION`].
///
/// To change the rules on purpose: bump `RULES_VERSION`, append a row here
/// with the new hash this test prints, and update `rules_id_is_pinned` in
/// `tests/rules.rs`. Never edit an existing row.
#[test]
fn rules_version_pins_the_transition_table() {
    let mut hash = Fnv(0xcbf2_9ce4_8422_2325);
    for_every_single_step(|_, _, _, next, out| {
        hash.state(next);
        hash.outputs(out);
    });
    let hash = hash.0;

    for pair in TRANSITION_TABLES.windows(2) {
        assert!(pair[0].0 < pair[1].0, "versions must increase");
    }
    for (i, a) in TRANSITION_TABLES.iter().enumerate() {
        for b in &TRANSITION_TABLES[i + 1..] {
            assert_ne!(a.1, b.1, "two versions with the same table");
        }
    }
    let &(version, pinned) = TRANSITION_TABLES.last().unwrap();
    assert_eq!(
        version, RULES_VERSION,
        "RULES_VERSION has no row in TRANSITION_TABLES"
    );
    assert_eq!(
        hash, pinned,
        "the transition table changed (hash {hash:#018x}): bump RULES_VERSION and append a row"
    );
}

/// A small xorshift generator, so the sequences are the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    /// True about one time in `n`.
    fn one_in(&mut self, n: u64) -> bool {
        self.next().is_multiple_of(n)
    }
    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[(self.next() % items.len() as u64) as usize]
    }
}

/// What the sequence so far permits, kept as plain flags and the outputs
/// seen, with no knowledge of the machine's states. It may permit more than
/// the machine does, never less.
#[derive(Default)]
struct Oracle {
    /// The arm switch has been seen off since the last restart or hazard.
    arm_seen_off: bool,
    /// A class 3 request that a press could still answer: the tick it was
    /// made, whether the button has been seen released since, and its target.
    request: Option<(Tick, bool, LeaseTarget)>,
    /// The tick of the press that could have granted the current lease, and
    /// the lease's target.
    grant: Option<(Tick, LeaseTarget)>,
    /// What the previous tick's outputs commanded, and since when.
    commanded: Commanded,
    /// The outputs of the previous tick.
    fault: Option<FaultReason>,
    rail_b: Option<Port>,
    arm_switch_was_on: bool,
    rail_a_was_on: bool,
    /// The challenge the previous tick showed, the tick it first appeared,
    /// and whether a correct reply to it has been sent.
    shown: Option<(HeartbeatChallenge, Tick, bool)>,
    /// A correct reply has been sent since the last restart or loss.
    beat_ok: bool,
    /// The tick of the last correct reply.
    last_good: Option<Tick>,
    /// A restart could not prove a clean record, and no panel clear has
    /// happened since.
    must_clear: bool,
    /// How the heartbeat was lost on the last tick, for the coverage counts:
    /// 1 a bad reply, 2 a missed deadline.
    lost_how: u8,
}

impl Oracle {
    /// Folds one tick in, then checks the machine's outputs against it.
    /// `kept` is the mode passed into the step: after a power loss it is
    /// `POWER_ON`, whatever was on before.
    fn check(
        &mut self,
        kept: Mode,
        inputs: Inputs,
        event: Option<Event>,
        out: Outputs,
        context: &str,
    ) {
        let now = inputs.now;
        let restart = restart_of(event);
        let armed_before = self.commanded.rail_a_since.is_some() && restart.is_none();
        let driven = if restart.is_some() {
            Commanded::default()
        } else {
            self.commanded
        };
        let mismatch = driven.disagrees(&inputs);
        let jumper = inputs.firmware_jumper_fitted;

        // The heartbeat, judged from the challenges shown and the replies
        // sent, with no look at the machine's watch.
        if restart.is_some() {
            self.shown = None;
            self.beat_ok = false;
        }
        self.lost_how = 0;
        if let Some(reply) = inputs.heartbeat {
            let good = self.shown.is_some_and(|(challenge, at, answered)| {
                !answered && reply == challenge.reply() && now > at && now - at < P
            });
            if good {
                if let Some(shown) = self.shown.as_mut() {
                    shown.2 = true;
                }
                self.beat_ok = true;
                self.last_good = Some(now);
            } else {
                self.beat_ok = false;
                self.lost_how = 1;
            }
        }
        let period_over = self
            .shown
            .is_some_and(|(_, at, _)| now < at || now - at >= P);
        let stalled = self
            .shown
            .is_some_and(|(_, at, _)| now < at || now - at >= 2 * P);
        if period_over && (stalled || !self.shown.is_some_and(|(_, _, answered)| answered)) {
            self.beat_ok = false;
            self.lost_how = 2;
        }
        let beat_lost = self.lost_how != 0;
        // A new challenge appears exactly when a period is over or on a
        // restart.
        let new_challenge = self.shown.is_none_or(|(c, _, _)| c != out.challenge);
        assert_eq!(
            new_challenge,
            period_over || restart.is_some(),
            "a challenge changed off schedule: {context}"
        );
        if new_challenge {
            self.shown = Some((out.challenge, now, false));
        }

        // What a restart may come up as.
        let restart_dirty = restart.is_some_and(|(record, _)| {
            let ram_dirty = match kept {
                Mode::Faulted(reason) => reason != UNKNOWN,
                Mode::Armed { .. } => true,
                _ => false,
            };
            ram_dirty || record != Some(FaultRecord::Clear)
        });
        if restart_dirty {
            assert!(
                out.fault.is_some(),
                "a restart without a clean record came up unfaulted: {context}"
            );
        }
        if restart.is_some() {
            // Each restart is judged afresh: a clean record after a power loss
            // is believed, as the firmware contract says.
            self.must_clear = restart_dirty;
        }

        let hazard = inputs.estop_pressed || beat_lost || mismatch || (jumper && armed_before);

        // The jumper takes the machine out of the arm sequence, except while
        // a fault is latched: a clear with the jumper fitted still sees the
        // arm switch off.
        let jumper_counts = jumper && self.fault.is_none();
        if restart.is_some() || hazard || jumper_counts || restart_dirty {
            self.arm_seen_off = false;
        }
        if !inputs.arm_switch_on && !hazard && !jumper_counts && !restart_dirty {
            self.arm_seen_off = true;
        }

        let withdrawn =
            restart.is_some() || hazard || jumper || !inputs.arm_switch_on || !inputs.key_switch_on;
        let request_open = self
            .request
            .is_some_and(|(at, _, _)| now - at < REQUEST_TTL_TICKS);
        let grant_live = self.grant.filter(|(at, _)| now - at < LEASE_TTL_TICKS);

        // A wrong use is refused and ends the lease.
        if let (Some(Event::LeaseUsed { target: named }), Some((_, target)), false) =
            (event, grant_live, withdrawn)
        {
            if named != target {
                assert_eq!(
                    out.refusal,
                    Some(Refusal::LeaseMismatch),
                    "a wrong use was not refused: {context}"
                );
            }
        }

        if withdrawn {
            self.request = None;
        }
        if withdrawn || matches!(event, Some(Event::LeaseUsed { .. })) {
            self.grant = None;
        }
        if let Some((at, released, target)) = self.request {
            if !inputs.panel_button_pressed {
                self.request = Some((at, true, target));
            } else if released && now - at < REQUEST_TTL_TICKS {
                self.grant = Some((now, target));
                self.request = None;
            }
        }
        if let Some(Event::RequestLease {
            class: HazardClass::Class3,
            target,
        }) = event
        {
            let free = !request_open && grant_live.is_none();
            if armed_before && !withdrawn && free && target.port.exists() {
                self.request = Some((now, !inputs.panel_button_pressed, target));
            }
        }

        let rail_on = out.rail_a || out.rail_b.is_some();
        // After a restart no rail is on until the arm switch goes off, then
        // on; and after one without a clean record, not before a panel clear.
        if rail_on {
            assert!(
                self.arm_seen_off && inputs.arm_switch_on,
                "a rail on without a fresh arm sequence: {context}"
            );
            assert!(
                !self.must_clear,
                "a rail on after an unproven restart with no panel clear: {context}"
            );
            // No rail stays on without correct heartbeat replies.
            assert!(
                self.beat_ok,
                "a rail on without a correct reply since the last loss: {context}"
            );
            assert!(
                self.last_good.is_some_and(|at| now - at < 2 * P),
                "a rail on two periods after the last good reply: {context}"
            );
        }
        // Rail A comes on only as the arm switch goes from off to on: no
        // reply, clear or other event can bring it on by itself.
        if out.rail_a && !self.rail_a_was_on {
            assert!(
                inputs.arm_switch_on && !self.arm_switch_was_on,
                "rail A came on without the arm switch going on: {context}"
            );
        }
        // Rail B on implies a live lease that a button press granted, for
        // this port.
        if let Some(port) = out.rail_b {
            let live = self
                .grant
                .is_some_and(|(at, target)| now - at < LEASE_TTL_TICKS && target.port == port);
            assert!(live, "rail B without a live lease for its port: {context}");
            assert!(
                self.rail_b == Some(port) || inputs.panel_button_pressed,
                "rail B came on without a press: {context}"
            );
        }
        // A hazard latches.
        if hazard {
            assert!(out.fault.is_some(), "a hazard did not latch: {context}");
        }
        // A supervisor clear is refused, always.
        if event == Some(SUPERVISOR_CLEAR) {
            assert_eq!(
                out.refusal,
                Some(Refusal::ClearNotFromPanel),
                "a supervisor clear was not refused: {context}"
            );
        }
        // A fault is latched: only a panel clear with the button sensed, the
        // stop released, a live heartbeat, every rail dead and the arm switch
        // off can remove it; or a restart, if the fault was only unknown.
        let panel_clear = event == Some(PANEL_CLEAR)
            && inputs.panel_button_pressed
            && !inputs.estop_pressed
            && !inputs.arm_switch_on
            && self.beat_ok
            && !inputs.any_rail_sensed();
        if let Some(reason) = self.fault {
            let resolved = restart.is_some() && (reason == UNKNOWN || kept == State::POWER_ON.mode);
            if !panel_clear && !resolved {
                assert_eq!(out.fault, Some(reason), "a fault unlatched: {context}");
            }
        } else if !hazard && !restart_dirty {
            assert_eq!(out.fault, None, "a fault from nowhere: {context}");
        }
        if out.fault.is_none() {
            assert!(
                !self.must_clear || panel_clear,
                "an unproven restart was left without a panel clear: {context}"
            );
            self.must_clear = false;
        }

        // Record what this tick commanded, for the next tick's feedback.
        self.commanded = Commanded {
            rail_a_since: if out.rail_a {
                self.commanded
                    .rail_a_since
                    .filter(|_| restart.is_none())
                    .or(Some(now))
            } else {
                None
            },
            rail_b: match out.rail_b {
                Some(port) => match self.commanded.rail_b {
                    Some((was, at)) if was == port && restart.is_none() => Some((port, at)),
                    _ => Some((port, now)),
                },
                None => None,
            },
        };
        self.fault = out.fault;
        self.rail_b = out.rail_b;
        self.rail_a_was_on = out.rail_a;
        self.arm_switch_was_on = inputs.arm_switch_on;
    }
}

/// The supervisor as the interlock sees it over the link. It answers each
/// new challenge once, usually on the next sample, and now and then sends a
/// wrong, repeated or stale reply, answers late, or freezes.
#[derive(Default)]
struct Supervisor {
    /// The challenge it last answered.
    answered: Option<HeartbeatChallenge>,
    /// A reply waiting to go out, and how many samples until it does.
    pending: Option<(HeartbeatChallenge, u32)>,
    /// Samples left frozen.
    frozen: u32,
    current: Option<HeartbeatChallenge>,
    previous: Option<HeartbeatChallenge>,
    sent: Option<HeartbeatReply>,
}

impl Supervisor {
    /// The reply on the next sample. A quiet sample injects no fault.
    fn reply(
        &mut self,
        rng: &mut Rng,
        shown: HeartbeatChallenge,
        restart: bool,
        quiet: bool,
    ) -> Option<HeartbeatReply> {
        if self.current != Some(shown) {
            self.previous = self.current;
            self.current = Some(shown);
        }
        // Nothing is in flight across a reset of the link.
        if restart {
            self.pending = None;
            return None;
        }
        if quiet && self.frozen == 0 {
            if self.answered == Some(shown) {
                return None;
            }
            self.pending = None;
            self.answered = Some(shown);
            self.sent = Some(shown.reply());
            return self.sent;
        }
        if self.frozen > 0 {
            self.frozen -= 1;
            return None;
        }
        if rng.one_in(500) {
            self.frozen = 5 + (rng.next() % 40) as u32;
            return None;
        }
        let reply = match rng.next() % 1_000 {
            0..=2 => Some(HeartbeatReply(shown.reply().0.rotate_left(7))),
            3..=5 => self.sent,
            6..=8 => self.previous.map(HeartbeatChallenge::reply),
            _ => {
                if self.answered != Some(shown) && self.pending.is_none_or(|(c, _)| c != shown) {
                    let delay = match rng.next() % 100 {
                        0 => 10 + (rng.next() % 5) as u32,
                        1..=9 => 1 + (rng.next() % 3) as u32,
                        _ => 0,
                    };
                    self.pending = Some((shown, delay));
                }
                match self.pending {
                    Some((challenge, 0)) => {
                        self.pending = None;
                        self.answered = Some(challenge);
                        Some(challenge.reply())
                    }
                    Some((challenge, wait)) => {
                        self.pending = Some((challenge, wait - 1));
                        None
                    }
                    None => None,
                }
            }
        };
        if reply.is_some() {
            self.sent = reply;
        }
        reply
    }
}

/// The rails as hardware: each closes some ticks after it is commanded on,
/// usually inside the settle window, and drops at once when commanded off.
/// Now and then a rail sticks on or drops out.
#[derive(Default)]
struct Plant {
    /// Rail A: the tick it was commanded on and its closing delay.
    rail_a: Option<(Tick, Tick)>,
    /// Rail B: the port, the tick it was commanded on and its closing delay.
    rail_b: Option<(Port, Tick, Tick)>,
    /// A rail that is stuck on.
    stuck: Option<Rail>,
}

impl Plant {
    fn delay(rng: &mut Rng) -> Tick {
        if rng.one_in(25) {
            OUTPUT_SETTLE_TICKS + 1 + rng.next() % 3
        } else {
            rng.next() % (OUTPUT_SETTLE_TICKS + 1)
        }
    }

    /// Follows what a tick commanded.
    fn follow(&mut self, rng: &mut Rng, out: Outputs, now: Tick) {
        self.rail_a = match (out.rail_a, self.rail_a) {
            (false, _) => None,
            (true, Some(on)) => Some(on),
            (true, None) => Some((now, Self::delay(rng))),
        };
        self.rail_b = match (out.rail_b, self.rail_b) {
            (None, _) => None,
            (Some(port), Some(on)) if on.0 == port => Some(on),
            (Some(port), _) => Some((port, now, Self::delay(rng))),
        };
    }

    /// The feedback on a tick. After a restart every output has been off
    /// since reset, so only a stuck rail is live.
    /// A quiet sample injects no new fault.
    fn sense(
        &mut self,
        rng: &mut Rng,
        now: Tick,
        restart: bool,
        quiet: bool,
    ) -> (bool, [bool; RAIL_B_PORTS]) {
        if restart {
            self.rail_a = None;
            self.rail_b = None;
        }
        let closed = |since: Tick, delay: Tick| now - since >= delay;
        if quiet {
            let mut rail_b = [false; RAIL_B_PORTS];
            if let Some((port, since, d)) = self.rail_b {
                rail_b[usize::from(port.0)] = closed(since, d);
            }
            let mut rail_a = self.rail_a.is_some_and(|(since, d)| closed(since, d));
            match self.stuck {
                Some(Rail::A) => rail_a = true,
                Some(Rail::B(port)) => rail_b[usize::from(port.0)] = true,
                None => {}
            }
            return (rail_a, rail_b);
        }
        if self.stuck.is_some() && rng.one_in(4) {
            self.stuck = None;
        } else if self.stuck.is_none() && rng.one_in(500) {
            self.stuck = Some(if rng.one_in(2) {
                Rail::A
            } else {
                Rail::B(Port((rng.next() % RAIL_B_PORTS as u64) as u8))
            });
        }
        let mut rail_a = self.rail_a.is_some_and(|(since, d)| closed(since, d));
        if rail_a && rng.one_in(1_000) {
            rail_a = false;
        }
        let mut rail_b = [false; RAIL_B_PORTS];
        if let Some((port, since, d)) = self.rail_b {
            rail_b[usize::from(port.0)] = closed(since, d);
        }
        match self.stuck {
            Some(Rail::A) => rail_a = true,
            Some(Rail::B(port)) => rail_b[usize::from(port.0)] = true,
            None => {}
        }
        (rail_a, rail_b)
    }
}

/// How far the clock moves before the next drawn sample. Most samples are one
/// tick apart; a longer gap is filled with quiet samples, since the firmware
/// loop samples every tick. Gaps as long as a lease or a request are drawn
/// only while one is open, where they matter.
fn next_advance(rng: &mut Rng, state: State) -> Tick {
    let (live, pending) = match state.mode {
        Mode::Armed { approval, .. } => (
            matches!(approval, Approval::Live { .. }),
            matches!(
                approval,
                Approval::AwaitPress { .. } | Approval::AwaitRelease { .. }
            ),
        ),
        _ => (false, false),
    };
    match rng.next() % 64 {
        0..=1 => 0,
        2..=3 => 2 + rng.next() % 30,
        4..=7 if live => LEASE_TTL_TICKS - 1 + rng.next() % 3,
        4..=5 if pending => REQUEST_TTL_TICKS - 1 + rng.next() % 3,
        _ => 1,
    }
}

/// Draws the next sample, `advance` ticks on. Switches hold their position
/// for a while, so the sequence spends time in every state instead of
/// faulting on every tick.
fn next_inputs(
    rng: &mut Rng,
    plant: &mut Plant,
    prev: Inputs,
    advance: Tick,
    restart: bool,
    heartbeat: Option<HeartbeatReply>,
) -> Inputs {
    let now = prev.now + advance;
    let (rail_a_sensed, rail_b_sensed) = plant.sense(rng, now, restart, false);
    Inputs {
        estop_pressed: if prev.estop_pressed {
            !rng.one_in(3)
        } else {
            rng.one_in(150)
        },
        heartbeat,
        arm_switch_on: prev.arm_switch_on ^ rng.one_in(if prev.arm_switch_on { 60 } else { 4 }),
        key_switch_on: prev.key_switch_on ^ rng.one_in(if prev.key_switch_on { 60 } else { 4 }),
        panel_button_pressed: prev.panel_button_pressed ^ rng.one_in(3),
        firmware_jumper_fitted: prev.firmware_jumper_fitted
            ^ rng.one_in(if prev.firmware_jumper_fitted { 6 } else { 400 }),
        rail_a_sensed,
        rail_b_sensed,
        now,
    }
}

/// Draws the next event. A use names the live lease's target half the time,
/// so both right and wrong uses are common. A restart reads back the record
/// that was written most of the time, and sometimes a torn or wrong one.
fn next_event(
    rng: &mut Rng,
    state: State,
    record: FaultRecord,
    targets: &[LeaseTarget],
) -> Option<Event> {
    match rng.next() % 100 {
        0 => {
            let record = match rng.next() % 10 {
                0 => None,
                1 => Some(FaultRecord::Clear),
                2 => Some(FaultRecord::Armed),
                _ => Some(record),
            };
            Some(Event::Restart {
                record,
                nonce: rng.next(),
            })
        }
        1..=5 => Some(Event::RequestArm),
        6..=25 => Some(Event::RequestLease {
            class: HazardClass::Class3,
            target: rng.pick(targets),
        }),
        26..=30 => Some(Event::RequestLease {
            class: rng.pick(&CLASSES),
            target: rng.pick(targets),
        }),
        31..=35 => {
            let live = match state.mode {
                Mode::Armed {
                    approval: Approval::Live { target, .. },
                    ..
                } => Some(target),
                _ => None,
            };
            let target = match live {
                Some(target) if rng.one_in(2) => target,
                _ => rng.pick(targets),
            };
            Some(Event::LeaseUsed { target })
        }
        36..=45 => Some(PANEL_CLEAR),
        46..=48 => Some(SUPERVISOR_CLEAR),
        _ => None,
    }
}

/// Counts how often each condition in `random_sequences_keep_the_invariants`
/// was reached on one step.
#[allow(clippy::too_many_arguments)]
fn tally(
    counts: &mut [u32; 21],
    kept: State,
    inputs: Inputs,
    event: Option<Event>,
    next: State,
    out: Outputs,
    lost_how: u8,
    power_loss: bool,
) {
    let restart = restart_of(event).is_some();
    let was_faulted = matches!(kept.mode, Mode::Faulted(_));
    let newly = |reason: &dyn Fn(FaultReason) -> bool| {
        u32::from(!was_faulted && out.fault.is_some_and(reason))
    };
    counts[0] += u32::from(out.rail_a);
    counts[1] += u32::from(out.rail_b.is_some());
    counts[2] += u32::from(out.rail_b == Some(Port(LAST_PORT)));
    counts[3] += u32::from(out.fault.is_some());
    counts[4] += u32::from(was_faulted && out.fault.is_none());
    if let Mode::Armed {
        approval: Approval::Live { granted_at, .. },
        ..
    } = kept.mode
    {
        let ran_out = inputs.now - granted_at >= LEASE_TTL_TICKS;
        counts[5] += u32::from(
            ran_out
                && inputs.key_switch_on
                && matches!(
                    next.mode,
                    Mode::Armed {
                        approval: Approval::Idle,
                        ..
                    }
                ),
        );
    }
    counts[6] += u32::from(restart);
    counts[7] += newly(&|r| {
        matches!(
            r,
            FaultReason::OutputMismatch {
                sensed_live: true,
                ..
            }
        )
    });
    counts[8] += newly(&|r| {
        matches!(
            r,
            FaultReason::OutputMismatch {
                sensed_live: false,
                ..
            }
        )
    });
    counts[9] += newly(&|r| r == FaultReason::FirmwareJumperWhileArmed);
    counts[10] += u32::from(out.firmware_update_allowed);
    counts[11] += u32::from(out.refusal == Some(Refusal::LeaseMismatch));
    counts[12] += u32::from(
        matches!(event, Some(Event::LeaseUsed { .. }))
            && matches!(
                kept.mode,
                Mode::Armed {
                    approval: Approval::Live { .. },
                    ..
                }
            )
            && out.refusal.is_none()
            && out.rail_a,
    );
    counts[13] += u32::from(power_loss);
    counts[14] += u32::from(restart && out.fault == Some(UNKNOWN));
    counts[15] += u32::from(
        kept.mode == Mode::Faulted(UNKNOWN) && out.fault.is_none() && event == Some(PANEL_CLEAR),
    );
    counts[16] += u32::from(out.refusal == Some(Refusal::ClearNotFromPanel));
    counts[17] += u32::from(out.refusal == Some(Refusal::PanelButtonNotPressed));
    counts[18] += u32::from(lost_how == 1);
    counts[19] += u32::from(lost_how == 2);
    counts[20] += u32::from(
        kept.mode == Mode::Disarmed && next.mode == Mode::AwaitArmOff && inputs.arm_switch_on,
    );
}

#[test]
fn random_sequences_keep_the_invariants() {
    const SEEDS: u64 = 200;
    const STEPS: u32 = 5_000;

    // How often each condition was reached, so a quiet pass cannot be an
    // empty one.
    let names = [
        "rail A on",
        "rail B on",
        "rail B on the last port",
        "fault latched",
        "fault cleared",
        "lease expired",
        "restart",
        "stuck-on fault",
        "dead-rail fault",
        "jumper-while-armed fault",
        "firmware update allowed",
        "wrong use refused",
        "right use spent",
        "power loss",
        "unknown after restart",
        "unknown cleared at the panel",
        "supervisor clear refused",
        "panel clear without the button refused",
        "heartbeat lost to a bad reply",
        "heartbeat lost to a missed deadline",
        "arm refused without a heartbeat",
    ];
    // The firmware floor is lower than before #198: long gaps are now drawn
    // only with a lease or request open, so fewer quiet ticks pass with the
    // jumper fitted. Arming with no heartbeat yet is a narrow window (a clean
    // restart, then the switch on before the first reply); the table tests
    // cover it exactly.
    let floors = [
        1_000, 1_000, 100, 1_000, 1_000, 1_000, 1_000, 100, 100, 100, 500, 100, 100, 1_000, 1_000,
        100, 1_000, 100, 1_000, 1_000, 20,
    ];
    let mut counts = [0u32; 21];

    // Targets on ports 0, 1 and the last, plus one that does not exist.
    let mut targets = targets();
    targets.push(LeaseTarget {
        port: Port(1),
        action_digest: DIGESTS[1],
    });

    for seed in 1..=SEEDS {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut oracle = Oracle::default();
        let mut plant = Plant::default();
        let mut supervisor = Supervisor::default();
        let mut inputs = Inputs {
            estop_pressed: false,
            arm_switch_on: seed.is_multiple_of(2),
            key_switch_on: seed.is_multiple_of(3),
            heartbeat: None,
            panel_button_pressed: false,
            rail_a_sensed: false,
            rail_b_sensed: [false; RAIL_B_PORTS],
            firmware_jumper_fitted: false,
            now: seed,
        };
        // Boot as the firmware does: a restart first, with whatever the
        // memory holds (here, often nothing).
        let first = if seed.is_multiple_of(4) {
            None
        } else {
            Some(FaultRecord::Clear)
        };
        let mut event = Some(Event::Restart {
            record: first,
            nonce: rng.next(),
        });
        let mut state = State::POWER_ON;
        let mut last = state.outputs();
        let mut record = state.fault_record();
        for index in 0..STEPS {
            if index > 0 {
                event = next_event(&mut rng, state, record, &targets);
            }
            // The first step is the boot, with nothing before it.
            let advance = if index == 0 {
                1
            } else {
                next_advance(&mut rng, state)
            };
            // The quiet ticks of a gap: same switches, no event, no new fault
            // injected, the supervisor answering.
            for _ in 1..advance {
                let reply = supervisor.reply(&mut rng, last.challenge, false, true);
                let now = inputs.now + 1;
                let (rail_a_sensed, rail_b_sensed) = plant.sense(&mut rng, now, false, true);
                inputs = Inputs {
                    heartbeat: reply,
                    rail_a_sensed,
                    rail_b_sensed,
                    now,
                    ..inputs
                };
                let (next, out) = step(state, inputs, None);
                let context = || {
                    format!("seed {seed} step {index} (quiet): {state:?} + {inputs:?} -> {next:?}")
                };
                check_tick(state, inputs, None, next, out, &context);
                oracle.check(state.mode, inputs, None, out, &context());
                plant.follow(&mut rng, out, inputs.now);
                tally(
                    &mut counts,
                    state,
                    inputs,
                    None,
                    next,
                    out,
                    oracle.lost_how,
                    false,
                );
                state = next;
                last = out;
            }
            let restart = restart_of(event).is_some();
            // Half the restarts lose RAM, so only the record remains.
            let power_loss = restart && (index == 0 || rng.one_in(2));
            let kept = if power_loss { State::POWER_ON } else { state };
            let reply = supervisor.reply(&mut rng, last.challenge, restart, false);
            inputs = next_inputs(&mut rng, &mut plant, inputs, advance.min(1), restart, reply);
            // The panel driver makes a clear token from a press, mostly.
            if event == Some(PANEL_CLEAR) && !rng.one_in(10) {
                inputs.panel_button_pressed = true;
            }
            let (next, out) = step(kept, inputs, event);
            let context = || {
                format!("seed {seed} step {index}: {kept:?} + {inputs:?} + {event:?} -> {next:?}")
            };
            check_tick(kept, inputs, event, next, out, &context);
            oracle.check(kept.mode, inputs, event, out, &context());
            plant.follow(&mut rng, out, inputs.now);
            tally(
                &mut counts,
                kept,
                inputs,
                event,
                next,
                out,
                oracle.lost_how,
                power_loss,
            );
            state = next;
            last = out;
            record = next.fault_record();
        }
    }

    for ((name, count), floor) in names.iter().zip(counts).zip(floors) {
        assert!(count >= floor, "{name} was reached only {count} times");
    }
}
