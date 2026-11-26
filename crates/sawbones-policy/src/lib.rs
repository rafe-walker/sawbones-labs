//! # sawbones-policy: PROVISIONAL
//!
//! **This crate is provisional. The Sawbones design is still under review, and
//! every type here may change or be replaced without notice.**
//!
//! The hazard-class policy engine. It decides what permission an action
//! needs, as one pure function, [`classify`], over three things:
//!
//! - an [`EffectRequest`]: what a plugin asks for, as typed data. Plugins
//!   never expose tools of their own; they emit requests such as a
//!   [`BusWrite`] of a typed [`WriteValue`] to a named [`SignalId`] on a
//!   [`PortId`]. A write to a raw address cannot be written down.
//! - the [`WriteMap`]: one [`WriteEntry`] fact per writable signal, with its
//!   class, its [`SettingKind`], whether it is energy-relevant and the
//!   [`Trust`](sawbones_reading::Trust) tier of the entry itself.
//! - the [`BusFacts`]: one mastership fact per port, saying whether the bench
//!   is the only master on that bus.
//!
//! The answer is a [`Decision`]: a [`HazardClass`] and the [`Requirement`]
//! that class carries: nothing, armed rail A, a rail B lease bound to an
//! [`ActionDigest`] of the exact request, or a refusal with a typed
//! [`RefusalReason`]. The rules are listed on [`classify`].
//!
//! ```
//! use sawbones_interlock::HazardClass;
//! use sawbones_policy::*;
//! use sawbones_reading::Trust;
//!
//! let port = PortId::parse("rs485-1").unwrap();
//! let signal = SignalId::parse("battery_low_cutoff").unwrap();
//! let writes = WriteMap::new([WriteEntry {
//!     port: port.clone(),
//!     signal: signal.clone(),
//!     class: HazardClass::Class2,
//!     kind: SettingKind::BatteryProtectionLimit,
//!     energy_relevant: true,
//!     trust: Trust::OperatorAttested,
//!     range: Some(ValueRange::new(WriteValue::U16(4400), WriteValue::U16(5000)).unwrap()),
//! }])
//! .unwrap();
//! let buses = BusFacts::new([BusFact {
//!     port: port.clone(),
//!     mastership: Mastership::BenchOnly,
//!     trust: Trust::OperatorAttested,
//! }])
//! .unwrap();
//!
//! let request = EffectRequest::BusWrite(BusWrite { port, signal, value: WriteValue::U16(4600) });
//! let decision = classify(&request, &writes, &buses);
//! // A battery protection limit is class 3, whatever its entry says.
//! assert_eq!(decision.class(), HazardClass::Class3);
//! assert!(matches!(decision.requirement(), Requirement::RailBLease { .. }));
//! ```
//!
//! **Value limits.** A write-map entry may carry a [`ValueRange`]. A write
//! outside it is refused with [`RefusalReason::ValueOutOfRange`]; the range
//! never changes the class. The range belongs to the entry, so it has the
//! entry's trust tier.
//!
//! **No world text in decisions.** Every argument is a typed value. Ports and
//! signals ([`PortId`], [`SignalId`]) are constructible only from a string of
//! 1 to 64 bytes that starts with a lower-case ASCII letter and continues with
//! `a-z 0-9 _ -` only; anything else is a [`PolicyIdentError`]. Values are
//! integers or booleans, and no type here holds or converts from
//! [`WorldText`](sawbones_reading::WorldText). Text copied out of a
//! `WorldText` through `as_untrusted_str` must pass the same rule, so a
//! sentence, markup or a path cannot become an identifier. The rule limits
//! shape, not meaning; the class depends on a name only through an exact
//! lookup in the write map, so an unlisted name is refused as unknown.
//!
//! What is deliberately not here: the lease itself (the interlock grants it),
//! and binding a lease to the state it was checked against.

#![forbid(unsafe_code)]

mod decide;
mod facts;
mod request;

pub use decide::{class_rank, classify, ActionDigest, Decision, RefusalReason, Requirement};
pub use facts::{
    BusFact, BusFacts, FactError, Mastership, RangeError, SettingKind, ValueRange, WriteEntry,
    WriteMap,
};
pub use request::{
    BusListen, BusRead, BusWrite, EffectRequest, PolicyIdentError, PortId, SignalId, WriteValue,
    ACTION_ENCODING, MAX_POLICY_IDENT_BYTES,
};
pub use sawbones_interlock::HazardClass;
