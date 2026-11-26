//! Randomised: no message passes without the matching key.
//!
//! Random messages of every kind, role and counter, signed by a random choice
//! of the enrolled keys, a key nobody trusts, or random bytes. A message may
//! pass only if it was signed by the key of the role it names, that role may
//! approve it, it is for this device, it has not expired and its counter is
//! new. Every pass is checked against that independent rule, and so is every
//! refusal.

mod common;

use common::{seed_for, DEVICE};
use sawbones_approvals::test_keys::{sign, verifier, NOT_ENROLLED_SEED, TEST_MODULE};
use sawbones_approvals::{
    may_approve, Approval, BusGuardAllowList, BusWriteAction, DeviceId, Digest, GrantLease,
    InstallCoreRelease, Message, ModuleId, NativeInstall, OperatorAttestation, PluginId,
    PluginPermissionGrowth, Refusal, Role, Signature, WriteFunction, MESSAGE_LEN,
};
use sawbones_modbus_encode::MAX_ACTION_WORDS;

/// A small xorshift generator, so the sequences are the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn bytes32(&mut self) -> [u8; 32] {
        let mut out = [0; 32];
        for chunk in out.chunks_mut(8) {
            chunk.copy_from_slice(&self.next().to_le_bytes());
        }
        out
    }
}

fn random_approval(rng: &mut Rng) -> Approval {
    let digest = Digest(rng.bytes32());
    match rng.below(6) {
        0 => Approval::InstallCoreRelease(InstallCoreRelease { release: digest }),
        1 => {
            let count = 1 + rng.below(MAX_ACTION_WORDS as u64) as usize;
            let values: Vec<u16> = (0..count).map(|_| rng.next() as u16).collect();
            let function = if count == 1 && rng.below(2) == 0 {
                WriteFunction::WriteSingleRegister
            } else {
                WriteFunction::WriteMultipleRegisters
            };
            let unit = 1 + rng.below(247) as u8;
            let register = rng.next() as u16 % (0xFFFF - 8);
            Approval::GrantLease(GrantLease {
                action: BusWriteAction::new(rng.next() as u8, unit, function, register, &values)
                    .unwrap(),
                action_digest: digest,
            })
        }
        2 => Approval::BusGuardAllowList(BusGuardAllowList { allow_list: digest }),
        3 => Approval::PluginPermissionGrowth(PluginPermissionGrowth {
            plugin: PluginId(rng.bytes32()),
            manifest: digest,
        }),
        4 => Approval::NativeInstall(NativeInstall { package: digest }),
        _ => Approval::OperatorAttestation(OperatorAttestation { fact: digest }),
    }
}

fn random_role(rng: &mut Rng) -> Role {
    match rng.below(4) {
        0 => Role::TrustRoot,
        1 => Role::OwnerPhone,
        2 => Role::Module(TEST_MODULE),
        _ => Role::Module(ModuleId(rng.below(4) as u32 + 2)),
    }
}

/// Who actually signed.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Signer {
    Key(Role),
    NotEnrolled,
    RandomBytes,
}

#[test]
fn no_message_passes_without_the_matching_key() {
    const ROUNDS: usize = 3000;
    let mut rng = Rng(0x5EED_A55E_55ED_0172);
    let mut verifier = verifier(DEVICE);
    let mut last = [0u64; 3]; // trust root, owner phone, test module
    let index = |role: Role| match role {
        Role::TrustRoot => Some(0),
        Role::OwnerPhone => Some(1),
        Role::Module(id) if id == TEST_MODULE => Some(2),
        Role::Module(_) => None,
    };
    let mut passed = 0;
    let mut now = 0u64;
    for round in 0..ROUNDS {
        now += rng.below(3);
        let named = random_role(&mut rng);
        let approval = random_approval(&mut rng);
        let target = if rng.below(10) == 0 {
            DeviceId(rng.next() as u32)
        } else {
            DEVICE
        };
        let message = Message {
            target,
            signer: named,
            // Mostly a fresh counter, sometimes a replay or an old one.
            counter: match index(named) {
                Some(i) if rng.below(4) != 0 => last[i] + 1 + rng.below(3),
                Some(i) => rng.below(last[i] + 2),
                None => rng.below(ROUNDS as u64),
            },
            expires_at: now + rng.below(5),
            approval,
        };
        let bytes = message.encode();
        let enrolled = [Role::TrustRoot, Role::OwnerPhone, Role::Module(TEST_MODULE)];
        let signer = match rng.below(5) {
            // Mostly the named role's own key, so many pass.
            0 | 1 => Signer::Key(named),
            2 => Signer::Key(enrolled[rng.below(3) as usize]),
            3 => Signer::NotEnrolled,
            _ => Signer::RandomBytes,
        };
        let signature = match signer {
            Signer::Key(role) if index(role).is_some() => sign(&seed_for(role), &bytes),
            Signer::Key(_) | Signer::NotEnrolled => sign(&NOT_ENROLLED_SEED, &bytes),
            Signer::RandomBytes => {
                let mut sig = [0; 64];
                sig[..32].copy_from_slice(&rng.bytes32());
                sig[32..].copy_from_slice(&rng.bytes32());
                Signature(sig)
            }
        };

        let result = verifier.verify::<Approval>(&bytes, &signature, now);

        let right_key = signer == Signer::Key(named) && index(named).is_some();
        let should_pass = right_key
            && target == DEVICE
            && may_approve(named, &approval)
            && now < message.expires_at
            && message.counter > last[index(named).unwrap()];
        match result {
            Ok(approved) => {
                assert!(should_pass, "round {round}: {message:?} by {signer:?}");
                assert_eq!(*approved.approval(), approval);
                last[index(named).unwrap()] = message.counter;
                passed += 1;
            }
            Err(refusal) => {
                assert!(!should_pass, "round {round}: {refusal:?} for {message:?}");
                // A wrong key is caught by the signature, not by luck.
                let checked = target == DEVICE && may_approve(named, &approval);
                if !right_key && checked && index(named).is_some() {
                    assert_eq!(refusal, Refusal::BadSignature, "round {round}");
                }
            }
        }
        for role in [Role::TrustRoot, Role::OwnerPhone, Role::Module(TEST_MODULE)] {
            assert_eq!(
                verifier.last_counter(role),
                Some(last[index(role).unwrap()])
            );
        }
    }
    assert!(
        passed > ROUNDS / 20,
        "only {passed} passed: the test is too weak"
    );
}

#[test]
fn random_bytes_never_pass() {
    let mut rng = Rng(0x0BAD_F00D_DEAD_BEEF);
    let mut verifier = verifier(DEVICE);
    for _ in 0..3000 {
        let mut bytes = [0u8; MESSAGE_LEN];
        for byte in &mut bytes {
            *byte = rng.next() as u8;
        }
        // Make a fraction structurally valid so the signature check runs.
        if rng.below(2) == 0 {
            bytes = Message::decode(&bytes).map_or_else(
                |_| {
                    let mut m =
                        common::message(Role::TrustRoot, rng.next(), random_approval(&mut rng));
                    m.expires_at = u64::MAX;
                    m.encode()
                },
                |m| m.encode(),
            );
        }
        let mut sig = [0; 64];
        sig[..32].copy_from_slice(&rng.bytes32());
        sig[32..].copy_from_slice(&rng.bytes32());
        assert!(verifier
            .verify::<Approval>(&bytes, &Signature(sig), 0)
            .is_err());
    }
}
