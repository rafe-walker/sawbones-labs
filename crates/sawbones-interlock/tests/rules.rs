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

#[test]
fn a_class_3_request_shows_on_the_panel_without_energising_rail_b() {
    let last = run(&script(&ARMED, &[ARM.key().event(LEASE3)]));
    assert_eq!(
        last,
        want_outputs(true, false, PanelState::ApprovalRequested)
    );
    let late = REQUEST_TTL_TICKS.max(LEASE_TTL_TICKS);
    let last = run(&script(
        &ARMED,
        &[ARM.key().event(LEASE3), ARM.key().button().after(late)],
    ));
    assert_eq!(last, want_outputs(true, false, PanelState::Armed));
}

fn want_outputs(rail_a: bool, rail_b: bool, panel: PanelState) -> Outputs {
    Outputs {
        rail_a,
        rail_b,
        panel,
        fault: None,
        refusal: None,
    }
}
