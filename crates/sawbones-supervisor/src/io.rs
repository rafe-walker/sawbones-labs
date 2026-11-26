//! The links the supervisor talks over, as traits, so tests can use
//! in-memory fakes and the Pi can use USB and serial later.

use sawbones_interlock::{HeartbeatChallenge, HeartbeatReply, LeaseTarget, Port};
use sawbones_module_protocol::Challenge;
use sawbones_policy::WriteValue;

use crate::registry::HubPort;
use crate::write::Authorized;

/// The USB hub the modules hang off, one framed byte stream per hub port.
pub trait ModuleTransport {
    /// The hub ports with a device on them now. A port that drops out of
    /// this list has been unplugged.
    fn present(&self) -> Vec<HubPort>;
    /// Sends one frame to the device on `port`. Delivery is not promised:
    /// a lost frame shows up as a missed deadline.
    fn send(&mut self, port: HubPort, frame: &[u8]);
    /// The next frame that arrived from the device on `port`, if any.
    fn receive(&mut self, port: HubPort) -> Option<Vec<u8>>;
}

/// The link to the trust root (the interlock controller).
///
/// Everything it reports is the trust root's word as the link carried it.
/// The supervisor uses the report only to refuse sooner: the trust root and
/// the modules check rails themselves, whatever the supervisor believes.
pub trait InterlockLink {
    /// The heartbeat challenge the trust root sent last, if any.
    fn challenge(&self) -> Option<HeartbeatChallenge>;
    /// Sends a heartbeat reply. Only [`Supervisor::step`] calls this.
    ///
    /// [`Supervisor::step`]: crate::Supervisor::step
    fn reply(&mut self, reply: HeartbeatReply);
    /// Sends a request. None of them can turn a rail on.
    fn send(&mut self, request: ToInterlock);
    /// The trust root's last status report, if one arrived.
    fn report(&self) -> Option<InterlockReport>;
}

/// What the supervisor may ask of the trust root. There is no clear-fault
/// request: only the panel clears a fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToInterlock {
    /// Asks for rail A. Only the arm switch arms it.
    RequestArm,
    /// Asks for a class 3 lease for exactly this target. Only a panel press
    /// grants it.
    RequestLease {
        /// The port and action digest.
        target: LeaseTarget,
    },
    /// The leased action was carried out: the lease is spent.
    LeaseUsed {
        /// The port and action digest.
        target: LeaseTarget,
    },
}

/// The trust root's status, as it reported it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InterlockReport {
    /// The rule set the trust root enforces. The supervisor refuses to plan
    /// against rules it was not built for.
    pub rules_id: u64,
    /// Rail A is on.
    pub rail_a: bool,
    /// The one port rail B is on for, if any.
    pub rail_b: Option<Port>,
    /// A fault is latched.
    pub faulted: bool,
}

/// The bus a comms pod carries. A transport acts only on an [`Authorized`]
/// action, which only [`Supervisor::invoke`] makes, after
/// [`confirm`](sawbones_policy::confirm) has consumed a preflight token.
///
/// [`Supervisor::invoke`]: crate::Supervisor::invoke
pub trait BusTransport {
    /// Carries out the action and reports what came back.
    fn perform(&mut self, action: &Authorized) -> BusReply;
}

/// What came back from the bus. A value is the device's claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BusReply {
    /// The device acknowledged a write, or a listen started.
    Done,
    /// The value a read returned.
    Value(WriteValue),
    /// Nothing came back in time. For a write, its effect is unknown.
    NoReply,
}

/// Fresh challenges for module identities, from the host's random number
/// generator. A counter is not good enough outside tests: a challenge must
/// not be predictable.
pub trait ChallengeSource {
    /// The next challenge.
    fn challenge(&mut self) -> Challenge;
}
