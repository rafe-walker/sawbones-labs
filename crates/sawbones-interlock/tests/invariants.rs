//! Safety invariants, checked two ways: exhaustively over every single step
//! from a set of representative states, and over long random input sequences
//! against an independent record of what the sequence so far permits. A third
//! test pins the whole single-step transition table to [`RULES_VERSION`].

use sawbones_interlock::{
    step, ActionDigest, Approval, Event, FaultReason, HazardClass, Inputs, LeaseTarget, Outputs,
    PanelState, Port, Rail, Refusal, State, Tick, LEASE_TTL_TICKS, OUTPUT_SETTLE_TICKS,
    RAIL_B_PORTS, REQUEST_TTL_TICKS, RULES_VERSION,
};

const CLASSES: [HazardClass; 5] = [
    HazardClass::Class0,
    HazardClass::Class1,
    HazardClass::Class2,
    HazardClass::Class3,
    HazardClass::Class4,
];

const DIGESTS: [ActionDigest; 2] = [ActionDigest([0xa1; 32]), ActionDigest([0xb2; 32])];

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

/// Every kind of event, and no event.
fn all_events() -> Vec<Option<Event>> {
    let mut events = vec![
        None,
        Some(Event::Restart),
        Some(Event::RequestArm),
        Some(Event::ClearFault),
    ];
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

/// All 128 combinations of the seven sampled booleans, with each rail B
/// feedback pattern.
fn all_inputs(now: Tick) -> impl Iterator<Item = Inputs> {
    (0u8..128).flat_map(move |bits| {
        rail_b_patterns().map(move |rail_b_sensed| Inputs {
            estop_pressed: bits & 1 != 0,
            arm_switch_on: bits & 2 != 0,
            key_switch_on: bits & 4 != 0,
            heartbeat_healthy: bits & 8 != 0,
            panel_button_pressed: bits & 16 != 0,
            rail_a_sensed: bits & 32 != 0,
            firmware_jumper_fitted: bits & 64 != 0,
            rail_b_sensed,
            now,
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
    /// From a state, for the single-step test.
    fn of(state: State) -> Self {
        match state {
            State::Armed { since, approval } => Commanded {
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

/// The invariants that can be judged from one tick alone. `driven` is what
/// was commanded before the tick, `armed_before` whether rail A was on.
#[allow(clippy::too_many_arguments)]
fn check_tick(
    driven: Commanded,
    armed_before: bool,
    inputs: Inputs,
    event: Option<Event>,
    next: State,
    out: Outputs,
    context: &dyn Fn() -> String,
) {
    let restart = event == Some(Event::Restart);
    let driven = if restart {
        Commanded::default()
    } else {
        driven
    };
    let disagrees = driven.disagrees(&inputs);
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
            inputs.heartbeat_healthy,
            "a rail on without a heartbeat: {}",
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
    if inputs.estop_pressed || !inputs.heartbeat_healthy || disagrees {
        assert!(
            out.fault.is_some(),
            "a hazard that did not latch: {}",
            context()
        );
    }
    if inputs.firmware_jumper_fitted && armed_before && !restart {
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
    if restart {
        assert!(!rail_on, "a rail on at a restart: {}", context());
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

/// The representative states for the single-step tests.
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
    let mut states = vec![
        State::AwaitArmOff,
        State::Disarmed,
        State::FirmwareUpdate,
        State::Faulted(FaultReason::EmergencyStop),
        State::Faulted(FaultReason::HeartbeatLost),
        State::Faulted(FaultReason::FirmwareJumperWhileArmed),
        State::Faulted(FaultReason::OutputMismatch {
            rail: Rail::A,
            sensed_live: true,
        }),
        State::Faulted(FaultReason::OutputMismatch {
            rail: Rail::B(Port(LAST_PORT)),
            sensed_live: false,
        }),
    ];
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
    for since in [
        now,
        now - (OUTPUT_SETTLE_TICKS - 1),
        now - OUTPUT_SETTLE_TICKS,
    ] {
        states.push(State::Armed {
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
                    states.push(State::Armed { since, approval });
                }
            }
        }
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
        for inputs in all_inputs(now) {
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
        let armed_before = matches!(state, State::Armed { .. });
        check_tick(
            Commanded::of(state),
            armed_before,
            inputs,
            event,
            next,
            out,
            &context,
        );
        assert_eq!(step(state, inputs, event), (next, out), "not deterministic");

        // Rail A needs the switch to have been seen off already.
        if out.rail_a {
            assert!(
                matches!(state, State::Disarmed | State::Armed { .. }),
                "armed without a sequence: {}",
                context()
            );
        }
        // Rail B needs a lease still in its window, or a fresh press
        // answering a request still in its window, for that very port.
        if let Some(port) = out.rail_b {
            let permitted = match state {
                State::Armed {
                    approval: Approval::Live { granted_at, target },
                    ..
                } => {
                    within(granted_at, LEASE_TTL_TICKS)
                        && !matches!(event, Some(Event::LeaseUsed { .. }))
                        && target.port == port
                }
                State::Armed {
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
            State::Armed {
                approval: Approval::Live { target, .. },
                ..
            },
            Some(Event::LeaseUsed { target: named }),
        ) = (state, event)
        {
            assert_eq!(out.rail_b, None, "a use left rail B on: {}", context());
            let live = inputs.key_switch_on
                && inputs.arm_switch_on
                && out.fault.is_none()
                && matches!(next, State::Armed { .. });
            if live && named != target && out.refusal != Some(Refusal::NoLiveLease) {
                assert_eq!(
                    out.refusal,
                    Some(Refusal::LeaseMismatch),
                    "a wrong use not refused: {}",
                    context()
                );
            }
        }
        // A latched fault leaves only by an explicit clear.
        if let State::Faulted(reason) = state {
            let clear = event == Some(Event::ClearFault)
                && !inputs.estop_pressed
                && !inputs.arm_switch_on
                && inputs.heartbeat_healthy
                && !inputs.rail_a_sensed
                && !inputs.rail_b_sensed.contains(&true);
            let want = if clear {
                State::Disarmed
            } else {
                State::Faulted(reason)
            };
            assert_eq!(next, want, "latch: {}", context());
        }
        // The firmware state is reached only from a disarmed state.
        if out.firmware_update_allowed {
            assert!(
                matches!(
                    state,
                    State::AwaitArmOff | State::Disarmed | State::FirmwareUpdate
                ) || (event == Some(Event::Restart) && !matches!(state, State::Faulted(_))),
                "firmware allowed from an armed or faulted state: {}",
                context()
            );
        }
    });
    let states = representative_states(now).len() as u64;
    assert_eq!(checked, states * 512 * all_events().len() as u64);
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
        }
    }
    fn state(&mut self, state: State) {
        match state {
            State::AwaitArmOff => self.byte(0),
            State::Disarmed => self.byte(1),
            State::FirmwareUpdate => self.byte(2),
            State::Armed { since, approval } => {
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
            State::Faulted(reason) => {
                self.byte(4);
                self.fault(reason);
            }
        }
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
        });
    }
}

/// Every rule-set version and the hash of its single-step transition table,
/// oldest first. Version 1 (#150) predates this pin.
const TRANSITION_TABLES: &[(u32, u64)] = &[(2, 0xa1ce_7c5f_feda_0b92)];

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
}

impl Oracle {
    /// Folds one tick in, then checks the machine's outputs against it.
    fn check(&mut self, inputs: Inputs, event: Option<Event>, out: Outputs, context: &str) {
        let now = inputs.now;
        let restart = event == Some(Event::Restart);
        let armed_before = self.commanded.rail_a_since.is_some() && !restart;
        let driven = if restart {
            Commanded::default()
        } else {
            self.commanded
        };
        let mismatch = driven.disagrees(&inputs);
        let jumper = inputs.firmware_jumper_fitted;
        let hazard = inputs.estop_pressed
            || !inputs.heartbeat_healthy
            || mismatch
            || (jumper && armed_before);

        // The jumper takes the machine out of the arm sequence, except while
        // a fault is latched: a clear with the jumper fitted still sees the
        // arm switch off.
        let jumper_counts = jumper && self.fault.is_none();
        if restart || hazard || jumper_counts {
            self.arm_seen_off = false;
        }
        if !inputs.arm_switch_on && !hazard && !jumper_counts {
            self.arm_seen_off = true;
        }

        let withdrawn =
            restart || hazard || jumper || !inputs.arm_switch_on || !inputs.key_switch_on;
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

        // After a restart no rail is on until the arm switch goes off, then on.
        if out.rail_a || out.rail_b.is_some() {
            assert!(
                self.arm_seen_off && inputs.arm_switch_on,
                "a rail on without a fresh arm sequence: {context}"
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
        // A fault is latched: only an explicit clear with the stop released,
        // every rail dead and the arm switch off can remove it.
        if let Some(reason) = self.fault {
            let may_clear = event == Some(Event::ClearFault)
                && !inputs.estop_pressed
                && !inputs.arm_switch_on
                && !inputs.any_rail_sensed();
            if !may_clear {
                assert_eq!(out.fault, Some(reason), "a fault unlatched: {context}");
            }
        } else if !hazard {
            assert_eq!(out.fault, None, "a fault from nowhere: {context}");
        }

        // Record what this tick commanded, for the next tick's feedback.
        self.commanded = Commanded {
            rail_a_since: if out.rail_a {
                self.commanded
                    .rail_a_since
                    .filter(|_| !restart)
                    .or(Some(now))
            } else {
                None
            },
            rail_b: match out.rail_b {
                Some(port) => match self.commanded.rail_b {
                    Some((was, at)) if was == port && !restart => Some((port, at)),
                    _ => Some((port, now)),
                },
                None => None,
            },
        };
        self.fault = out.fault;
        self.rail_b = out.rail_b;
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
    fn sense(&mut self, rng: &mut Rng, now: Tick, restart: bool) -> (bool, [bool; RAIL_B_PORTS]) {
        if restart {
            self.rail_a = None;
            self.rail_b = None;
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
        let closed = |since: Tick, delay: Tick| now - since >= delay;
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

/// Draws the next sample. Switches hold their position for a while, so the
/// sequence spends time in every state instead of faulting on every tick.
fn next_inputs(rng: &mut Rng, plant: &mut Plant, prev: Inputs, restart: bool) -> Inputs {
    let advance = match rng.next() % 64 {
        0..=3 => 0,
        4..=55 => 1,
        56..=61 => 2 + rng.next() % 30,
        62 => LEASE_TTL_TICKS - 1 + rng.next() % 3,
        _ => REQUEST_TTL_TICKS - 1 + rng.next() % 3,
    };
    let now = prev.now + advance;
    let (rail_a_sensed, rail_b_sensed) = plant.sense(rng, now, restart);
    Inputs {
        estop_pressed: if prev.estop_pressed {
            !rng.one_in(3)
        } else {
            rng.one_in(150)
        },
        heartbeat_healthy: if prev.heartbeat_healthy {
            !rng.one_in(150)
        } else {
            rng.one_in(3)
        },
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
/// so both right and wrong uses are common.
fn next_event(rng: &mut Rng, state: State, targets: &[LeaseTarget]) -> Option<Event> {
    match rng.next() % 100 {
        0 => Some(Event::Restart),
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
            let live = match state {
                State::Armed {
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
        36..=47 => Some(Event::ClearFault),
        _ => None,
    }
}

#[test]
fn random_sequences_keep_the_invariants() {
    const SEEDS: u64 = 200;
    const STEPS: u32 = 5_000;

    // How often each condition was reached, so a quiet pass cannot be an
    // empty one.
    let mut counts = [0u32; 13];
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
    ];
    let floors = [
        1_000, 1_000, 100, 1_000, 1_000, 1_000, 1_000, 100, 100, 100, 1_000, 100, 100,
    ];

    // Targets on ports 0, 1 and the last, plus one that does not exist.
    let mut targets = targets();
    targets.push(LeaseTarget {
        port: Port(1),
        action_digest: DIGESTS[1],
    });

    for seed in 1..=SEEDS {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut state = State::POWER_ON;
        let mut oracle = Oracle::default();
        let mut plant = Plant::default();
        let mut inputs = Inputs {
            estop_pressed: false,
            arm_switch_on: seed.is_multiple_of(2),
            key_switch_on: seed.is_multiple_of(3),
            heartbeat_healthy: true,
            panel_button_pressed: false,
            rail_a_sensed: false,
            rail_b_sensed: [false; RAIL_B_PORTS],
            firmware_jumper_fitted: false,
            now: seed,
        };
        for index in 0..STEPS {
            let event = next_event(&mut rng, state, &targets);
            let restart = event == Some(Event::Restart);
            inputs = next_inputs(&mut rng, &mut plant, inputs, restart);
            let (next, out) = step(state, inputs, event);
            let context = || {
                format!("seed {seed} step {index}: {state:?} + {inputs:?} + {event:?} -> {next:?}")
            };
            check_tick(
                Commanded::of(state),
                matches!(state, State::Armed { .. }),
                inputs,
                event,
                next,
                out,
                &context,
            );
            oracle.check(inputs, event, out, &context());
            plant.follow(&mut rng, out, inputs.now);

            let newly = |reason: &dyn Fn(FaultReason) -> bool| {
                u32::from(!matches!(state, State::Faulted(_)) && out.fault.is_some_and(reason))
            };
            counts[0] += u32::from(out.rail_a);
            counts[1] += u32::from(out.rail_b.is_some());
            counts[2] += u32::from(out.rail_b == Some(Port(LAST_PORT)));
            counts[3] += u32::from(out.fault.is_some());
            counts[4] += u32::from(matches!(state, State::Faulted(_)) && out.fault.is_none());
            if let State::Armed {
                approval: Approval::Live { granted_at, .. },
                ..
            } = state
            {
                let ran_out = inputs.now - granted_at >= LEASE_TTL_TICKS;
                counts[5] += u32::from(
                    ran_out
                        && inputs.key_switch_on
                        && matches!(
                            next,
                            State::Armed {
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
                        state,
                        State::Armed {
                            approval: Approval::Live { .. },
                            ..
                        }
                    )
                    && out.refusal.is_none()
                    && out.rail_a,
            );
            state = next;
        }
    }

    for ((name, count), floor) in names.iter().zip(counts).zip(floors) {
        assert!(count >= floor, "{name} was reached only {count} times");
    }
}
