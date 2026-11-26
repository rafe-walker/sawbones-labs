#!/usr/bin/env bash
# #172: a default build contains no private key.
#
# Every fixed test seed in crates/sawbones-approvals/src/test_keys.rs starts
# with one marker. Build the firmware crates, and the trust-root firmware
# image (#114), for the firmware target without the test-keys feature and fail if the marker is in any artifact. First
# build with the feature and require the marker, so a scan that can never
# find anything cannot pass.
set -euo pipefail

TARGET=thumbv8m.main-none-eabihf
MARKER="SAWBONES-""TEST-KEY"
OUT=target/key-scan

cargo build --locked --target "$TARGET" --target-dir "$OUT/with" \
    -p sawbones-approvals --no-default-features --features test-keys
if ! grep -rqaF "$MARKER" "$OUT/with/$TARGET"; then
    echo "control failed: the test-keys build should contain the marker" >&2
    exit 1
fi

cargo build --locked --target "$TARGET" --target-dir "$OUT/without" \
    -p sawbones-approvals --no-default-features
cargo build --locked --target "$TARGET" --target-dir "$OUT/without" \
    -p sawbones-busguard --no-default-features --features approvals
# The trust-root firmware image, linked as it ships (#114).
ci/build_trust_root_fw.sh "$OUT/without"
if grep -rlaF "$MARKER" "$OUT/without/$TARGET"; then
    echo "a test private key is in a default build (see the files above)" >&2
    exit 1
fi
echo "no test key in the default firmware build"
