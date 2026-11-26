//! What a command with a hazard must carry for the interlock.

use sha2::{Digest, Sha256};

use crate::message::{Channel, ConfigKey};
use crate::wire::{Reader, Writer};
use crate::{ActionDigest, DecodeError, EncodeError, HazardClass, LeaseTarget, ModuleId, Port};

/// The bytes hashed before every [`action_digest`], so the digest of a module
/// action can never equal the digest of anything else (such as a bus write
/// action, whose domain differs).
pub const ACTION_DOMAIN: [u8; 35] = *b"sawbones-module-protocol/action/v1\0";

const TAG_NONE: u8 = 0;
const TAG_RAIL_A: u8 = 2;
const TAG_LEASE: u8 = 3;
const TAG_CLASS_4: u8 = 4;

/// What a config write or an actuation carries for the interlock: the
/// highest hazard class it may be, and for class 3 the interlock's lease
/// target.
///
/// There is no class 4 authority: class 4 has no rail and is never offered,
/// and its tag is refused on decode with [`DecodeError::ClassNeverPermitted`].
/// The module checks the authority against its own
/// [`ClassTable`](crate::ClassTable) in
/// [`ModuleLink::accept`](crate::ModuleLink::accept);
/// the host's choice of authority only ever lowers what it may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Authority {
    /// Hazard class 0 or 1: no rail and no permission.
    None,
    /// Hazard class 2: armed rail A. Only the arm switch arms it, so there is
    /// nothing to carry; the module checks rail A is sensed live.
    RailA,
    /// Hazard class 3: a live interlock lease for exactly this target. The
    /// target's digest must be [`action_digest`] of this action on this
    /// module.
    Lease(LeaseTarget),
}

impl Authority {
    /// The highest hazard class this authority covers.
    #[must_use]
    pub const fn class(self) -> HazardClass {
        match self {
            Authority::None => HazardClass::Class1,
            Authority::RailA => HazardClass::Class2,
            Authority::Lease(_) => HazardClass::Class3,
        }
    }

    /// True if this authority covers an action of class `required`. Class 4
    /// is never covered.
    #[must_use]
    pub const fn permits(self, required: HazardClass) -> bool {
        rank(required) <= rank(self.class()) && !matches!(required, HazardClass::Class4)
    }

    /// The lease target, for class 3.
    #[must_use]
    pub const fn lease_target(self) -> Option<LeaseTarget> {
        match self {
            Authority::Lease(target) => Some(target),
            Authority::None | Authority::RailA => None,
        }
    }

    pub(crate) fn encode(self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        match self {
            Authority::None => w.u8(TAG_NONE),
            Authority::RailA => w.u8(TAG_RAIL_A),
            Authority::Lease(target) => {
                w.u8(TAG_LEASE)?;
                w.u8(target.port.0)?;
                w.put(&target.action_digest.0)
            }
        }
    }

    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Authority, DecodeError> {
        match r.u8()? {
            TAG_NONE => Ok(Authority::None),
            TAG_RAIL_A => Ok(Authority::RailA),
            TAG_LEASE => {
                let port = Port(r.u8()?);
                if !port.exists() {
                    return Err(DecodeError::OutOfBounds {
                        field: "lease port",
                    });
                }
                Ok(Authority::Lease(LeaseTarget {
                    port,
                    action_digest: ActionDigest(r.array()?),
                }))
            }
            TAG_CLASS_4 => Err(DecodeError::ClassNeverPermitted),
            value => Err(DecodeError::BadTag {
                field: "authority",
                value,
            }),
        }
    }
}

/// The order of the hazard classes, as a number.
pub(crate) const fn rank(class: HazardClass) -> u8 {
    match class {
        HazardClass::Class0 => 0,
        HazardClass::Class1 => 1,
        HazardClass::Class2 => 2,
        HazardClass::Class3 => 3,
        HazardClass::Class4 => 4,
    }
}

/// What the host asks for when it builds a command: the authority, without
/// the digest, which [`Grant::authority`] computes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Grant {
    /// Class 0 or 1.
    None,
    /// Class 2, under armed rail A.
    RailA,
    /// Class 3, under a lease for rail B on this port.
    Lease {
        /// The rail B port the lease is for.
        port: Port,
    },
}

impl Grant {
    /// The authority for `action` on `module`. For a lease, the target's
    /// digest is [`action_digest`] of exactly this action.
    #[must_use]
    pub fn authority(self, module: ModuleId, action: ActionBody) -> Authority {
        match self {
            Grant::None => Authority::None,
            Grant::RailA => Authority::RailA,
            Grant::Lease { port } => Authority::Lease(LeaseTarget {
                port,
                action_digest: action_digest(module, action),
            }),
        }
    }
}

/// The part of a command that a lease approves: what is to be done, without
/// the authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionBody {
    /// Write a config value.
    ConfigWrite {
        /// The key.
        key: ConfigKey,
        /// The value.
        value: i64,
    },
    /// Drive an output channel.
    Actuate {
        /// The channel.
        channel: Channel,
        /// The setting, in the channel's units from the manifest.
        setting: i32,
    },
}

const BODY_CONFIG_WRITE: u8 = 1;
const BODY_ACTUATE: u8 = 2;

/// The digest a class 3 lease for `action` on `module` carries: SHA-256 of
/// [`ACTION_DOMAIN`], the module id (4 bytes), then the action's kind (1 for
/// a config write, 2 for an actuation) and its fields, little-endian. So a
/// lease for one action on one module pays for no other action and no other
/// module, even on the same rail B port.
#[must_use]
pub fn action_digest(module: ModuleId, action: ActionBody) -> ActionDigest {
    let mut hasher = Sha256::new();
    hasher.update(ACTION_DOMAIN);
    hasher.update(module.0.to_le_bytes());
    match action {
        ActionBody::ConfigWrite { key, value } => {
            hasher.update([BODY_CONFIG_WRITE]);
            hasher.update(key.0.to_le_bytes());
            hasher.update(value.to_le_bytes());
        }
        ActionBody::Actuate { channel, setting } => {
            hasher.update([BODY_ACTUATE]);
            hasher.update([channel.0]);
            hasher.update(setting.to_le_bytes());
        }
    }
    ActionDigest(hasher.finalize().into())
}
