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
//!   [`BusWrite`] of a typed [`WriteValue`] to a named [`SignalId`] of one
//!   Modbus unit on a [`PortId`]. A write to a raw address cannot be written
//!   down.
//! - the [`WriteMap`]: one [`WriteEntry`] fact per writable signal of each
//!   unit, with its register, its class, its [`SettingKind`], whether it is
//!   energy-relevant and the [`Trust`](sawbones_reading::Trust) tier of the
//!   entry itself.
//! - the [`BusFacts`]: one fact per port, saying which bench port it is wired
//!   to and whether the bench is the only master on that bus.
//!
//! The answer is a [`Decision`]: a [`HazardClass`] and the [`Requirement`]
//! that class carries: nothing, armed rail A, a rail B lease bound to one
//! [`BusWriteAction`] and its [`ActionDigest`], or a refusal with a typed
//! [`RefusalReason`]. The rules are listed on [`classify`].
//!
//! **One encoder (#203).** The bus action is built with the encoder the
//! approvals and the bus guard share (`sawbones-modbus-encode`), so the
//! digest the interlock binds rail B to, the action the trust root approves
//! and the frame the bus guard passes are the same action.
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
//!     unit: 1,
//!     signal: signal.clone(),
//!     register: 0x0010,
//!     class: HazardClass::Class2,
//!     kind: SettingKind::BatteryProtectionLimit,
//!     energy_relevant: true,
//!     trust: Trust::OperatorAttested,
//!     range: Some(ValueRange::new(WriteValue::U16(4400), WriteValue::U16(5000)).unwrap()),
//! }])
//! .unwrap();
//! let buses = BusFacts::new([BusFact {
//!     port: port.clone(),
//!     bench_port: 0,
//!     mastership: Mastership::BenchOnly,
//!     trust: Trust::OperatorAttested,
//! }])
//! .unwrap();
//!
//! let request = EffectRequest::BusWrite(BusWrite {
//!     port,
//!     unit: 1,
//!     signal,
//!     value: WriteValue::U16(4600),
//! });
//! let decision = classify(&request, &writes, &buses);
//! // A battery protection limit is class 3, whatever its entry says.
//! assert_eq!(decision.class(), HazardClass::Class3);
//! let Requirement::RailBLease { action, .. } = decision.requirement() else { panic!() };
//! // The lease is for one exact frame: write 4600 to register 0x0010 of unit 1.
//! assert_eq!(
//!     action.frame().as_bytes(),
//!     [0x01, 0x06, 0x00, 0x10, 0x11, 0xF8, 0x85, 0xDD]
//! );
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

pub use decide::{
    bus_action, class_rank, classify, ActionDigest, Decision, RefusalReason, Requirement,
};
pub use facts::{
    BusFact, BusFacts, FactError, Mastership, RangeError, SettingKind, ValueRange, WriteEntry,
    WriteMap,
};
pub use request::{
    BusListen, BusRead, BusWrite, EffectRequest, PolicyIdentError, PortId, SignalId, WriteValue,
    MAX_POLICY_IDENT_BYTES, REQUEST_ENCODING,
};
pub use sawbones_interlock::HazardClass;
pub use sawbones_modbus_encode::{BusWriteAction, WriteFunction};
