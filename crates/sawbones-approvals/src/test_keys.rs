//! Fixed test keypairs. **NEVER FOR REAL HARDWARE.**
//!
//! The private halves are in this file, in a public repository: anyone can
//! sign with them. They exist so tests and simulation can make real
//! signatures. This module is compiled only with the `test-keys` feature,
//! and a build without that feature refuses these public keys
//! ([`KeyError::TestKey`](crate::KeyError::TestKey)).
//!
//! Each seed is readable ASCII that starts with the same marker, so a stray
//! copy is easy to find in a binary. CI builds the crate without the feature
//! and fails if the marker appears in the output.

use ed25519_dalek::{Signer, SigningKey};

use crate::{signed_bytes, DeviceId, ModuleId, PublicKey, Signature, Verifier, MESSAGE_LEN};

/// TEST ONLY. The trust root's seed.
pub const TRUST_ROOT_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY trust-root    ";

/// TEST ONLY. The owner phone's seed.
pub const OWNER_PHONE_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY owner-phone   ";

/// TEST ONLY. The seed of module [`TEST_MODULE`].
pub const MODULE_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY module-1      ";

/// TEST ONLY. A seed no [`verifier`] trusts, for forgery tests.
pub const NOT_ENROLLED_SEED: [u8; 32] = *b"SAWBONES-TEST-KEY not-enrolled  ";

/// Every test seed, in the order trust root, owner phone, module, not
/// enrolled.
pub const ALL_SEEDS: [[u8; 32]; 4] = [
    TRUST_ROOT_SEED,
    OWNER_PHONE_SEED,
    MODULE_SEED,
    NOT_ENROLLED_SEED,
];

/// The module [`MODULE_SEED`] is enrolled as.
pub const TEST_MODULE: ModuleId = ModuleId(1);

/// The public key for `seed`.
#[must_use]
pub fn public_key(seed: &[u8; 32]) -> PublicKey {
    PublicKey(SigningKey::from_bytes(seed).verifying_key().to_bytes())
}

/// Signs `message` with `seed`, over [`DOMAIN`](crate::DOMAIN) and the
/// message, as a real signer would.
#[must_use]
pub fn sign(seed: &[u8; 32], message: &[u8; MESSAGE_LEN]) -> Signature {
    Signature(
        SigningKey::from_bytes(seed)
            .sign(&signed_bytes(message))
            .to_bytes(),
    )
}

/// A verifier for `device` trusting the test trust root, the test owner
/// phone and the test module.
///
/// # Panics
///
/// Never: the test keys are valid and distinct.
#[must_use]
pub fn verifier(device: DeviceId) -> Verifier {
    let mut verifier =
        Verifier::new(device, public_key(&TRUST_ROOT_SEED)).expect("test trust-root key");
    verifier
        .enroll_owner_phone(public_key(&OWNER_PHONE_SEED))
        .expect("test owner-phone key");
    verifier
        .enroll_module(TEST_MODULE, public_key(&MODULE_SEED))
        .expect("test module key");
    verifier
}
