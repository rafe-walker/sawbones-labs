//! # sawbones-approvals: PROVISIONAL
//!
//! **This crate is provisional. The Sawbones design is still under review, and
//! every type here may change or be replaced without notice.**
//!
//! Approvals as signatures, scaled by risk (#172).
//!
//! The principle: **the Raspberry Pi carries data but does not vouch for
//! it.** The keys that matter live off the Pi: the trust-root
//! microcontroller (the panel button), the owner's phone, and each module.
//! Something that acts on an approval (the bus guard, the installer) checks
//! the signature itself, so a compromised Pi can relay an approval but can
//! neither forge nor replay one.
//!
//! ## What a signed approval says
//!
//! A [`Message`] names:
//!
//! - **what is approved**, an [`Approval`]: install a core release, grant a
//!   bus-guard lease, update the bus-guard allow-list, grow a plugin's
//!   permissions, install a native package, or attest a fact as the
//!   operator. Each carries the digest of the exact thing approved;
//! - **the device it is for**, a [`DeviceId`], so an approval for one bus
//!   guard cannot be replayed on another that trusts the same key;
//! - **who signed it**, a [`Role`]: the trust root, the owner's phone, or a
//!   module;
//! - **a counter**, strictly increasing per signer, so a captured approval
//!   cannot be sent again;
//! - **an expiry tick**, after which it is refused.
//!
//! It has one encoding: [`MESSAGE_LEN`] bytes in a fixed layout, documented
//! on [`Message`] and pinned by golden tests. The signature is Ed25519 over
//! [`DOMAIN`] followed by those bytes, so a signature made for anything else
//! cannot be taken for an approval.
//!
//! ## Who may approve what
//!
//! [`may_approve`] is the policy, from the approval scale in #172:
//!
//! | Approval | Trust root (panel press) | Owner phone | Module |
//! | --- | --- | --- | --- |
//! | Grant lease (energy) | yes | no | no |
//! | Bus-guard allow-list update (energy) | yes | no | no |
//! | Install core release (trust-root countersign) | yes | no | no |
//! | Plugin permission growth | yes | yes | no |
//! | Native install | yes | yes | no |
//! | Operator attestation | yes | yes | no |
//!
//! Modules sign their own measurements, not approvals, so a module key
//! approves nothing here.
//!
//! ## Checking one
//!
//! A [`Verifier`] holds the public key for each role and the last counter it
//! accepted from each signer. [`Verifier::verify`] returns an [`Approved<T>`]
//! only if every check passes, and a typed [`Refusal`] otherwise. A refusal
//! changes nothing. [`Approved<T>`] has private fields and no other
//! constructor, so code that takes one as an argument can only be called
//! with an approval that was checked.
//!
//! ## Test keys
//!
//! The `test-keys` feature adds [`test_keys`](crate#test-keys): fixed
//! keypairs whose private halves are in this repository. **They are never
//! for real hardware.** Without the feature the crate contains no private
//! key, and [`Verifier::new`] and the enroll calls refuse the test public keys
//! with [`KeyError::TestKey`].
//!
//! ## What is deliberately not here yet
//!
//! - **Persistence.** The last counter per signer lives in the
//!   [`Verifier`]. Across a restart the firmware must store it and give it
//!   back with [`Verifier::restore_counter`], and must store the new value
//!   before it acts on an approval. Otherwise an approval accepted before a
//!   power cut can be replayed after it.
//! - **The clock.** The expiry is a tick on the verifier's clock, compared
//!   with the `now` it is given. How the signer learns that clock is not
//!   settled.
//! - **Key provisioning and rotation.** The keys are given to the
//!   [`Verifier`] when it is built. Where they come from on a real device,
//!   and how one is replaced, is for the hardware phase.
//!
//! The crate reads no clock, allocates nothing and is `no_std`, so the same
//! code runs under test on a laptop and in the trust root's firmware.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "test-keys")]
pub mod test_keys;

use ed25519_dalek::VerifyingKey;

/// A tick on the verifier's clock. The tick period is the caller's choice
/// and is not fixed yet.
pub type Tick = u64;

/// The bytes signed before every message, so a signature over an approval
/// cannot be mistaken for a signature over anything else, and the reverse.
pub const DOMAIN: [u8; 22] = *b"sawbones-approvals/v1\0";

/// The format version, the first byte of every message.
pub const FORMAT_VERSION: u8 = 1;

/// The length of an encoded [`Message`], in bytes.
pub const MESSAGE_LEN: usize = 91;

/// The length of what is signed: [`DOMAIN`] followed by the message.
pub const SIGNED_LEN: usize = DOMAIN.len() + MESSAGE_LEN;

/// How many module keys a [`Verifier`] holds.
pub const MAX_MODULES: usize = 8;

const OFFSET_VERSION: usize = 0;
const OFFSET_TARGET: usize = 1;
const OFFSET_ROLE: usize = 5;
const OFFSET_MODULE: usize = 6;
const OFFSET_COUNTER: usize = 10;
const OFFSET_EXPIRES: usize = 18;
const OFFSET_KIND: usize = 26;
const OFFSET_PAYLOAD: usize = 27;
const PAYLOAD_LEN: usize = MESSAGE_LEN - OFFSET_PAYLOAD;

const ROLE_TRUST_ROOT: u8 = 1;
const ROLE_OWNER_PHONE: u8 = 2;
const ROLE_MODULE: u8 = 3;

const KIND_INSTALL_CORE_RELEASE: u8 = 1;
const KIND_GRANT_LEASE: u8 = 2;
const KIND_BUS_GUARD_ALLOW_LIST: u8 = 3;
const KIND_PLUGIN_PERMISSION_GROWTH: u8 = 4;
const KIND_NATIVE_INSTALL: u8 = 5;
const KIND_OPERATOR_ATTESTATION: u8 = 6;

/// A SHA-256 digest of the exact thing approved. What it is the digest of
/// depends on the [`Approval`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Digest(pub [u8; 32]);

/// The device an approval is for: one bus guard, one installer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeviceId(pub u32);

/// A module, by the id it was enrolled under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId(pub u32);

/// A plugin, by a fixed 32-byte id (for example a hash of its name).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PluginId(pub [u8; 32]);

/// Who signed an approval. Each role has its own key, and the role is part
/// of what is signed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// The trust-root microcontroller, after a press of the panel button.
    TrustRoot,
    /// The owner's phone, after a passkey prompt.
    OwnerPhone,
    /// A module.
    Module(ModuleId),
}

/// Install the core release with this digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstallCoreRelease {
    /// The digest of the release.
    pub release: Digest,
}

/// Grant a bus-guard lease: one write of exactly this frame, on this port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GrantLease {
    /// The bus-guard port the write goes out on.
    pub port: u8,
    /// The SHA-256 of the exact frame approved.
    pub action: Digest,
}

/// Replace the bus guard's allow-list with the one that has this digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BusGuardAllowList {
    /// The digest of the allow-list's canonical encoding.
    pub allow_list: Digest,
}

/// Let a plugin hold more permissions than it did: those in the manifest
/// with this digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PluginPermissionGrowth {
    /// The plugin.
    pub plugin: PluginId,
    /// The digest of the manifest that holds the new permissions.
    pub manifest: Digest,
}

/// Install the native package with this digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeInstall {
    /// The digest of the package.
    pub package: Digest,
}

/// The operator attests the fact with this digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperatorAttestation {
    /// The digest of the fact.
    pub fact: Digest,
}

/// What an approval approves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Approval {
    /// See [`InstallCoreRelease`].
    InstallCoreRelease(InstallCoreRelease),
    /// See [`GrantLease`]. An energy kind.
    GrantLease(GrantLease),
    /// See [`BusGuardAllowList`]. An energy kind.
    BusGuardAllowList(BusGuardAllowList),
    /// See [`PluginPermissionGrowth`].
    PluginPermissionGrowth(PluginPermissionGrowth),
    /// See [`NativeInstall`].
    NativeInstall(NativeInstall),
    /// See [`OperatorAttestation`].
    OperatorAttestation(OperatorAttestation),
}

impl Approval {
    /// True for the kinds that let energy move: a lease and an allow-list
    /// update. They need the trust root.
    #[must_use]
    pub const fn is_energy(&self) -> bool {
        matches!(
            self,
            Approval::GrantLease(_) | Approval::BusGuardAllowList(_)
        )
    }

    /// True for the kinds only the trust root may approve: the energy kinds,
    /// and installing a core release, which the trust root countersigns.
    #[must_use]
    pub const fn needs_trust_root(&self) -> bool {
        self.is_energy() || matches!(self, Approval::InstallCoreRelease(_))
    }
}

/// True if `role` may approve `approval`. See the table in the crate docs.
#[must_use]
pub const fn may_approve(role: Role, approval: &Approval) -> bool {
    match role {
        Role::TrustRoot => true,
        Role::OwnerPhone => !approval.needs_trust_root(),
        Role::Module(_) => false,
    }
}

/// One approval, before it is signed or after it is decoded.
///
/// # Encoding
///
/// Exactly [`MESSAGE_LEN`] (91) bytes. Integers are big-endian. Every byte is
/// fixed by the message, so each message has exactly one encoding, and
/// [`Message::decode`] refuses any bytes [`Message::encode`] would not
/// produce.
///
/// | Offset | Length | Field |
/// | --- | --- | --- |
/// | 0 | 1 | Format version, [`FORMAT_VERSION`] (1) |
/// | 1 | 4 | Target device, [`DeviceId`] |
/// | 5 | 1 | Role: 1 trust root, 2 owner phone, 3 module |
/// | 6 | 4 | Module id for role 3, otherwise 0 |
/// | 10 | 8 | Counter |
/// | 18 | 8 | Expiry tick |
/// | 26 | 1 | Kind, below |
/// | 27 | 64 | Payload, below, then zeros to the end |
///
/// | Kind | Approval | Payload |
/// | --- | --- | --- |
/// | 1 | [`InstallCoreRelease`] | release digest (32) |
/// | 2 | [`GrantLease`] | port (1), frame digest (32) |
/// | 3 | [`BusGuardAllowList`] | allow-list digest (32) |
/// | 4 | [`PluginPermissionGrowth`] | plugin id (32), manifest digest (32) |
/// | 5 | [`NativeInstall`] | package digest (32) |
/// | 6 | [`OperatorAttestation`] | fact digest (32) |
///
/// The signature is Ed25519 over [`DOMAIN`] followed by these bytes; see
/// [`signed_bytes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Message {
    /// The device the approval is for.
    pub target: DeviceId,
    /// Who signs it.
    pub signer: Role,
    /// Strictly greater than any counter the target has accepted from this
    /// signer. Counters start at 1.
    pub counter: u64,
    /// The first tick, on the target's clock, at which it is refused.
    pub expires_at: Tick,
    /// What it approves.
    pub approval: Approval,
}

/// Why bytes are not an encoded [`Message`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecodeError {
    /// Not exactly [`MESSAGE_LEN`] bytes.
    Length,
    /// A format version other than [`FORMAT_VERSION`].
    Version,
    /// A role byte that names no role.
    UnknownRole,
    /// A non-zero module id with a role that is not a module.
    StrayModuleId,
    /// A kind byte that names no approval.
    UnknownKind,
    /// A non-zero byte after the payload.
    NonZeroPadding,
}

const fn put_u32(out: &mut [u8; MESSAGE_LEN], at: usize, value: u32) {
    let bytes = value.to_be_bytes();
    let mut i = 0;
    while i < 4 {
        out[at + i] = bytes[i];
        i += 1;
    }
}

const fn put_u64(out: &mut [u8; MESSAGE_LEN], at: usize, value: u64) {
    let bytes = value.to_be_bytes();
    let mut i = 0;
    while i < 8 {
        out[at + i] = bytes[i];
        i += 1;
    }
}

const fn put_32(out: &mut [u8; MESSAGE_LEN], at: usize, value: &[u8; 32]) {
    let mut i = 0;
    while i < 32 {
        out[at + i] = value[i];
        i += 1;
    }
}

fn get_u32(bytes: &[u8; MESSAGE_LEN], at: usize) -> u32 {
    let mut out = [0; 4];
    out.copy_from_slice(&bytes[at..at + 4]);
    u32::from_be_bytes(out)
}

fn get_u64(bytes: &[u8; MESSAGE_LEN], at: usize) -> u64 {
    let mut out = [0; 8];
    out.copy_from_slice(&bytes[at..at + 8]);
    u64::from_be_bytes(out)
}

fn get_32(bytes: &[u8; MESSAGE_LEN], at: usize) -> [u8; 32] {
    let mut out = [0; 32];
    out.copy_from_slice(&bytes[at..at + 32]);
    out
}

impl Message {
    /// The message's one encoding. See [`Message`] for the layout.
    #[must_use]
    pub const fn encode(&self) -> [u8; MESSAGE_LEN] {
        let mut out = [0; MESSAGE_LEN];
        out[OFFSET_VERSION] = FORMAT_VERSION;
        put_u32(&mut out, OFFSET_TARGET, self.target.0);
        let (role, module) = match self.signer {
            Role::TrustRoot => (ROLE_TRUST_ROOT, 0),
            Role::OwnerPhone => (ROLE_OWNER_PHONE, 0),
            Role::Module(id) => (ROLE_MODULE, id.0),
        };
        out[OFFSET_ROLE] = role;
        put_u32(&mut out, OFFSET_MODULE, module);
        put_u64(&mut out, OFFSET_COUNTER, self.counter);
        put_u64(&mut out, OFFSET_EXPIRES, self.expires_at);
        let p = OFFSET_PAYLOAD;
        match self.approval {
            Approval::InstallCoreRelease(a) => {
                out[OFFSET_KIND] = KIND_INSTALL_CORE_RELEASE;
                put_32(&mut out, p, &a.release.0);
            }
            Approval::GrantLease(a) => {
                out[OFFSET_KIND] = KIND_GRANT_LEASE;
                out[p] = a.port;
                put_32(&mut out, p + 1, &a.action.0);
            }
            Approval::BusGuardAllowList(a) => {
                out[OFFSET_KIND] = KIND_BUS_GUARD_ALLOW_LIST;
                put_32(&mut out, p, &a.allow_list.0);
            }
            Approval::PluginPermissionGrowth(a) => {
                out[OFFSET_KIND] = KIND_PLUGIN_PERMISSION_GROWTH;
                put_32(&mut out, p, &a.plugin.0);
                put_32(&mut out, p + 32, &a.manifest.0);
            }
            Approval::NativeInstall(a) => {
                out[OFFSET_KIND] = KIND_NATIVE_INSTALL;
                put_32(&mut out, p, &a.package.0);
            }
            Approval::OperatorAttestation(a) => {
                out[OFFSET_KIND] = KIND_OPERATOR_ATTESTATION;
                put_32(&mut out, p, &a.fact.0);
            }
        }
        out
    }

    /// Decodes a message, refusing anything [`Message::encode`] would not
    /// produce.
    ///
    /// # Errors
    ///
    /// A [`DecodeError`] saying which part is wrong.
    pub fn decode(bytes: &[u8]) -> Result<Message, DecodeError> {
        let bytes: &[u8; MESSAGE_LEN] = bytes.try_into().map_err(|_| DecodeError::Length)?;
        if bytes[OFFSET_VERSION] != FORMAT_VERSION {
            return Err(DecodeError::Version);
        }
        let module = get_u32(bytes, OFFSET_MODULE);
        let signer = match bytes[OFFSET_ROLE] {
            ROLE_TRUST_ROOT => Role::TrustRoot,
            ROLE_OWNER_PHONE => Role::OwnerPhone,
            ROLE_MODULE => Role::Module(ModuleId(module)),
            _ => return Err(DecodeError::UnknownRole),
        };
        if module != 0 && !matches!(signer, Role::Module(_)) {
            return Err(DecodeError::StrayModuleId);
        }
        let p = OFFSET_PAYLOAD;
        let digest = |at: usize| Digest(get_32(bytes, at));
        let (approval, used) = match bytes[OFFSET_KIND] {
            KIND_INSTALL_CORE_RELEASE => (
                Approval::InstallCoreRelease(InstallCoreRelease { release: digest(p) }),
                32,
            ),
            KIND_GRANT_LEASE => (
                Approval::GrantLease(GrantLease {
                    port: bytes[p],
                    action: digest(p + 1),
                }),
                33,
            ),
            KIND_BUS_GUARD_ALLOW_LIST => (
                Approval::BusGuardAllowList(BusGuardAllowList {
                    allow_list: digest(p),
                }),
                32,
            ),
            KIND_PLUGIN_PERMISSION_GROWTH => (
                Approval::PluginPermissionGrowth(PluginPermissionGrowth {
                    plugin: PluginId(get_32(bytes, p)),
                    manifest: digest(p + 32),
                }),
                64,
            ),
            KIND_NATIVE_INSTALL => (
                Approval::NativeInstall(NativeInstall { package: digest(p) }),
                32,
            ),
            KIND_OPERATOR_ATTESTATION => (
                Approval::OperatorAttestation(OperatorAttestation { fact: digest(p) }),
                32,
            ),
            _ => return Err(DecodeError::UnknownKind),
        };
        debug_assert!(used <= PAYLOAD_LEN);
        if bytes[p + used..].iter().any(|&b| b != 0) {
            return Err(DecodeError::NonZeroPadding);
        }
        Ok(Message {
            target: DeviceId(get_u32(bytes, OFFSET_TARGET)),
            signer,
            counter: get_u64(bytes, OFFSET_COUNTER),
            expires_at: get_u64(bytes, OFFSET_EXPIRES),
            approval,
        })
    }
}

/// What a signer signs for `message`: [`DOMAIN`] followed by the message.
#[must_use]
pub const fn signed_bytes(message: &[u8; MESSAGE_LEN]) -> [u8; SIGNED_LEN] {
    let mut out = [0; SIGNED_LEN];
    let mut i = 0;
    while i < DOMAIN.len() {
        out[i] = DOMAIN[i];
        i += 1;
    }
    let mut j = 0;
    while j < MESSAGE_LEN {
        out[DOMAIN.len() + j] = message[j];
        j += 1;
    }
    out
}

/// An Ed25519 signature, 64 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signature(pub [u8; 64]);

/// An Ed25519 public key, 32 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PublicKey(pub [u8; 32]);

/// The public halves of the keypairs in `test_keys`, in the order trust
/// root, owner phone, module 1, not enrolled. Present in every build so a
/// build without the `test-keys` feature can refuse them. Public keys only:
/// no private key is in a default build.
pub const TEST_PUBLIC_KEYS: [[u8; 32]; 4] = [
    [
        0xed, 0xc4, 0x38, 0x75, 0xd2, 0x83, 0x80, 0xb1, 0x06, 0x40, 0x5c, 0x2f, 0xd0, 0x05, 0x0b,
        0x37, 0x1b, 0x9b, 0xe8, 0x1c, 0x7b, 0xa7, 0x09, 0x0e, 0xf3, 0x4f, 0x5e, 0x80, 0x6f, 0x6a,
        0x69, 0x1b,
    ],
    [
        0x3a, 0x4e, 0x5c, 0x10, 0x7c, 0x68, 0x5e, 0x64, 0xb2, 0x17, 0x8c, 0x8b, 0xe1, 0x38, 0x4e,
        0x3e, 0xd8, 0x2c, 0xed, 0x31, 0x75, 0xf2, 0xaf, 0xd8, 0xe8, 0xe9, 0x8f, 0x63, 0x75, 0xad,
        0xdc, 0x83,
    ],
    [
        0x93, 0xa7, 0x01, 0x22, 0x44, 0xa5, 0x67, 0xbf, 0x7b, 0x44, 0x34, 0x9d, 0x92, 0x56, 0xdb,
        0x74, 0x61, 0x48, 0xa3, 0xd1, 0x9b, 0x14, 0x35, 0x4d, 0xbd, 0xdd, 0x82, 0xd8, 0xe4, 0x72,
        0x23, 0x13,
    ],
    [
        0x6b, 0xef, 0xdf, 0xcb, 0xa8, 0x50, 0xd3, 0xd0, 0xb7, 0xf4, 0xbf, 0x67, 0x7f, 0xd9, 0x52,
        0xf1, 0x5e, 0xd3, 0x66, 0x72, 0x58, 0xae, 0x15, 0x1d, 0xe0, 0x8b, 0x07, 0xc6, 0x02, 0x20,
        0x62, 0xbb,
    ],
];

impl PublicKey {
    /// True for the public half of one of the fixed test keypairs.
    #[must_use]
    pub fn is_test_key(&self) -> bool {
        TEST_PUBLIC_KEYS.contains(&self.0)
    }
}

/// Why a key could not be given to a [`Verifier`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyError {
    /// The bytes are not an Ed25519 public key.
    Invalid,
    /// A small-order key, for which signatures prove nothing.
    Weak,
    /// One of the fixed test keys, in a build without the `test-keys`
    /// feature.
    TestKey,
    /// The same key is already held for another role or module. Each role
    /// needs its own key, or the roles collapse into one.
    Reused,
    /// The owner phone or this module already has a key.
    AlreadyEnrolled,
    /// The verifier already holds [`MAX_MODULES`] module keys.
    Full,
}

/// Why [`Verifier::verify`] refused. A refusal changes nothing in the
/// verifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Refusal {
    /// The bytes are not an encoded [`Message`].
    Malformed(DecodeError),
    /// The approval is for another device.
    WrongDevice,
    /// The approval is of another kind than the caller asked for.
    WrongKind,
    /// The signer's role may not approve this kind; see [`may_approve`].
    RoleNotAllowed,
    /// The verifier holds no key for the signer named.
    UnknownSigner,
    /// The signature does not verify under the named signer's key.
    BadSignature,
    /// The approval has expired.
    Expired,
    /// The counter is not greater than the last one accepted from this
    /// signer: a replay, or one overtaken by a later approval.
    Replayed {
        /// The last counter accepted from this signer.
        last: u64,
    },
}

/// The signer named does not have a key in this [`Verifier`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnknownSigner;

mod sealed {
    pub trait Sealed {}
}

/// A kind of approval [`Verifier::verify`] can be asked for: one of the six
/// approval structs, or [`Approval`] for any kind. Sealed.
pub trait Approvable: sealed::Sealed + Sized {
    /// The approval, if it is of this kind.
    fn from_approval(approval: Approval) -> Option<Self>;
}

macro_rules! approvable {
    ($($kind:ident),*) => {$(
        impl sealed::Sealed for $kind {}
        impl Approvable for $kind {
            fn from_approval(approval: Approval) -> Option<Self> {
                match approval {
                    Approval::$kind(inner) => Some(inner),
                    _ => None,
                }
            }
        }
    )*};
}

approvable!(
    InstallCoreRelease,
    GrantLease,
    BusGuardAllowList,
    PluginPermissionGrowth,
    NativeInstall,
    OperatorAttestation
);

impl sealed::Sealed for Approval {}
impl Approvable for Approval {
    fn from_approval(approval: Approval) -> Option<Self> {
        Some(approval)
    }
}

/// Proof that an approval of kind `T` passed every check in
/// [`Verifier::verify`].
///
/// The fields are private and this crate builds one only in
/// [`Verifier::verify`], so a function that takes an `Approved<T>` can only
/// be called with a checked approval. It is not `Clone`, `Copy`, `Default`
/// or deserializable.
///
/// ```compile_fail
/// use sawbones_approvals::{Approved, Digest, GrantLease, Role};
/// // The fields are private: no struct literal.
/// let _ = Approved {
///     approval: GrantLease { port: 0, action: Digest([0; 32]) },
///     signer: Role::TrustRoot,
///     counter: 1,
///     expires_at: 10,
/// };
/// ```
///
/// ```compile_fail
/// use sawbones_approvals::{Approved, GrantLease};
/// fn needs_clone<T: Clone>() {}
/// // Not Clone: one checked approval cannot become two.
/// needs_clone::<Approved<GrantLease>>();
/// ```
#[derive(Debug, PartialEq, Eq)]
#[must_use]
pub struct Approved<T> {
    approval: T,
    signer: Role,
    counter: u64,
    expires_at: Tick,
}

impl<T> Approved<T> {
    /// What was approved.
    pub const fn approval(&self) -> &T {
        &self.approval
    }

    /// Who signed it.
    pub const fn signer(&self) -> Role {
        self.signer
    }

    /// Its counter.
    pub const fn counter(&self) -> u64 {
        self.counter
    }

    /// The first tick at which it would have been refused.
    pub const fn expires_at(&self) -> Tick {
        self.expires_at
    }

    /// What was approved, giving up the proof.
    pub fn into_approval(self) -> T {
        self.approval
    }
}

/// One key the verifier trusts, and the last counter it accepted under it.
#[derive(Debug)]
struct Enrolled {
    key: VerifyingKey,
    raw: [u8; 32],
    last_counter: u64,
}

impl Enrolled {
    fn new(key: PublicKey) -> Result<Enrolled, KeyError> {
        if key.is_test_key() && !cfg!(feature = "test-keys") {
            return Err(KeyError::TestKey);
        }
        let parsed = VerifyingKey::from_bytes(&key.0).map_err(|_| KeyError::Invalid)?;
        if parsed.is_weak() {
            return Err(KeyError::Weak);
        }
        Ok(Enrolled {
            key: parsed,
            raw: key.0,
            last_counter: 0,
        })
    }
}

/// Checks signed approvals for one device.
///
/// Holds the trust root's key, the owner phone's key if enrolled, up to
/// [`MAX_MODULES`] module keys, and the last counter accepted from each. Not
/// `Clone`: two copies would each accept the same approval once.
#[derive(Debug)]
pub struct Verifier {
    device: DeviceId,
    trust_root: Enrolled,
    owner_phone: Option<Enrolled>,
    modules: [Option<(ModuleId, Enrolled)>; MAX_MODULES],
}

impl Verifier {
    /// A verifier for `device` that trusts `trust_root`. Every counter starts
    /// at 0, so the first approval accepted from a signer has counter 1 or
    /// more.
    ///
    /// # Errors
    ///
    /// [`KeyError::Invalid`], [`KeyError::Weak`] or [`KeyError::TestKey`].
    pub fn new(device: DeviceId, trust_root: PublicKey) -> Result<Verifier, KeyError> {
        Ok(Verifier {
            device,
            trust_root: Enrolled::new(trust_root)?,
            owner_phone: None,
            modules: [const { None }; MAX_MODULES],
        })
    }

    /// The device this verifier checks approvals for.
    #[must_use]
    pub const fn device(&self) -> DeviceId {
        self.device
    }

    /// Trusts `key` as the owner's phone.
    ///
    /// # Errors
    ///
    /// [`KeyError::AlreadyEnrolled`] if a phone key is held, or any error
    /// from checking the key, including [`KeyError::Reused`].
    pub fn enroll_owner_phone(&mut self, key: PublicKey) -> Result<(), KeyError> {
        if self.owner_phone.is_some() {
            return Err(KeyError::AlreadyEnrolled);
        }
        let enrolled = self.checked(key)?;
        self.owner_phone = Some(enrolled);
        Ok(())
    }

    /// Trusts `key` as module `id`.
    ///
    /// # Errors
    ///
    /// [`KeyError::AlreadyEnrolled`] if the module has a key,
    /// [`KeyError::Full`] if there is no room, or any error from checking the
    /// key, including [`KeyError::Reused`].
    pub fn enroll_module(&mut self, id: ModuleId, key: PublicKey) -> Result<(), KeyError> {
        if self.modules.iter().flatten().any(|(held, _)| *held == id) {
            return Err(KeyError::AlreadyEnrolled);
        }
        let enrolled = self.checked(key)?;
        let slot = self
            .modules
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or(KeyError::Full)?;
        *slot = Some((id, enrolled));
        Ok(())
    }

    /// The last counter accepted from `signer`, or `None` if no key is held
    /// for it.
    #[must_use]
    pub fn last_counter(&self, signer: Role) -> Option<u64> {
        self.enrolled(signer).map(|e| e.last_counter)
    }

    /// Raises the last counter accepted from `signer` to `last`, for example
    /// to the value stored before a restart. Never lowers it.
    ///
    /// # Errors
    ///
    /// [`UnknownSigner`] if no key is held for `signer`.
    pub fn restore_counter(&mut self, signer: Role, last: u64) -> Result<(), UnknownSigner> {
        let enrolled = self.enrolled_mut(signer).ok_or(UnknownSigner)?;
        enrolled.last_counter = enrolled.last_counter.max(last);
        Ok(())
    }

    /// Checks one signed approval of kind `T` at tick `now`.
    ///
    /// It passes only if, in this order: the bytes decode as a [`Message`];
    /// it is for this device; it approves a `T`; its role may approve that
    /// ([`may_approve`]); a key is held for its signer; the signature
    /// verifies (strict Ed25519) under that key over [`signed_bytes`]; `now`
    /// is before its expiry; and its counter is greater than the last
    /// accepted from that signer. Then the counter is recorded and the
    /// approval returned.
    ///
    /// # Errors
    ///
    /// The first check that failed, as a [`Refusal`]. Nothing is recorded.
    pub fn verify<T: Approvable>(
        &mut self,
        message: &[u8],
        signature: &Signature,
        now: Tick,
    ) -> Result<Approved<T>, Refusal> {
        let decoded = Message::decode(message).map_err(Refusal::Malformed)?;
        let bytes: &[u8; MESSAGE_LEN] = message
            .try_into()
            .map_err(|_| Refusal::Malformed(DecodeError::Length))?;
        if decoded.target != self.device {
            return Err(Refusal::WrongDevice);
        }
        let approval = T::from_approval(decoded.approval).ok_or(Refusal::WrongKind)?;
        if !may_approve(decoded.signer, &decoded.approval) {
            return Err(Refusal::RoleNotAllowed);
        }
        let enrolled = self
            .enrolled_mut(decoded.signer)
            .ok_or(Refusal::UnknownSigner)?;
        let signature = ed25519_dalek::Signature::from_bytes(&signature.0);
        enrolled
            .key
            .verify_strict(&signed_bytes(bytes), &signature)
            .map_err(|_| Refusal::BadSignature)?;
        if now >= decoded.expires_at {
            return Err(Refusal::Expired);
        }
        if decoded.counter <= enrolled.last_counter {
            return Err(Refusal::Replayed {
                last: enrolled.last_counter,
            });
        }
        enrolled.last_counter = decoded.counter;
        Ok(Approved {
            approval,
            signer: decoded.signer,
            counter: decoded.counter,
            expires_at: decoded.expires_at,
        })
    }

    fn checked(&self, key: PublicKey) -> Result<Enrolled, KeyError> {
        let enrolled = Enrolled::new(key)?;
        let reused = self.trust_root.raw == key.0
            || self.owner_phone.as_ref().is_some_and(|e| e.raw == key.0)
            || self.modules.iter().flatten().any(|(_, e)| e.raw == key.0);
        if reused {
            return Err(KeyError::Reused);
        }
        Ok(enrolled)
    }

    fn enrolled(&self, signer: Role) -> Option<&Enrolled> {
        match signer {
            Role::TrustRoot => Some(&self.trust_root),
            Role::OwnerPhone => self.owner_phone.as_ref(),
            Role::Module(id) => self
                .modules
                .iter()
                .flatten()
                .find(|(held, _)| *held == id)
                .map(|(_, e)| e),
        }
    }

    fn enrolled_mut(&mut self, signer: Role) -> Option<&mut Enrolled> {
        match signer {
            Role::TrustRoot => Some(&mut self.trust_root),
            Role::OwnerPhone => self.owner_phone.as_mut(),
            Role::Module(id) => self
                .modules
                .iter_mut()
                .flatten()
                .find(|(held, _)| *held == id)
                .map(|(_, e)| e),
        }
    }
}
