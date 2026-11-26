//! Table tests: one table per rule. Each row is a script of ticks and what the
//! outputs must be after the last one.

use sawbones_interlock::{
    step, Event, FaultReason, HazardClass, Inputs, Outputs, PanelState, Refusal, State,
    LEASE_TTL_TICKS, REQUEST_TTL_TICKS,
};

/// One tick of a script.
#[derive(Clone, Copy)]
struct Tick {
    inputs: Inputs,
    event: Option<Event>,
    /// How far the clock moves before this tick.
    advance: u64,
}

/// A healthy, idle sample: nothing pressed, every switch off.
const IDLE: Tick = Tick {
    inputs: Inputs {
        estop_pressed: false,
        arm_switch_on: false,
        key_switch_on: false,
        heartbeat_healthy: true,
        panel_button_pressed: false,
        now: 0,
    },
    event: None,
    advance: 1,
};

impl Tick {
    fn key(mut self) -> Self {
        self.inputs.key_switch_on = true;
        self
    }
    fn button(mut self) -> Self {
        self.inputs.panel_button_pressed = true;
        self
    }
    fn estop(mut self) -> Self {
        self.inputs.estop_pressed = true;
        self
    }
    fn no_heartbeat(mut self) -> Self {
        self.inputs.heartbeat_healthy = false;
        self
    }
    fn event(mut self, event: Event) -> Self {
        self.event = Some(event);
        self
    }
    fn after(mut self, ticks: u64) -> Self {
        self.advance = ticks;
        self
    }
}

/// What the last tick of a script must produce.
#[derive(Debug, PartialEq, Eq)]
struct Want {
    rail_a: bool,
    rail_b: bool,
    panel: PanelState,
    fault: Option<FaultReason>,
    refusal: Option<Refusal>,
}

const fn want(rail_a: bool, rail_b: bool, panel: PanelState) -> Want {
    Want {
        rail_a,
        rail_b,
        panel,
        fault: None,
        refusal: None,
    }
}

const fn fault(reason: FaultReason) -> Want {
    Want {
        rail_a: false,
        rail_b: false,
        panel: PanelState::Fault,
        fault: Some(reason),
        refusal: None,
    }
}

impl Want {
    const fn refused(mut self, why: Refusal) -> Self {
        self.refusal = Some(why);
        self
    }
}

const ALL_OFF_AWAIT: Want = want(false, false, PanelState::AwaitingArmOff);
const ALL_OFF_DISARMED: Want = want(false, false, PanelState::Disarmed);
const RAIL_A_ONLY: Want = want(true, false, PanelState::Armed);

const LEASE3: Event = Event::RequestLease {
    class: HazardClass::Class3,
};

/// Runs a script from power-on and returns the last tick's outputs.
fn run(script: &[Tick]) -> Outputs {
    let mut state = State::POWER_ON;
    let mut now = 0;
    let mut last = state.outputs();
    for tick in script {
        now += tick.advance;
        let inputs = Inputs { now, ..tick.inputs };
        let (next, outputs) = step(state, inputs, tick.event);
        state = next;
        last = outputs;
    }
    last
}

fn check(table: &[(&str, &[Tick], Want)]) {
    for (name, script, want) in table {
        let got = run(script);
        let got = Want {
            rail_a: got.rail_a,
            rail_b: got.rail_b,
            panel: got.panel,
            fault: got.fault,
            refusal: got.refusal,
        };
        assert_eq!(&got, want, "row: {name}");
    }
}

/// The shortest way to rail A: switch seen off, then on.
const ARMED: [Tick; 2] = [IDLE, ARM];
const ARM: Tick = Tick {
    inputs: Inputs {
        arm_switch_on: true,
        ..IDLE.inputs
    },
    ..IDLE
};

fn script(prefix: &[Tick], rest: &[Tick]) -> Vec<Tick> {
    [prefix, rest].concat()
}

#[test]
fn power_on_drives_nothing() {
    let outputs = State::POWER_ON.outputs();
    assert!(!outputs.rail_a && !outputs.rail_b);
    assert_eq!(outputs.panel, PanelState::AwaitingArmOff);
    assert_eq!(State::default(), State::POWER_ON);
}

#[test]
fn arm_sequence() {
    check(&[
        (
            "switch left on from power-on does not arm",
            &[ARM, ARM, ARM],
            ALL_OFF_AWAIT,
        ),
        ("switch off alone does not arm", &[IDLE], ALL_OFF_DISARMED),
        ("off then on arms rail A", &ARMED, RAIL_A_ONLY),
        (
            "left on, then off, then on arms",
            &[ARM, IDLE, ARM],
            RAIL_A_ONLY,
        ),
        (
            "switch off drops rail A",
            &[IDLE, ARM, IDLE],
            ALL_OFF_DISARMED,
        ),
        (
            "off and on again re-arms in the same session",
            &[IDLE, ARM, IDLE, ARM],
            RAIL_A_ONLY,
        ),
        (
            "a request cannot arm with the switch off",
            &[IDLE, IDLE.event(Event::RequestArm)],
            ALL_OFF_DISARMED.refused(Refusal::NotArmed),
        ),
        (
            "a request cannot stand in for seeing the switch off",
            &[ARM.event(Event::RequestArm), ARM.event(Event::RequestArm)],
            ALL_OFF_AWAIT.refused(Refusal::NotArmed),
        ),
        (
            "a request while armed is not refused",
            &[IDLE, ARM, ARM.event(Event::RequestArm)],
            RAIL_A_ONLY,
        ),
        (
            "the key and the button alone give nothing",
            &[IDLE.key().button(), ARM.key().button()],
            RAIL_A_ONLY,
        ),
    ]);
}

#[test]
fn restart_turns_everything_off() {
    check(&[
        (
            "restart while armed, switch left on",
            &[IDLE, ARM, ARM.event(Event::Restart)],
            ALL_OFF_AWAIT,
        ),
        (
            "still off on later ticks with the switch left on",
            &[IDLE, ARM, ARM.event(Event::Restart), ARM, ARM],
            ALL_OFF_AWAIT,
        ),
        (
            "a restart with the switch off still needs off then on",
            &[IDLE, ARM, IDLE.event(Event::Restart)],
            ALL_OFF_DISARMED,
        ),
        (
            "fresh sequence after restart re-arms",
            &[IDLE, ARM, ARM.event(Event::Restart), IDLE, ARM],
            RAIL_A_ONLY,
        ),
        (
            "restart does not clear a latched fault",
            &[IDLE.estop(), IDLE.event(Event::Restart)],
            fault(FaultReason::EmergencyStop),
        ),
    ]);
}

#[test]
fn emergency_stop_latches() {
    let stopped = fault(FaultReason::EmergencyStop);
    check(&[
        ("pressed while armed", &[IDLE, ARM, ARM.estop()], stopped),
        (
            "stays latched after release",
            &[IDLE, ARM, ARM.estop(), ARM, IDLE, ARM],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "pressed before any arming",
            &[ARM.estop()],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "outranks a restart on the same tick",
            &[IDLE, ARM, ARM.estop().event(Event::Restart)],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "outranks a lost heartbeat on the same tick",
            &[IDLE, ARM, ARM.estop().no_heartbeat()],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "a request on the tick it is pressed is refused",
            &[IDLE, ARM, ARM.estop().event(Event::RequestArm)],
            fault(FaultReason::EmergencyStop).refused(Refusal::Faulted),
        ),
    ]);
}

#[test]
fn lost_heartbeat_latches() {
    check(&[
        (
            "lost while armed",
            &[IDLE, ARM, ARM.no_heartbeat()],
            fault(FaultReason::HeartbeatLost),
        ),
        (
            "stays latched after it comes back",
            &[IDLE, ARM, ARM.no_heartbeat(), ARM, IDLE, ARM],
            fault(FaultReason::HeartbeatLost),
        ),
        (
            "the first reason is kept",
            &[IDLE, ARM, ARM.no_heartbeat(), ARM.estop()],
            fault(FaultReason::HeartbeatLost),
        ),
        (
            "missing from power-on",
            &[IDLE.no_heartbeat()],
            fault(FaultReason::HeartbeatLost),
        ),
    ]);
}

#[test]
fn clearing_a_fault() {
    let clear = Event::ClearFault;
    let stopped = fault(FaultReason::EmergencyStop);
    check(&[
        (
            "clears with the stop released and the switch off",
            &[IDLE, ARM, ARM.estop(), IDLE.event(clear)],
            ALL_OFF_DISARMED,
        ),
        (
            "refused while the stop is pressed",
            &[IDLE, ARM, ARM.estop(), IDLE.estop().event(clear)],
            fault(FaultReason::EmergencyStop).refused(Refusal::EmergencyStopPressed),
        ),
        (
            "refused while the arm switch is on",
            &[IDLE, ARM, ARM.estop(), ARM.event(clear)],
            fault(FaultReason::EmergencyStop).refused(Refusal::ArmSwitchOn),
        ),
        (
            "refused while the heartbeat is still missing",
            &[
                IDLE,
                ARM,
                ARM.no_heartbeat(),
                IDLE.no_heartbeat().event(clear),
            ],
            fault(FaultReason::HeartbeatLost).refused(Refusal::HeartbeatUnhealthy),
        ),
        (
            "no input clears it without the event",
            &[IDLE, ARM, ARM.estop(), IDLE, IDLE, IDLE],
            stopped,
        ),
        (
            "after a clear, the switch going on arms again",
            &[IDLE, ARM, ARM.estop(), IDLE.event(clear), ARM],
            RAIL_A_ONLY,
        ),
        (
            "other requests are refused while latched",
            &[IDLE, ARM, ARM.estop(), ARM.event(Event::RequestArm)],
            fault(FaultReason::EmergencyStop).refused(Refusal::Faulted),
        ),
        (
            "a clear with nothing latched is a no-op",
            &[IDLE, ARM, ARM.event(clear)],
            RAIL_A_ONLY,
        ),
    ]);
}

/// Armed with the key in: the starting point for every lease row.
const KEYED: Tick = Tick {
    inputs: Inputs {
        key_switch_on: true,
        ..ARM.inputs
    },
    ..ARM
};
const PENDING: Want = want(true, false, PanelState::ApprovalRequested);
const RAIL_B_ON: Want = want(true, true, PanelState::LeaseLive);

/// Armed, a class 3 request made, then granted by a press.
fn granted() -> Vec<Tick> {
    script(&ARMED, &[KEYED.event(LEASE3), KEYED.button()])
}

#[test]
fn lease_is_granted_only_by_a_fresh_button_press() {
    check(&[
        (
            "a request alone gives no rail B",
            &script(&ARMED, &[KEYED.event(LEASE3)]),
            PENDING,
        ),
        ("request then press grants", &granted(), RAIL_B_ON),
        (
            "a press with no request gives nothing",
            &script(&ARMED, &[KEYED.button(), KEYED, KEYED.button()]),
            RAIL_A_ONLY,
        ),
        (
            "a press before the request is not an approval of it",
            &script(&ARMED, &[KEYED.button(), KEYED.event(LEASE3)]),
            PENDING,
        ),
        (
            "a press on the tick the request arrives does not grant",
            &script(&ARMED, &[KEYED.button().event(LEASE3)]),
            PENDING,
        ),
        (
            "a button held down since before the request does not grant",
            &script(
                &ARMED,
                &[KEYED.button().event(LEASE3), KEYED.button(), KEYED.button()],
            ),
            PENDING,
        ),
        (
            "released and pressed again, it grants",
            &script(
                &ARMED,
                &[KEYED.button().event(LEASE3), KEYED, KEYED.button()],
            ),
            RAIL_B_ON,
        ),
        (
            "rail B stays on after the button is released",
            &script(&granted(), &[KEYED, KEYED]),
            RAIL_B_ON,
        ),
        (
            "a second request while one is pending is refused",
            &script(&ARMED, &[KEYED.event(LEASE3), KEYED.event(LEASE3)]),
            PENDING.refused(Refusal::LeaseBusy),
        ),
        (
            "a request while a lease is live is refused",
            &script(&granted(), &[KEYED.event(LEASE3)]),
            RAIL_B_ON.refused(Refusal::LeaseBusy),
        ),
        (
            "a request before arming is refused",
            &[IDLE.key().event(LEASE3)],
            ALL_OFF_DISARMED.refused(Refusal::NotArmed),
        ),
        (
            "a request on the tick of arming is refused and leaves nothing pending",
            &[IDLE, KEYED.event(LEASE3)],
            RAIL_A_ONLY.refused(Refusal::NotArmed),
        ),
    ]);
}

#[test]
fn lease_needs_the_key() {
    check(&[
        (
            "a request with the key off is refused",
            &script(&ARMED, &[ARM.event(LEASE3)]),
            RAIL_A_ONLY.refused(Refusal::KeyOff),
        ),
        (
            "the key going off withdraws a pending request",
            &script(&ARMED, &[KEYED.event(LEASE3), ARM, KEYED.button()]),
            RAIL_A_ONLY,
        ),
        (
            "a press with the key off grants nothing",
            &script(&ARMED, &[KEYED.event(LEASE3), ARM.button()]),
            RAIL_A_ONLY,
        ),
        (
            "the key going off drops rail B",
            &script(&granted(), &[ARM]),
            RAIL_A_ONLY,
        ),
        (
            "the key coming back does not bring rail B back",
            &script(&granted(), &[ARM, KEYED, KEYED.button()]),
            RAIL_A_ONLY,
        ),
    ]);
}

#[test]
fn lease_expires_after_a_fixed_number_of_ticks() {
    check(&[
        (
            "live on the last tick of the window",
            &script(&granted(), &[KEYED.after(LEASE_TTL_TICKS - 1)]),
            RAIL_B_ON,
        ),
        (
            "off on the tick the window ends",
            &script(&granted(), &[KEYED.after(LEASE_TTL_TICKS)]),
            RAIL_A_ONLY,
        ),
        (
            "holding the button does not extend it",
            &script(&granted(), &[KEYED.button().after(LEASE_TTL_TICKS)]),
            RAIL_A_ONLY,
        ),
        (
            "a press after expiry is not a new approval",
            &script(&granted(), &[KEYED.after(LEASE_TTL_TICKS), KEYED.button()]),
            RAIL_A_ONLY,
        ),
        (
            "a pending request can be granted on the last tick of its window",
            &script(
                &ARMED,
                &[
                    KEYED.event(LEASE3),
                    KEYED.button().after(REQUEST_TTL_TICKS - 1),
                ],
            ),
            RAIL_B_ON,
        ),
        (
            "a stale request is dropped, so a late press grants nothing",
            &script(
                &ARMED,
                &[KEYED.event(LEASE3), KEYED.button().after(REQUEST_TTL_TICKS)],
            ),
            RAIL_A_ONLY,
        ),
    ]);
}

#[test]
fn lease_is_spent_after_one_use() {
    let used = Event::LeaseUsed;
    check(&[
        (
            "use drops rail B",
            &script(&granted(), &[KEYED.event(used)]),
            RAIL_A_ONLY,
        ),
        (
            "a second use is refused",
            &script(&granted(), &[KEYED.event(used), KEYED.event(used)]),
            RAIL_A_ONLY.refused(Refusal::NoLiveLease),
        ),
        (
            "a replayed press after use grants nothing",
            &script(&granted(), &[KEYED.event(used), KEYED, KEYED.button()]),
            RAIL_A_ONLY,
        ),
        (
            "a use with nothing granted is refused",
            &script(&ARMED, &[KEYED.event(LEASE3), KEYED.event(used)]),
            PENDING.refused(Refusal::NoLiveLease),
        ),
        (
            "a new request and a new press grant a new lease",
            &script(
                &granted(),
                &[KEYED.event(used), KEYED.event(LEASE3), KEYED.button()],
            ),
            RAIL_B_ON,
        ),
    ]);
}

#[test]
fn lease_does_not_survive_disarm_restart_or_fault() {
    check(&[
        (
            "arm switch off drops both rails",
            &script(&granted(), &[IDLE.key()]),
            ALL_OFF_DISARMED,
        ),
        (
            "re-arming does not bring rail B back",
            &script(&granted(), &[IDLE.key(), KEYED, KEYED.button()]),
            RAIL_A_ONLY,
        ),
        (
            "restart drops both rails",
            &script(&granted(), &[KEYED.event(Event::Restart)]),
            ALL_OFF_AWAIT,
        ),
        (
            "emergency stop drops both rails",
            &script(&granted(), &[KEYED.estop()]),
            fault(FaultReason::EmergencyStop),
        ),
        (
            "lost heartbeat drops both rails",
            &script(&granted(), &[KEYED.no_heartbeat()]),
            fault(FaultReason::HeartbeatLost),
        ),
        (
            "after a fault is cleared and the rail re-armed, the lease is gone",
            &script(
                &granted(),
                &[
                    KEYED.estop(),
                    IDLE.key().event(Event::ClearFault),
                    KEYED,
                    KEYED.button(),
                ],
            ),
            RAIL_A_ONLY,
        ),
        (
            "a pending request does not survive a restart",
            &script(
                &ARMED,
                &[
                    KEYED.event(LEASE3),
                    KEYED.event(Event::Restart),
                    IDLE.key(),
                    KEYED,
                    KEYED.button(),
                ],
            ),
            RAIL_A_ONLY,
        ),
    ]);
}

#[test]
fn a_clock_that_goes_backwards_drops_the_lease() {
    let at = |now| Inputs {
        now,
        ..KEYED.inputs
    };
    let (state, _) = step(State::POWER_ON, at(10).with_arm(false), None);
    let (state, _) = step(state, at(11), None);
    let (state, _) = step(state, at(12), Some(LEASE3));
    let (state, outputs) = step(state, at(13).with_button(), None);
    assert!(outputs.rail_b);
    let (_, outputs) = step(state, at(12), None);
    assert!(outputs.rail_a && !outputs.rail_b);
}

#[test]
fn class_4_is_refused_in_every_state() {
    let class4 = Event::RequestLease {
        class: HazardClass::Class4,
    };
    let never = Refusal::ClassNeverPermitted;
    check(&[
        (
            "awaiting the arm switch",
            &[ARM.event(class4)],
            ALL_OFF_AWAIT.refused(never),
        ),
        (
            "disarmed",
            &[IDLE.event(class4)],
            ALL_OFF_DISARMED.refused(never),
        ),
        (
            "armed",
            &script(&ARMED, &[KEYED.event(class4)]),
            RAIL_A_ONLY.refused(never),
        ),
        (
            "armed, and a press afterwards grants nothing",
            &script(&ARMED, &[KEYED.event(class4), KEYED.button()]),
            RAIL_A_ONLY,
        ),
        (
            "with a class 3 request pending",
            &script(&ARMED, &[KEYED.event(LEASE3), KEYED.event(class4)]),
            PENDING.refused(never),
        ),
        (
            "with a class 3 lease live",
            &script(&granted(), &[KEYED.event(class4)]),
            RAIL_B_ON.refused(never),
        ),
        (
            "with a fault latched",
            &[IDLE.estop(), IDLE.event(class4)],
            fault(FaultReason::EmergencyStop).refused(never),
        ),
    ]);
}

#[test]
fn classes_without_a_lease_are_told_so() {
    for class in [
        HazardClass::Class0,
        HazardClass::Class1,
        HazardClass::Class2,
    ] {
        let request = Event::RequestLease { class };
        let last = run(&script(&ARMED, &[KEYED.event(request), KEYED.button()]));
        assert!(last.rail_a && !last.rail_b, "{class:?}");
        let last = run(&script(&ARMED, &[KEYED.event(request)]));
        assert_eq!(last.refusal, Some(Refusal::NoLeaseForClass), "{class:?}");
    }
}

/// Small setters for the one test that drives `step` directly.
trait InputsExt {
    fn with_arm(self, on: bool) -> Self;
    fn with_button(self) -> Self;
}

impl InputsExt for Inputs {
    fn with_arm(mut self, on: bool) -> Self {
        self.arm_switch_on = on;
        self
    }
    fn with_button(mut self) -> Self {
        self.panel_button_pressed = true;
        self
    }
}
