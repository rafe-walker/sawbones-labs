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
//! **Fact scope, invalidation and preflight (#110).** A fact is only as good
//! as the place, device, firmware, conditions and method it was earned on.
//! A [`FactLedger`] keeps each write-map entry and mastership fact a write
//! rests on with its [`FactScope`] and a version, and applies the
//! [`FactEvent`]s that make facts stale: a firmware, serial or model change,
//! a lost device, a rewired port, a foreign master, a reflashed instrument
//! (and every module qualified against it), a condition that ends, an open
//! contradiction and a break in the clock. [`preflight`] runs before a
//! request is classified: it checks every fact the request relies on
//! ([`relied_facts`], worked out from the request, never from the agent)
//! and refuses with a [`PreflightRefusal`] that names the fact, or returns a
//! [`PreflightToken`] bound to the request, its action digest, the fact
//! versions and the chain head. [`confirm`] consumes the token at the moment
//! of action and refuses if anything changed. A simulated firmware change
//! makes a dependent preflight fail:
//!
//! ```
//! use sawbones_policy::*;
//! use sawbones_reading::Trust;
//! use std::collections::BTreeSet;
//!
//! let port = PortId::parse("rs485-1").unwrap();
//! let signal = SignalId::parse("charge_voltage").unwrap();
//! let entry = WriteEntry {
//!     port: port.clone(),
//!     unit: 1,
//!     signal: signal.clone(),
//!     register: 0x0020,
//!     class: HazardClass::Class3,
//!     kind: SettingKind::ChargeVoltage,
//!     energy_relevant: true,
//!     trust: Trust::OperatorAttested,
//!     range: None,
//! };
//! let bus = BusFact {
//!     port: port.clone(),
//!     bench_port: 0,
//!     mastership: Mastership::BenchOnly,
//!     trust: Trust::OperatorAttested,
//! };
//! let writes = WriteMap::new([entry.clone()]).unwrap();
//! let buses = BusFacts::new([bus.clone()]).unwrap();
//! let device = |firmware: &str| DeviceIdentity {
//!     model: ScopeName::parse("inverter-a").unwrap(),
//!     serial: ScopeLabel::parse("SN0001").unwrap(),
//!     firmware: ScopeLabel::parse(firmware).unwrap(),
//! };
//!
//! let mut ledger = FactLedger::new(LedgerId(7));
//! ledger
//!     .apply(FactEvent::DeviceIdentified { port: port.clone(), unit: 1, identity: device("V1.0") }, 1)
//!     .unwrap();
//! let scope = |unit, device| FactScope {
//!     device,
//!     interface: Interface { port: port.clone(), bench_port: 0, unit },
//!     conditions: BTreeSet::new(),
//!     method: Method::OperatorAttested,
//! };
//! ledger.record(FactRecord {
//!     content: FactContent::WriteEntry(entry),
//!     scope: scope(Some(1), Some(device("V1.0"))),
//!     earned_at: 1,
//!     validity: Validity::UntilInvalidated,
//! }, 2).unwrap();
//! ledger.record(FactRecord {
//!     content: FactContent::Mastership(bus),
//!     scope: scope(None, None),
//!     earned_at: 1,
//!     validity: Validity::For { ticks: 1_000 },
//! }, 2).unwrap();
//!
//! let request = EffectRequest::BusWrite(BusWrite { port: port.clone(), unit: 1, signal, value: WriteValue::U16(5600) });
//! let head = ChainHead([0; 32]);
//! assert!(preflight(&request, &writes, &buses, &ledger, head, 3).is_ok());
//!
//! // The inverter is reflashed.
//! ledger
//!     .apply(FactEvent::DeviceIdentified { port: port.clone(), unit: 1, identity: device("V1.1") }, 4)
//!     .unwrap();
//! let refusal = preflight(&request, &writes, &buses, &ledger, head, 5).unwrap_err();
//! assert!(matches!(
//!     refusal,
//!     PreflightRefusal::Fact { fact: FactKey::WriteEntry { .. }, problem: FactProblem::Invalidated { .. } }
//! ));
//! ```
//!
//! What is deliberately not here: the lease itself (the interlock grants it),
//! where ledger events come from and who signs them (#172), and the MCP
//! `invoke_*` tools that will present a [`PreflightToken`]. [`classify`]
//! stays a pure function of the write map and bus facts; the host's action
//! path must go through [`preflight`] and [`confirm`].

#![forbid(unsafe_code)]

mod decide;
mod facts;
mod preflight;
mod request;

pub use decide::{
    bus_action, class_rank, classify, ActionDigest, Decision, RefusalReason, Requirement,
};
pub use facts::{
    BusFact, BusFacts, FactError, Mastership, RangeError, SettingKind, ValueRange, WriteEntry,
    WriteMap,
};
pub use preflight::{
    confirm, preflight, relied_facts, Applied, ChainHead, Confirmed, DeviceIdentity, EventRefusal,
    FactContent, FactEvent, FactKey, FactLedger, FactProblem, FactRecord, FactScope, FactVersion,
    Interface, InvalidationCause, LedgerId, Method, PreflightRefusal, PreflightToken, RecordError,
    Reference, ScopeField, ScopeLabel, ScopeLabelError, ScopeName, TokenProblem, Uncertainty,
    Validity, MAX_SCOPE_LABEL_BYTES, PREFLIGHT_TTL_TICKS,
};
pub use request::{
    BusListen, BusRead, BusWrite, EffectRequest, PolicyIdentError, PortId, SignalId, WriteValue,
    MAX_POLICY_IDENT_BYTES, REQUEST_ENCODING,
};
pub use sawbones_interlock::{HazardClass, Tick};
pub use sawbones_modbus_encode::{BusWriteAction, WriteFunction};
