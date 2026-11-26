//! A host end for the tests: a [`HostLink`], a simulated clock, a deadline for
//! every call, and the checks the supervisor (#100) will have to make.
//!
//! This is not the supervisor. It does the minimum that lets a test say what
//! the protocol layer guarantees: a request that gets no answer times out, a
//! frame the link refuses changes nothing, an identity that does not verify
//! is never trusted. Each place it goes beyond [`HostLink`] is marked, and is
//! what #100 must provide.

use sawbones_module_protocol::{
    Capabilities, Challenge, HealthStatus, HostError, HostLink, IdentityClaim, IdentityError,
    ModuleId, ModuleKey, Request, Response, ResponseKind, MAX_FRAME_LEN,
};

use crate::link::SimLink;
use crate::rng::Rng;

/// What the host learned from a reply, as plain data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    /// The kind of response.
    pub kind: ResponseKind,
    /// The value, for a config or reading reply.
    pub value: Option<i64>,
    /// The module's own view of its health, for a health reply.
    pub health: Option<HealthStatus>,
}

/// How a call ended. Every call ends in one of these; none panics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The module answered and the link took the reply.
    Replied(Summary),
    /// The deadline passed. The request is abandoned; whether the module
    /// acted on it is unknown, so after a lost reply to a command that is not
    /// idempotent the host must read the state back.
    Timeout,
    /// More frames were refused during this call than the host tolerates. The
    /// request is abandoned. (#100: rate limit and quarantine.)
    Flooded,
    /// The host would not send: [`HostLink`] said no.
    SendFailed(HostError),
    /// The module's identity did not verify. The session is untrusted.
    IdentityRejected(IdentityError),
    /// The identity verified but claims more than the module was enrolled
    /// with. The session is untrusted. (#100: the registry holds the
    /// enrolment.)
    CapabilityMismatch {
        /// What the module was enrolled as.
        enrolled: Capabilities,
        /// What its signed identity says.
        signed: Capabilities,
    },
    /// No verified identity: the host sends nothing but an `Identify`.
    NotTrusted,
}

/// The host's end of one module's link.
#[derive(Debug)]
pub struct HostHarness {
    /// The protocol's host link.
    pub link: HostLink,
    /// The simulated clock, in ticks (milliseconds).
    pub now: u64,
    /// Every frame the host link refused, in order.
    pub refused: Vec<HostError>,
    /// How many calls timed out.
    pub timeouts: u32,
    /// How many refused frames one call tolerates before it gives up.
    pub flood_limit: usize,
    module: ModuleId,
    key: ModuleKey,
    enrolled: Capabilities,
    rng: Rng,
    trusted: bool,
    last_identity: Option<IdentityClaim>,
}

impl HostHarness {
    /// A host for module `module`, enrolled with `key` and `enrolled`
    /// capabilities. `seed` draws the challenges.
    #[must_use]
    pub fn new(module: ModuleId, key: ModuleKey, enrolled: Capabilities, seed: u64) -> Self {
        HostHarness {
            link: HostLink::new(),
            now: 0,
            refused: Vec::new(),
            timeouts: 0,
            flood_limit: 64,
            module,
            key,
            enrolled,
            rng: Rng::new(seed),
            trusted: false,
            last_identity: None,
        }
    }

    /// True once an identity verified and matched the enrolment, until the
    /// next `open`.
    #[must_use]
    pub const fn trusted(&self) -> bool {
        self.trusted
    }

    /// Opens a session: `Identify`, then the check against the enrolled key
    /// and capabilities. `timeout` is the deadline in ticks.
    pub fn open(&mut self, line: &mut SimLink, timeout: u64) -> Outcome {
        self.trusted = false;
        self.last_identity = None;
        let challenge = Challenge(self.rng.array());
        let mut out = [0u8; MAX_FRAME_LEN];
        let n = match self.link.open(challenge, &mut out) {
            Ok(n) => n,
            Err(e) => return Outcome::SendFailed(e),
        };
        line.send_from_host(self.now, out[..n].to_vec());
        let outcome = self.wait(line, timeout);
        let (Outcome::Replied(_), Some(claim), Some(session)) =
            (outcome, self.last_identity, self.link.session())
        else {
            return outcome;
        };
        match claim.verify(self.module, &self.key, &challenge, session) {
            Err(e) => Outcome::IdentityRejected(e),
            Ok(verified) if verified.capabilities() != self.enrolled => {
                Outcome::CapabilityMismatch {
                    enrolled: self.enrolled,
                    signed: verified.capabilities(),
                }
            }
            Ok(_) => {
                self.trusted = true;
                outcome
            }
        }
    }

    /// Sends `request` and waits `timeout` ticks for the reply.
    pub fn call(&mut self, line: &mut SimLink, request: &Request<'_>, timeout: u64) -> Outcome {
        if !self.trusted {
            return Outcome::NotTrusted;
        }
        let mut out = [0u8; MAX_FRAME_LEN];
        let n = match self.link.send(request, &mut out) {
            Ok(n) => n,
            Err(e) => return Outcome::SendFailed(e),
        };
        line.send_from_host(self.now, out[..n].to_vec());
        self.wait(line, timeout)
    }

    /// Lets `ticks` pass with nothing outstanding, taking whatever arrives.
    /// Every frame that arrives must be refused, since nothing was asked for.
    pub fn idle(&mut self, line: &mut SimLink, ticks: u64) {
        let end = self.now.saturating_add(ticks);
        while self.now < end {
            self.now += 1;
            line.pump(self.now);
            let mut taken = 0;
            while let Some(frame) = line.next_for_host(self.now) {
                if let Err(e) = self.link.receive(&frame).map(|_| ()) {
                    self.refused.push(e);
                }
                taken += 1;
                if taken > self.flood_limit {
                    // Beyond the budget the rest wait for the next tick.
                    break;
                }
            }
        }
    }

    fn wait(&mut self, line: &mut SimLink, timeout: u64) -> Outcome {
        let deadline = self.now.saturating_add(timeout);
        let mut refused_here = 0usize;
        loop {
            line.pump(self.now);
            while let Some(frame) = line.next_for_host(self.now) {
                match self.link.receive(&frame) {
                    Ok(from_module) => {
                        let response = from_module.into_claim();
                        if let Response::Identity(claim) = response {
                            self.last_identity = Some(claim);
                        }
                        return Outcome::Replied(summarise(&response));
                    }
                    Err(e) => {
                        self.refused.push(e);
                        refused_here += 1;
                        if refused_here > self.flood_limit {
                            self.link.abandon();
                            return Outcome::Flooded;
                        }
                    }
                }
            }
            if self.now >= deadline {
                self.link.abandon();
                self.timeouts += 1;
                return Outcome::Timeout;
            }
            self.now += 1;
        }
    }
}

fn summarise(response: &Response<'_>) -> Summary {
    Summary {
        kind: response.kind(),
        value: match response {
            Response::ConfigValue { value, .. } => Some(*value),
            _ => None,
        },
        health: match response {
            Response::Health(h) => Some(h.status),
            _ => None,
        },
    }
}
