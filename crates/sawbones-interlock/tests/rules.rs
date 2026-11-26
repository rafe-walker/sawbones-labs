//! Table tests: one table per rule. Each row is a script of ticks and what the
//! outputs must be after the last one.

use sawbones_interlock::{
    step, ActionDigest, Event, FaultReason, HazardClass, Inputs, LeaseTarget, Outputs, PanelState,
    Port, Rail, Refusal, State, LEASE_TTL_TICKS, OUTPUT_SETTLE_TICKS, RAIL_B_PORTS,
    REQUEST_TTL_TICKS,
};

/// One tick of a script.
#[derive(Clone, Copy)]
struct Tick {
    inputs: Inputs,
    event: Option<Event>,
    /// How far the clock moves before this tick.
    advance: u64,
    /// The rail feedback for this tick. `None` is healthy hardware: each rail
    /// is sensed exactly as the previous tick drove it, and off after a
    /// restart.
    sensed: Option<(bool, [bool; RAIL_B_PORTS])>,
}

const NO_RAIL_B: [bool; RAIL_B_PORTS] = [false; RAIL_B_PORTS];

/// Rail B sensed live on one port only.
const fn b_live(port: u8) -> [bool; RAIL_B_PORTS] {
    let mut b = NO_RAIL_B;
    b[port as usize] = true;
    b
}

/// A healthy, idle sample: nothing pressed, every switch off.
const IDLE: Tick = Tick {
    inputs: Inputs {
        estop_pressed: false,
        arm_switch_on: false,
        key_switch_on: false,
        heartbeat_healthy: true,
        panel_button_pressed: false,
        rail_a_sensed: false,
        rail_b_sensed: NO_RAIL_B,
        firmware_jumper_fitted: false,
        now: 0,
    },
    event: None,
    advance: 1,
    sensed: None,
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
    fn jumper(mut self) -> Self {
        self.inputs.firmware_jumper_fitted = true;
        self
    }
    /// Overrides the rail feedback on this tick.
    fn sensed(mut self, rail_a: bool, rail_b: [bool; RAIL_B_PORTS]) -> Self {
        self.sensed = Some((rail_a, rail_b));
        self
    }
}

/// What the last tick of a script must produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Want {
    rail_a: bool,
    rail_b: Option<Port>,
    firmware_update_allowed: bool,
    panel: PanelState,
    fault: Option<FaultReason>,
    refusal: Option<Refusal>,
}

/// `rail_b` true means rail B on for the default target's port.
const fn want(rail_a: bool, rail_b: bool, panel: PanelState) -> Want {
    Want {
        rail_a,
        rail_b: if rail_b { Some(PORT0) } else { None },
        firmware_update_allowed: false,
        panel,
        fault: None,
        refusal: None,
    }
}

const fn fault(reason: FaultReason) -> Want {
    Want {
        rail_a: false,
        rail_b: None,
        firmware_update_allowed: false,
        panel: PanelState::Fault,
        fault: Some(reason),
        refusal: None,
    }
}

const FIRMWARE: Want = Want {
    rail_a: false,
    rail_b: None,
    firmware_update_allowed: true,
    panel: PanelState::FirmwareUpdate,
    fault: None,
    refusal: None,
};

const fn mismatch(rail: Rail, sensed_live: bool) -> Want {
    fault(FaultReason::OutputMismatch { rail, sensed_live })
}

impl Want {
    const fn refused(mut self, why: Refusal) -> Self {
        self.refusal = Some(why);
        self
    }
    const fn on_port(mut self, port: Port) -> Self {
        self.rail_b = Some(port);
        self
    }
}

const ALL_OFF_AWAIT: Want = want(false, false, PanelState::AwaitingArmOff);
const ALL_OFF_DISARMED: Want = want(false, false, PanelState::Disarmed);
const RAIL_A_ONLY: Want = want(true, false, PanelState::Armed);

const PORT0: Port = Port(0);
const DIGEST1: ActionDigest = ActionDigest([1; 32]);
const DIGEST2: ActionDigest = ActionDigest([2; 32]);
/// The default target: one action on port 0.
const T0: LeaseTarget = LeaseTarget {
    port: PORT0,
    action_digest: DIGEST1,
};

const fn lease3(target: LeaseTarget) -> Event {
    Event::RequestLease {
        class: HazardClass::Class3,
        target,
    }
}
const fn used(target: LeaseTarget) -> Event {
    Event::LeaseUsed { target }
}
const LEASE3: Event = lease3(T0);

/// Runs a script from power-on and returns the last tick's outputs.
fn run(script: &[Tick]) -> Outputs {
    let mut state = State::POWER_ON;
    let mut now = 0;
    let mut last = state.outputs();
    for tick in script {
        now += tick.advance;
        let (rail_a_sensed, rail_b_sensed) = match tick.sensed {
            Some(sensed) => sensed,
            None if tick.event == Some(Event::Restart) => (false, NO_RAIL_B),
            None => {
                let mut b = NO_RAIL_B;
                if let Some(port) = last.rail_b {
                    b[usize::from(port.0)] = true;
                }
                (last.rail_a, b)
            }
        };
        let inputs = Inputs {
            now,
            rail_a_sensed,
            rail_b_sensed,
            ..tick.inputs
        };
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
            firmware_update_allowed: got.firmware_update_allowed,
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
    assert!(!outputs.rail_a && outputs.rail_b.is_none());
    assert!(!outputs.firmware_update_allowed);
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
    let used = used(T0);
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
    let (state, _) = step(state, at(12).with_rail_a(), Some(LEASE3));
    let (state, outputs) = step(state, at(13).with_rail_a().with_button(), None);
    assert_eq!(outputs.rail_b, Some(PORT0));
    let healthy = Inputs {
        rail_b_sensed: b_live(0),
        ..at(12).with_rail_a()
    };
    let (_, outputs) = step(state, healthy, None);
    assert!(outputs.rail_a && outputs.rail_b.is_none());
    assert_eq!(outputs.fault, None);
}

#[test]
fn class_4_is_refused_in_every_state() {
    let class4 = Event::RequestLease {
        class: HazardClass::Class4,
        target: T0,
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
        let request = Event::RequestLease { class, target: T0 };
        let last = run(&script(&ARMED, &[KEYED.event(request), KEYED.button()]));
        assert!(last.rail_a && last.rail_b.is_none(), "{class:?}");
        let last = run(&script(&ARMED, &[KEYED.event(request)]));
        assert_eq!(last.refusal, Some(Refusal::NoLeaseForClass), "{class:?}");
    }
}

/// Small setters for the one test that drives `step` directly.
trait InputsExt {
    fn with_arm(self, on: bool) -> Self;
    fn with_button(self) -> Self;
    fn with_rail_a(self) -> Self;
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
    fn with_rail_a(mut self) -> Self {
        self.rail_a_sensed = true;
        self
    }
}

/// Armed, a class 3 request for `target` made, then granted by a press.
fn granted_for(target: LeaseTarget) -> Vec<Tick> {
    script(&ARMED, &[KEYED.event(lease3(target)), KEYED.button()])
}

const T1: LeaseTarget = LeaseTarget {
    port: Port(1),
    action_digest: DIGEST1,
};
const T0_OTHER_ACTION: LeaseTarget = LeaseTarget {
    port: PORT0,
    action_digest: DIGEST2,
};
#[allow(clippy::cast_possible_truncation)]
const NO_SUCH_PORT: Port = Port(RAIL_B_PORTS as u8);

#[test]
fn lease_is_bound_to_one_port_and_one_action() {
    let on_1 = RAIL_B_ON.on_port(Port(1));
    check(&[
        (
            "a lease enables rail B on its own port only",
            &granted_for(T1),
            on_1,
        ),
        (
            "a use naming the same port and action spends it",
            &script(&granted_for(T1), &[KEYED.event(used(T1))]),
            RAIL_A_ONLY,
        ),
        (
            "a use naming another action is refused and ends the lease",
            &script(&granted(), &[KEYED.event(used(T0_OTHER_ACTION))]),
            RAIL_A_ONLY.refused(Refusal::LeaseMismatch),
        ),
        (
            "a use naming another port is refused and ends the lease",
            &script(&granted(), &[KEYED.event(used(T1))]),
            RAIL_A_ONLY.refused(Refusal::LeaseMismatch),
        ),
        (
            "after a mismatched use the right use finds nothing",
            &script(&granted(), &[KEYED.event(used(T1)), KEYED.event(used(T0))]),
            RAIL_A_ONLY.refused(Refusal::NoLiveLease),
        ),
        (
            "a request for another port while a lease is live is refused",
            &script(&granted(), &[KEYED.event(lease3(T1)), KEYED.button()]),
            RAIL_B_ON,
        ),
        (
            "and the refusal says busy",
            &script(&granted(), &[KEYED.event(lease3(T1))]),
            RAIL_B_ON.refused(Refusal::LeaseBusy),
        ),
        (
            "a press grants the target that was requested, not another",
            &script(
                &ARMED,
                &[
                    KEYED.event(lease3(T1)),
                    KEYED.event(lease3(T0)),
                    KEYED.button(),
                ],
            ),
            on_1,
        ),
        (
            "a request for a port with no rail B is refused",
            &script(
                &ARMED,
                &[KEYED.event(lease3(LeaseTarget {
                    port: NO_SUCH_PORT,
                    action_digest: DIGEST1,
                }))],
            ),
            RAIL_A_ONLY.refused(Refusal::UnknownPort),
        ),
        (
            "and leaves nothing for a press to grant",
            &script(
                &ARMED,
                &[
                    KEYED.event(lease3(LeaseTarget {
                        port: NO_SUCH_PORT,
                        action_digest: DIGEST1,
                    })),
                    KEYED.button(),
                ],
            ),
            RAIL_A_ONLY,
        ),
        (
            "an unknown port is refused before arming too",
            &[IDLE.key().event(lease3(LeaseTarget {
                port: Port(u8::MAX),
                action_digest: DIGEST1,
            }))],
            ALL_OFF_DISARMED.refused(Refusal::UnknownPort),
        ),
        (
            "a new lease after a spent one may be for another port",
            &script(
                &granted(),
                &[
                    KEYED.event(used(T0)),
                    KEYED.event(lease3(T1)),
                    KEYED.button(),
                ],
            ),
            on_1,
        ),
    ]);
}

#[test]
fn a_stuck_on_rail_drops_everything_and_latches() {
    let stuck_a = mismatch(Rail::A, true);
    check(&[
        (
            "rail A live while disarmed",
            &[IDLE, IDLE.sensed(true, NO_RAIL_B)],
            stuck_a,
        ),
        (
            "rail A live at power-on",
            &[IDLE.sensed(true, NO_RAIL_B)],
            stuck_a,
        ),
        (
            "rail A live on the tick of a restart, whatever was driven",
            &[IDLE, ARM, ARM.event(Event::Restart).sensed(true, NO_RAIL_B)],
            stuck_a,
        ),
        (
            "rail B live on a port with no lease, while armed",
            &script(&ARMED, &[KEYED.sensed(true, b_live(2))]),
            mismatch(Rail::B(Port(2)), true),
        ),
        (
            "rail B live on another port while a lease is live drops the lease",
            &script(
                &granted(),
                &[KEYED.sensed(true, [true, false, false, true])],
            ),
            mismatch(Rail::B(Port(3)), true),
        ),
        (
            "a rail has the tick it is commanded off to drop",
            &script(&granted(), &[KEYED.event(used(T0))]),
            RAIL_A_ONLY,
        ),
        (
            "but live on the tick after is stuck on",
            &script(
                &granted(),
                &[KEYED.event(used(T0)), KEYED.sensed(true, b_live(0))],
            ),
            mismatch(Rail::B(PORT0), true),
        ),
        (
            "rail A still live the tick after disarming is stuck on",
            &[IDLE, ARM, IDLE, IDLE.sensed(true, NO_RAIL_B)],
            stuck_a,
        ),
        (
            "the emergency stop outranks it",
            &[IDLE.estop().sensed(true, NO_RAIL_B)],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "rail A is judged before rail B",
            &[IDLE.sensed(true, b_live(1))],
            stuck_a,
        ),
        (
            "a clear is refused while a rail is still live",
            &[
                IDLE.sensed(true, NO_RAIL_B),
                IDLE.event(Event::ClearFault).sensed(true, NO_RAIL_B),
            ],
            stuck_a.refused(Refusal::RailSensedLive),
        ),
        (
            "a clear is refused while rail B is live on any port",
            &[
                IDLE.sensed(false, b_live(3)),
                IDLE.event(Event::ClearFault).sensed(false, b_live(3)),
            ],
            mismatch(Rail::B(Port(3)), true).refused(Refusal::RailSensedLive),
        ),
        (
            "a clear succeeds once every rail is dead",
            &[IDLE.sensed(true, NO_RAIL_B), IDLE.event(Event::ClearFault)],
            ALL_OFF_DISARMED,
        ),
    ]);
}

#[test]
fn a_rail_dead_while_commanded_on_latches_after_the_settle_window() {
    let settling = OUTPUT_SETTLE_TICKS - 1;
    check(&[
        (
            "rail A dead inside its settle window is tolerated",
            &[IDLE, ARM, ARM.sensed(false, NO_RAIL_B).after(settling)],
            RAIL_A_ONLY,
        ),
        (
            "rail A dead at the end of the window latches",
            &[
                IDLE,
                ARM,
                ARM.sensed(false, NO_RAIL_B).after(OUTPUT_SETTLE_TICKS),
            ],
            mismatch(Rail::A, false),
        ),
        (
            "rail A dropping out mid-session latches",
            &[IDLE, ARM, ARM.after(50), ARM.sensed(false, NO_RAIL_B)],
            mismatch(Rail::A, false),
        ),
        (
            "rail B dead inside its window is tolerated",
            &script(&granted(), &[KEYED.sensed(true, NO_RAIL_B).after(settling)]),
            RAIL_B_ON,
        ),
        (
            "rail B dead at the end of its window latches and drops rail A",
            &script(
                &granted(),
                &[KEYED.sensed(true, NO_RAIL_B).after(OUTPUT_SETTLE_TICKS)],
            ),
            mismatch(Rail::B(PORT0), false),
        ),
        (
            "rail B live on the wrong port instead of the leased one",
            &script(
                &granted(),
                &[KEYED.sensed(true, b_live(1)).after(OUTPUT_SETTLE_TICKS)],
            ),
            mismatch(Rail::B(PORT0), false),
        ),
        (
            "a dead rail cannot be cleared into service: it is re-armed afresh",
            &[
                IDLE,
                ARM,
                ARM.sensed(false, NO_RAIL_B).after(OUTPUT_SETTLE_TICKS),
                IDLE.event(Event::ClearFault),
                ARM,
            ],
            RAIL_A_ONLY,
        ),
    ]);
}

#[test]
fn firmware_jumper() {
    let armed_jumper = fault(FaultReason::FirmwareJumperWhileArmed);
    check(&[
        (
            "fitted while disarmed allows an update",
            &[IDLE, IDLE.jumper()],
            FIRMWARE,
        ),
        (
            "fitted at power-on with the arm switch on allows an update",
            &[ARM.jumper()],
            FIRMWARE,
        ),
        (
            "fitted while armed latches a fault",
            &[IDLE, ARM, ARM.jumper()],
            armed_jumper,
        ),
        (
            "fitted while a lease is live latches a fault",
            &script(&granted(), &[KEYED.jumper()]),
            armed_jumper,
        ),
        (
            "fitted on the tick the arm switch goes off still counts as armed",
            &[IDLE, ARM, IDLE.jumper()],
            armed_jumper,
        ),
        (
            "fitted on the tick the arm switch goes on does not arm",
            &[IDLE, ARM.jumper()],
            FIRMWARE,
        ),
        (
            "the arm switch cannot arm during an update",
            &[IDLE, IDLE.jumper(), ARM.jumper(), ARM.jumper()],
            FIRMWARE,
        ),
        (
            "a request to arm is refused during an update",
            &[IDLE, IDLE.jumper(), ARM.jumper().event(Event::RequestArm)],
            FIRMWARE.refused(Refusal::FirmwareJumperFitted),
        ),
        (
            "a lease request is refused during an update",
            &[IDLE, IDLE.jumper().key().event(LEASE3)],
            FIRMWARE.refused(Refusal::FirmwareJumperFitted),
        ),
        (
            "removing it with the arm switch on does not arm",
            &[IDLE, IDLE.jumper(), ARM.jumper(), ARM],
            ALL_OFF_AWAIT,
        ),
        (
            "removing it needs a fresh arm sequence",
            &[IDLE, IDLE.jumper(), ARM.jumper(), ARM, IDLE, ARM],
            RAIL_A_ONLY,
        ),
        (
            "removing it with the arm switch off disarms",
            &[IDLE, IDLE.jumper(), IDLE],
            ALL_OFF_DISARMED,
        ),
        (
            "no update with a fault latched",
            &[IDLE.estop(), IDLE.jumper()],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "the emergency stop during an update latches",
            &[IDLE, IDLE.jumper(), IDLE.jumper().estop()],
            fault(FaultReason::EmergencyStop),
        ),
        (
            "a stuck-on rail during an update latches",
            &[IDLE, IDLE.jumper(), IDLE.jumper().sensed(true, NO_RAIL_B)],
            mismatch(Rail::A, true),
        ),
        (
            "after a jumper fault is cleared with the jumper still fitted, an update",
            &[
                IDLE,
                ARM,
                ARM.jumper(),
                IDLE.jumper().event(Event::ClearFault),
                IDLE.jumper(),
            ],
            FIRMWARE,
        ),
        (
            "a restart during an update keeps it once the jumper is seen",
            &[IDLE, IDLE.jumper(), ARM.jumper().event(Event::Restart)],
            FIRMWARE,
        ),
    ]);
}

#[test]
fn rules_id_is_pinned() {
    use sawbones_interlock::{rules_id, RULES_VERSION};
    // If this fails, a placeholder constant or RULES_VERSION changed. Update
    // it on purpose: the supervisor's expected id changes with it. The value
    // was checked against an independent FNV-1a in Python over the words
    // [2, 100, 300, 5, 4] as little-endian u64s.
    assert_eq!(RULES_VERSION, 2);
    assert_eq!(rules_id(), 0x24ac_74ab_b848_d6b5);
}
