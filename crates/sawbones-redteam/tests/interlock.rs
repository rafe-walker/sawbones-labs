//! Interlock attacks (lane `interlock-hardware`), at the pure state machine.

use sawbones_interlock::{
    step, ActionDigest, Event, FaultReason, FaultRecord, HazardClass, HeartbeatChallenge, Inputs,
    LeaseTarget, Outputs, Port, Rail, Refusal, State, OUTPUT_SETTLE_TICKS, RAIL_B_PORTS,
};

const TARGET: LeaseTarget = LeaseTarget {
    port: Port(0),
    action_digest: ActionDigest([1; 32]),
};
const OTHER_ACTION: LeaseTarget = LeaseTarget {
    port: Port(0),
    action_digest: ActionDigest([2; 32]),
};
const OTHER_PORT: LeaseTarget = LeaseTarget {
    port: Port(1),
    action_digest: ActionDigest([1; 32]),
};

/// A bench with healthy feedback (each rail is sensed as last driven) and a
/// healthy supervisor (each heartbeat challenge answered once, on the next
/// tick), booted cleanly at tick 0.
struct Bench {
    state: State,
    last: Outputs,
    now: u64,
    inputs: Inputs,
    answered: Option<HeartbeatChallenge>,
}

impl Bench {
    fn new() -> Self {
        let inputs = Inputs {
            estop_pressed: false,
            arm_switch_on: false,
            key_switch_on: false,
            heartbeat: None,
            panel_button_pressed: false,
            rail_a_sensed: false,
            rail_b_sensed: [false; RAIL_B_PORTS],
            firmware_jumper_fitted: false,
            now: 0,
        };
        let boot = Event::Restart {
            record: Some(FaultRecord::Clear),
            nonce: 0x0b00_7ed0,
        };
        let (state, last) = step(State::POWER_ON, inputs, Some(boot));
        Self {
            state,
            last,
            now: 0,
            inputs,
            answered: None,
        }
    }

    /// One tick. `sensed` overrides the healthy feedback.
    fn tick(
        &mut self,
        event: Option<Event>,
        sensed: Option<(bool, [bool; RAIL_B_PORTS])>,
    ) -> Outputs {
        self.now += 1;
        let (a, b) = sensed.unwrap_or_else(|| {
            let mut b = [false; RAIL_B_PORTS];
            if let Some(p) = self.last.rail_b {
                b[usize::from(p.0)] = true;
            }
            (self.last.rail_a, b)
        });
        let challenge = self.last.challenge;
        let heartbeat = if self.answered == Some(challenge) {
            None
        } else {
            self.answered = Some(challenge);
            Some(challenge.reply())
        };
        let inputs = Inputs {
            now: self.now,
            rail_a_sensed: a,
            rail_b_sensed: b,
            heartbeat,
            ..self.inputs
        };
        let (state, outputs) = step(self.state, inputs, event);
        self.state = state;
        self.last = outputs;
        outputs
    }

    fn armed_with_lease(&mut self, target: LeaseTarget) {
        self.tick(None, None);
        self.inputs.arm_switch_on = true;
        self.tick(None, None);
        self.inputs.key_switch_on = true;
        self.tick(
            Some(Event::RequestLease {
                class: HazardClass::Class3,
                target,
            }),
            None,
        );
        self.inputs.panel_button_pressed = false;
        self.tick(None, None);
        self.inputs.panel_button_pressed = true;
        let granted = self.tick(None, None);
        self.inputs.panel_button_pressed = false;
        assert_eq!(granted.rail_b, Some(target.port), "setup: lease granted");
    }
}

#[test]
fn redteam_hw_014_a_lease_is_spent_only_on_its_own_target_and_dies_with_its_conditions() {
    // Another action, another port: refused, and rail B goes off.
    for wrong in [OTHER_ACTION, OTHER_PORT] {
        let mut b = Bench::new();
        b.armed_with_lease(TARGET);
        let out = b.tick(Some(Event::LeaseUsed { target: wrong }), None);
        assert_eq!(out.refusal, Some(Refusal::LeaseMismatch));
        assert_eq!(out.rail_b, None);
        let again = b.tick(Some(Event::LeaseUsed { target: TARGET }), None);
        assert_eq!(again.refusal, Some(Refusal::NoLiveLease));
    }
    // The key comes out between the lease and the act.
    let mut b = Bench::new();
    b.armed_with_lease(TARGET);
    b.inputs.key_switch_on = false;
    let out = b.tick(Some(Event::LeaseUsed { target: TARGET }), None);
    assert_eq!(out.refusal, Some(Refusal::NoLiveLease));
    assert_eq!(out.rail_b, None);
    // The arm switch goes off between the lease and the act.
    let mut b = Bench::new();
    b.armed_with_lease(TARGET);
    b.inputs.arm_switch_on = false;
    let out = b.tick(Some(Event::LeaseUsed { target: TARGET }), None);
    assert!(out.refusal.is_some());
    assert_eq!((out.rail_a, out.rail_b), (false, None));
    // A fault on the very tick of the use: no success, everything off, latched.
    let mut b = Bench::new();
    b.armed_with_lease(TARGET);
    b.inputs.estop_pressed = true;
    let out = b.tick(Some(Event::LeaseUsed { target: TARGET }), None);
    assert_eq!(out.refusal, Some(Refusal::Faulted));
    assert_eq!((out.rail_a, out.rail_b), (false, None));
    assert_eq!(out.fault, Some(FaultReason::EmergencyStop));
}

#[test]
fn redteam_hw_025_feedback_is_checked_every_tick_in_both_directions() {
    // Feedback reading live while everything is off (welded contact, shorted
    // switch, or a sensor stuck live): fault on that tick.
    let mut b = Bench::new();
    let out = b.tick(None, Some((true, [false; RAIL_B_PORTS])));
    assert_eq!(
        out.fault,
        Some(FaultReason::OutputMismatch {
            rail: Rail::A,
            sensed_live: true
        })
    );
    // A sensor stuck dead while rail A is commanded on: fault once the settle
    // window has passed, however long the rail had been on before.
    let mut b = Bench::new();
    b.tick(None, None);
    b.inputs.arm_switch_on = true;
    assert!(b.tick(None, None).rail_a);
    for _ in 0..OUTPUT_SETTLE_TICKS - 1 {
        assert!(b.tick(None, Some((false, [false; RAIL_B_PORTS]))).rail_a);
    }
    let out = b.tick(None, Some((false, [false; RAIL_B_PORTS])));
    assert_eq!(
        out.fault,
        Some(FaultReason::OutputMismatch {
            rail: Rail::A,
            sensed_live: false
        })
    );
    assert!(!out.rail_a);
    // Healthy for a long time, then the sensor fails: still caught (not only
    // checked once at arm).
    let mut b = Bench::new();
    b.tick(None, None);
    b.inputs.arm_switch_on = true;
    for _ in 0..50 {
        assert!(b.tick(None, None).rail_a);
    }
    let out = b.tick(None, Some((false, [false; RAIL_B_PORTS])));
    assert!(out.fault.is_some() && !out.rail_a);
}
