//! Messages and signers shared by the tests.
#![allow(dead_code)]

use sawbones_approvals::test_keys::{
    sign, verifier, MODULE_SEED, OWNER_PHONE_SEED, TEST_MODULE, TRUST_ROOT_SEED,
};
use sawbones_approvals::{
    Approval, BusGuardAllowList, DeviceId, Digest, GrantLease, InstallCoreRelease, Message,
    NativeInstall, OperatorAttestation, PluginId, PluginPermissionGrowth, Role, Signature,
    Verifier, MESSAGE_LEN,
};

/// The device the tests' verifier is for.
pub const DEVICE: DeviceId = DeviceId(7);

/// The verifier for [`DEVICE`] with the test keys enrolled.
pub fn test_verifier() -> Verifier {
    verifier(DEVICE)
}

/// The test seed for `role`.
pub fn seed_for(role: Role) -> [u8; 32] {
    match role {
        Role::TrustRoot => TRUST_ROOT_SEED,
        Role::OwnerPhone => OWNER_PHONE_SEED,
        Role::Module(id) if id == TEST_MODULE => MODULE_SEED,
        Role::Module(_) => panic!("no test seed for that module"),
    }
}

/// One of each kind of approval.
pub fn every_approval() -> [Approval; 6] {
    [
        Approval::InstallCoreRelease(InstallCoreRelease {
            release: Digest([0x11; 32]),
        }),
        Approval::GrantLease(GrantLease {
            port: 2,
            action: Digest([0x22; 32]),
        }),
        Approval::BusGuardAllowList(BusGuardAllowList {
            allow_list: Digest([0x33; 32]),
        }),
        Approval::PluginPermissionGrowth(PluginPermissionGrowth {
            plugin: PluginId([0x44; 32]),
            manifest: Digest([0x55; 32]),
        }),
        Approval::NativeInstall(NativeInstall {
            package: Digest([0x66; 32]),
        }),
        Approval::OperatorAttestation(OperatorAttestation {
            fact: Digest([0x77; 32]),
        }),
    ]
}

/// A lease approval for `port` and frame digest `action`.
pub fn lease(port: u8, action: u8) -> Approval {
    Approval::GrantLease(GrantLease {
        port,
        action: Digest([action; 32]),
    })
}

/// A message to [`DEVICE`] expiring at tick 1000.
pub fn message(signer: Role, counter: u64, approval: Approval) -> Message {
    Message {
        target: DEVICE,
        signer,
        counter,
        expires_at: 1000,
        approval,
    }
}

/// `message` encoded and signed by its own role's test key.
pub fn signed(message: &Message) -> ([u8; MESSAGE_LEN], Signature) {
    let bytes = message.encode();
    let signature = sign(&seed_for(message.signer), &bytes);
    (bytes, signature)
}
