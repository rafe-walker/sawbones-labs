//! Safety invariants, checked two ways: exhaustively over every single step
//! from a set of representative states, and over long random input sequences
//! against an independent record of what the sequence so far permits.

use sawbones_interlock::{
    step, Approval, Event, FaultReason, HazardClass, Inputs, Outputs, PanelState, Refusal, State,
    Tick, LEASE_TTL_TICKS, REQUEST_TTL_TICKS,
};

const CLASSES: [HazardClass; 5] = [
    HazardClass::Class0,
    HazardClass::Class1,
    HazardClass::Class2,
    HazardClass::Class3,
    HazardClass::Class4,
];

/// Every event, and no event.
fn all_events() -> Vec<Option<Event>> {
    let mut events = vec![
        None,
        Some(Event::Restart),
        Some(Event::RequestArm),
        Some(Event::LeaseUsed),
        Some(Event::ClearFault),
    ];
    events.extend(CLASSES.map(|class| Some(Event::RequestLease { class })));
    events
}

/// All 32 combinations of the five sampled booleans.
fn all_inputs(now: Tick) -> impl Iterator<Item = Inputs> {
    (0u8..32).map(move |bits| Inputs {
        estop_pressed: bits & 1 != 0,
        arm_switch_on: bits & 2 != 0,
        key_switch_on: bits & 4 != 0,
        heartbeat_healthy: bits & 8 != 0,
        panel_button_pressed: bits & 16 != 0,
        now,
    })
}

/// The invariants that can be judged from one tick alone.
fn check_tick(inputs: Inputs, event: Option<Event>, next: State, out: Outputs, context: &str) {
    let rail_on = out.rail_a || out.rail_b;
    assert!(
        !out.rail_b || out.rail_a,
        "rail B without rail A: {context}"
    );
    assert!(
        !out.rail_b || inputs.key_switch_on,
        "rail B without the key: {context}"
    );
    if rail_on {
        assert_eq!(out.fault, None, "a rail on with a fault: {context}");
        assert!(
            !inputs.estop_pressed,
            "a rail on with the stop pressed: {context}"
        );
        assert!(
            inputs.heartbeat_healthy,
            "a rail on without a heartbeat: {context}"
        );
        assert!(
            inputs.arm_switch_on,
            "a rail on with the arm switch off: {context}"
        );
    }
    if inputs.estop_pressed || !inputs.heartbeat_healthy {
        assert!(
            out.fault.is_some(),
            "a hazard that did not latch: {context}"
        );
    }
    assert_eq!(
        out.fault.is_some(),
        out.panel == PanelState::Fault,
        "panel and fault disagree: {context}"
    );
    if event == Some(Event::Restart) {
        assert!(!rail_on, "a rail on at a restart: {context}");
    }
    if let Some(Event::RequestLease { class }) = event {
        let want = match class {
            HazardClass::Class4 => Some(Some(Refusal::ClassNeverPermitted)),
            HazardClass::Class3 => None,
            _ => Some(Some(Refusal::NoLeaseForClass)),
        };
        if let Some(want) = want {
            assert_eq!(out.refusal, want, "class refusal: {context}");
        }
    }
    // The outputs are a function of the state alone, apart from the refusal.
    let from_state = Outputs {
        refusal: out.refusal,
        ..next.outputs()
    };
    assert_eq!(
        out, from_state,
        "outputs not derived from the state: {context}"
    );
}

#[test]
fn every_single_step_from_representative_states_is_safe() {
    let now: Tick = 1_000;
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
        State::Armed(Approval::Idle),
        State::Faulted(FaultReason::EmergencyStop),
        State::Faulted(FaultReason::HeartbeatLost),
    ];
    for at in stamps {
        states.push(State::Armed(Approval::AwaitRelease { requested_at: at }));
        states.push(State::Armed(Approval::AwaitPress { requested_at: at }));
        states.push(State::Armed(Approval::Live { granted_at: at }));
    }

    let within = |since: Tick, ttl: Tick| since <= now && now - since < ttl;
    let mut checked = 0u32;
    for &state in &states {
        for inputs in all_inputs(now) {
            for event in all_events() {
                let (next, out) = step(state, inputs, event);
                let context = format!("{state:?} + {inputs:?} + {event:?} -> {next:?}");
                check_tick(inputs, event, next, out, &context);
                assert_eq!(step(state, inputs, event), (next, out), "not deterministic");

                // Rail A needs the switch to have been seen off already.
                if out.rail_a {
                    assert!(
                        matches!(state, State::Disarmed | State::Armed(_)),
                        "armed without a sequence: {context}"
                    );
                }
                // Rail B needs a lease still in its window, or a fresh press
                // answering a request still in its window.
                if out.rail_b {
                    let permitted = match state {
                        State::Armed(Approval::Live { granted_at }) => {
                            within(granted_at, LEASE_TTL_TICKS) && event != Some(Event::LeaseUsed)
                        }
                        State::Armed(Approval::AwaitPress { requested_at }) => {
                            inputs.panel_button_pressed && within(requested_at, REQUEST_TTL_TICKS)
                        }
                        _ => false,
                    };
                    assert!(permitted, "rail B without a lease: {context}");
                }
                // A latched fault leaves only by an explicit clear.
                if let State::Faulted(reason) = state {
                    let clear = event == Some(Event::ClearFault)
                        && !inputs.estop_pressed
                        && !inputs.arm_switch_on
                        && inputs.heartbeat_healthy;
                    let want = if clear {
                        State::Disarmed
                    } else {
                        State::Faulted(reason)
                    };
                    assert_eq!(next, want, "latch: {context}");
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 29 * 32 * 10);
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
}

/// What the sequence so far permits, kept as plain flags with no knowledge of
/// the machine's states. It may permit more than the machine does, never less.
#[derive(Default)]
struct Oracle {
    /// The arm switch has been seen off since the last restart or hazard.
    arm_seen_off: bool,
    /// A class 3 request that a press could still answer: the tick it was
    /// made and whether the button has been seen released since.
    request: Option<(Tick, bool)>,
    /// The tick of the press that could have granted the current lease.
    grant: Option<Tick>,
    /// The outputs of the previous tick.
    fault: Option<FaultReason>,
    rail_b: bool,
}

impl Oracle {
    /// Folds one tick in, then checks the machine's outputs against it.
    fn check(&mut self, inputs: Inputs, event: Option<Event>, out: Outputs, context: &str) {
        let now = inputs.now;
        let restart = event == Some(Event::Restart);
        let hazard = inputs.estop_pressed || !inputs.heartbeat_healthy;

        if restart || hazard {
            self.arm_seen_off = false;
        }
        if !inputs.arm_switch_on && !hazard {
            self.arm_seen_off = true;
        }

        let withdrawn = restart || hazard || !inputs.arm_switch_on || !inputs.key_switch_on;
        if withdrawn {
            self.request = None;
        }
        if withdrawn || event == Some(Event::LeaseUsed) {
            self.grant = None;
        }
        if let Some((at, released)) = self.request {
            if !inputs.panel_button_pressed {
                self.request = Some((at, true));
            } else if released && now - at < REQUEST_TTL_TICKS {
                self.grant = Some(now);
                self.request = None;
            }
        }
        let class3 = Some(Event::RequestLease {
            class: HazardClass::Class3,
        });
        if event == class3 && !withdrawn {
            self.request = Some((now, !inputs.panel_button_pressed));
        }

        // After a restart no rail is on until the arm switch goes off, then on.
        if out.rail_a || out.rail_b {
            assert!(
                self.arm_seen_off && inputs.arm_switch_on,
                "a rail on without a fresh arm sequence: {context}"
            );
        }
        // Rail B on implies a live lease that a button press granted.
        if out.rail_b {
            let live = self.grant.is_some_and(|at| now - at < LEASE_TTL_TICKS);
            assert!(live, "rail B without a live lease: {context}");
            assert!(
                self.rail_b || inputs.panel_button_pressed,
                "rail B came on without a press: {context}"
            );
        }
        // A fault is latched: only an explicit clear with the stop released
        // and the arm switch off can remove it.
        if let Some(reason) = self.fault {
            let may_clear =
                event == Some(Event::ClearFault) && !inputs.estop_pressed && !inputs.arm_switch_on;
            if !may_clear {
                assert_eq!(out.fault, Some(reason), "a fault unlatched: {context}");
            }
        } else if !hazard {
            assert_eq!(out.fault, None, "a fault from nowhere: {context}");
        }

        self.fault = out.fault;
        self.rail_b = out.rail_b;
    }
}

/// Draws the next sample. Switches hold their position for a while, so the
/// sequence spends time in every state instead of faulting on every tick.
fn next_inputs(rng: &mut Rng, prev: Inputs) -> Inputs {
    let advance = match rng.next() % 64 {
        0..=3 => 0,
        4..=55 => 1,
        56..=61 => 2 + rng.next() % 30,
        62 => LEASE_TTL_TICKS - 1 + rng.next() % 3,
        _ => REQUEST_TTL_TICKS - 1 + rng.next() % 3,
    };
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
        now: prev.now + advance,
    }
}

fn next_event(rng: &mut Rng) -> Option<Event> {
    match rng.next() % 100 {
        0 => Some(Event::Restart),
        1..=5 => Some(Event::RequestArm),
        6..=25 => Some(Event::RequestLease {
            class: HazardClass::Class3,
        }),
        26..=30 => Some(Event::RequestLease {
            class: CLASSES[(rng.next() % 5) as usize],
        }),
        31..=33 => Some(Event::LeaseUsed),
        34..=45 => Some(Event::ClearFault),
        _ => None,
    }
}

#[test]
fn random_sequences_keep_the_invariants() {
    const SEEDS: u64 = 200;
    const STEPS: u32 = 5_000;

    // How often each condition was reached, so a quiet pass cannot be an
    // empty one.
    let (mut rail_a, mut rail_b, mut faults, mut clears, mut expiries, mut restarts) =
        (0u32, 0u32, 0u32, 0u32, 0u32, 0u32);

    for seed in 1..=SEEDS {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut state = State::POWER_ON;
        let mut oracle = Oracle::default();
        let mut inputs = Inputs {
            estop_pressed: false,
            arm_switch_on: seed.is_multiple_of(2),
            key_switch_on: seed.is_multiple_of(3),
            heartbeat_healthy: true,
            panel_button_pressed: false,
            now: seed,
        };
        for index in 0..STEPS {
            inputs = next_inputs(&mut rng, inputs);
            let event = next_event(&mut rng);
            let (next, out) = step(state, inputs, event);
            let context =
                format!("seed {seed} step {index}: {state:?} + {inputs:?} + {event:?} -> {next:?}");
            check_tick(inputs, event, next, out, &context);
            oracle.check(inputs, event, out, &context);

            rail_a += u32::from(out.rail_a);
            rail_b += u32::from(out.rail_b);
            faults += u32::from(out.fault.is_some());
            restarts += u32::from(event == Some(Event::Restart));
            clears += u32::from(matches!(state, State::Faulted(_)) && out.fault.is_none());
            if let State::Armed(Approval::Live { granted_at }) = state {
                let ran_out = inputs.now - granted_at >= LEASE_TTL_TICKS;
                expiries += u32::from(
                    ran_out && inputs.key_switch_on && next == State::Armed(Approval::Idle),
                );
            }
            state = next;
        }
    }

    let floor = 1_000;
    for (name, count) in [
        ("rail A on", rail_a),
        ("rail B on", rail_b),
        ("fault latched", faults),
        ("fault cleared", clears),
        ("lease expired", expiries),
        ("restart", restarts),
    ] {
        assert!(count >= floor, "{name} was reached only {count} times");
    }
}
