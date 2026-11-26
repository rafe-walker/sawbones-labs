//! Module identity by a signed challenge, and the manifest digest.

use core::num::NonZeroU64;

use sha2::{Digest, Sha256};

use crate::message::{Capabilities, Challenge, ImageDigest};
use crate::untrusted::ModuleBytes;
use crate::ModuleId;

/// The bytes signed before every identity, so an identity signature can
/// never be taken for any other signature, and the reverse.
pub const IDENTITY_DOMAIN: [u8; 28] = *b"sawbones-module-identity/v1\0";

/// The bytes hashed before a manifest's pages.
pub const MANIFEST_DOMAIN: [u8; 28] = *b"sawbones-module-manifest/v1\0";

/// The length of [`IdentityClaim::signed_bytes`]: the domain, the challenge
/// (16), the session (8), the module id (4), `can_emit` (1), the highest
/// class (1), the manifest digest (32) and the firmware digest (32).
pub const IDENTITY_SIGNED_LEN: usize = IDENTITY_DOMAIN.len() + 16 + 8 + 4 + 1 + 1 + 32 + 32;

/// The SHA-256 digest of a module's manifest: [`MANIFEST_DOMAIN`], then every
/// page's bytes in order. Pinned at install, and covered by the identity
/// signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ManifestDigest(pub [u8; 32]);

/// Hashes a manifest page by page, with no allocation.
#[derive(Debug, Clone)]
pub struct ManifestHasher {
    hasher: Sha256,
}

impl Default for ManifestHasher {
    fn default() -> Self {
        ManifestHasher::new()
    }
}

impl ManifestHasher {
    /// A hasher with nothing hashed but the domain.
    #[must_use]
    pub fn new() -> Self {
        let mut hasher = Sha256::new();
        hasher.update(MANIFEST_DOMAIN);
        ManifestHasher { hasher }
    }

    /// Adds the next page as the module sent it.
    pub fn page(&mut self, page: ModuleBytes<'_>) {
        self.hasher.update(page.as_untrusted_bytes());
    }

    /// Adds the next page from the module's own copy, for firmware and for
    /// the install-time pin.
    pub fn bytes(&mut self, bytes: &[u8]) {
        self.hasher.update(bytes);
    }

    /// The digest.
    #[must_use]
    pub fn finish(self) -> ManifestDigest {
        ManifestDigest(self.hasher.finalize().into())
    }
}

/// An Ed25519 signature over [`IdentityClaim::signed_bytes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdentitySignature(pub [u8; 64]);

/// Who a module says it is. A claim until `IdentityClaim::verify` (feature
/// `verify`) checks the signature against the key enrolled for that module
/// id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdentityClaim {
    /// The id the module claims.
    pub module: ModuleId,
    /// What it claims it can do.
    pub capabilities: Capabilities,
    /// The digest of its manifest.
    pub manifest_digest: ManifestDigest,
    /// The digest of the firmware it runs.
    pub firmware_digest: ImageDigest,
    /// Its signature.
    pub signature: IdentitySignature,
}

impl IdentityClaim {
    /// The bytes the module signs, and the host checks: [`IDENTITY_DOMAIN`],
    /// the host's challenge, the session the module opened, then every field
    /// but the signature, little-endian. A signature for one challenge or
    /// session never answers another, and changing any claimed field, such
    /// as `can_emit`, voids it.
    #[must_use]
    pub fn signed_bytes(
        &self,
        challenge: &Challenge,
        session: NonZeroU64,
    ) -> [u8; IDENTITY_SIGNED_LEN] {
        let mut out = [0u8; IDENTITY_SIGNED_LEN];
        let parts: [&[u8]; 8] = [
            &IDENTITY_DOMAIN,
            &challenge.0,
            &session.get().to_le_bytes(),
            &self.module.0.to_le_bytes(),
            &[u8::from(self.capabilities.can_emit)],
            &[self.capabilities.class_byte()],
            &self.manifest_digest.0,
            &self.firmware_digest.0,
        ];
        let mut at = 0;
        for part in parts {
            if let Some(slot) = out.get_mut(at..at + part.len()) {
                slot.copy_from_slice(part);
            }
            at += part.len();
        }
        out
    }
}

#[cfg(feature = "verify")]
mod verify {
    use core::num::NonZeroU64;

    use ed25519_dalek::{Signature, VerifyingKey};

    use super::{IdentityClaim, ManifestDigest};
    use crate::message::{Capabilities, Challenge, ImageDigest};
    use crate::ModuleId;

    /// The public key enrolled for one module.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ModuleKey(pub [u8; 32]);

    /// Why an identity did not verify.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum IdentityError {
        /// The module claims an id other than the one the key is enrolled
        /// for.
        WrongModule {
            /// The id the key is enrolled for.
            expected: ModuleId,
            /// The id the module claimed.
            claimed: ModuleId,
        },
        /// The enrolled key is not a valid Ed25519 public key.
        BadKey,
        /// The signature does not verify over this challenge, session and
        /// claim.
        BadSignature,
    }

    /// An identity whose signature verified against the key enrolled for its
    /// module, over this host's challenge and this session. Private fields,
    /// made only by [`IdentityClaim::verify`].
    ///
    /// It proves the module holds the enrolled key. It does not prove the
    /// module is on the port it was enrolled on, or that its readings are
    /// right: the supervisor's registry (#100) and the known-answer check
    /// (#178) decide those.
    ///
    /// ```compile_fail
    /// use sawbones_module_protocol::VerifiedIdentity;
    /// let _ = VerifiedIdentity { module: todo!(), capabilities: todo!(),
    ///     manifest_digest: todo!(), firmware_digest: todo!(), session: todo!() };
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct VerifiedIdentity {
        module: ModuleId,
        capabilities: Capabilities,
        manifest_digest: ManifestDigest,
        firmware_digest: ImageDigest,
        session: NonZeroU64,
    }

    impl VerifiedIdentity {
        /// The module.
        #[must_use]
        pub const fn module(&self) -> ModuleId {
            self.module
        }

        /// Its signed capabilities.
        #[must_use]
        pub const fn capabilities(&self) -> Capabilities {
            self.capabilities
        }

        /// Its signed manifest digest. Compare it with the pin taken at
        /// install and with the pages it sends.
        #[must_use]
        pub const fn manifest_digest(&self) -> ManifestDigest {
            self.manifest_digest
        }

        /// Its signed firmware digest. A change means a reflash: every fact
        /// it made is unverified again (#110).
        #[must_use]
        pub const fn firmware_digest(&self) -> ImageDigest {
            self.firmware_digest
        }

        /// The session the signature is bound to.
        #[must_use]
        pub const fn session(&self) -> NonZeroU64 {
            self.session
        }
    }

    impl IdentityClaim {
        /// Checks the claim against the key enrolled for `expected`, over the
        /// host's `challenge` and the `session` the module opened.
        ///
        /// # Errors
        ///
        /// [`IdentityError`]: the wrong module, a bad key or a bad signature.
        pub fn verify(
            &self,
            expected: ModuleId,
            key: &ModuleKey,
            challenge: &Challenge,
            session: NonZeroU64,
        ) -> Result<VerifiedIdentity, IdentityError> {
            if self.module != expected {
                return Err(IdentityError::WrongModule {
                    expected,
                    claimed: self.module,
                });
            }
            let key = VerifyingKey::from_bytes(&key.0).map_err(|_| IdentityError::BadKey)?;
            let signature = Signature::from_bytes(&self.signature.0);
            key.verify_strict(&self.signed_bytes(challenge, session), &signature)
                .map_err(|_| IdentityError::BadSignature)?;
            Ok(VerifiedIdentity {
                module: self.module,
                capabilities: self.capabilities,
                manifest_digest: self.manifest_digest,
                firmware_digest: self.firmware_digest,
                session,
            })
        }
    }
}

#[cfg(feature = "verify")]
pub use verify::{IdentityError, ModuleKey, VerifiedIdentity};
