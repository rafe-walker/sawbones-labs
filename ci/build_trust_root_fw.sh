#!/usr/bin/env bash
# #114: build the trust-root firmware image for the RP2350 and check it.
#
# Usage: ci/build_trust_root_fw.sh [target-dir]
#
# Builds firmware/trust-root with the `firmware` profile, linked with
# cortex-m-rt's link.x and the crate's memory.x, then:
# - checks that the boot ROM's IMAGE_DEF block is in the first 4 KiB of flash
#   (without it the RP2350 will not boot the image);
# - checks that flash use is under the budget below;
# - writes the .bin next to the ELF;
# - with SIZE_REPORT=1, adds a size table to the GitHub job summary.
set -euo pipefail
cd "$(dirname "$0")/.."

TARGET=thumbv8m.main-none-eabihf
TARGET_DIR="${1:-target}"
CRATE=sawbones-trust-root-fw
# A placeholder ceiling, far above today's image: it catches a dependency
# that drags in a lot of code (a HAL, a formatter) before review does.
FLASH_BUDGET=$((64 * 1024))

rustup target add "$TARGET" >/dev/null 2>&1 || true

cargo rustc --locked --profile firmware --target "$TARGET" --target-dir "$TARGET_DIR" \
    -p "$CRATE" --bin "$CRATE" -- \
    -C link-arg=--nmagic \
    -C link-arg=-Tlink.x \
    -C "link-arg=-L$PWD/firmware/trust-root"

ELF="$TARGET_DIR/$TARGET/firmware/$CRATE"
BIN="$ELF.bin"

host="$(rustc -vV | sed -n 's/^host: //p')"
tools="$(rustc --print sysroot)/lib/rustlib/$host/bin"
if [ ! -x "$tools/llvm-size" ]; then
    rustup component add llvm-tools-preview >/dev/null
fi
"$tools/llvm-objcopy" -O binary "$ELF" "$BIN"

python3 -I - "$BIN" <<'EOF'
import struct, sys
data = open(sys.argv[1], "rb").read()
block = struct.pack("<5I", 0xffffded3, 0x10210142, 0x000001ff, 0, 0xab123579)
at = data.find(block, 0, 4096)
if at < 0 or at % 4:
    sys.exit("error: no IMAGE_DEF block in the first 4 KiB of flash; the RP2350 will not boot this image")
print(f"IMAGE_DEF block at flash offset {at:#x}")
EOF

read -r text data bss _ < <("$tools/llvm-size" --format=berkeley "$ELF" | sed -n 2p)
flash=$((text + data))
ram=$((data + bss))
echo "flash $flash bytes (text $text, data $data), RAM $ram bytes (data $data, bss $bss)"

if [ "${SIZE_REPORT:-0}" = 1 ] && [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    {
        echo "### Trust-root firmware size (#114)"
        echo
        echo "| Section | Bytes |"
        echo "| --- | ---: |"
        echo "| text (flash) | $text |"
        echo "| data (flash and RAM) | $data |"
        echo "| bss (RAM) | $bss |"
        echo "| **flash total** | **$flash** of $FLASH_BUDGET budget |"
        echo "| **static RAM** | **$ram** |"
        echo "| .bin image | $(wc -c <"$BIN") |"
        echo
        echo "Profile \`firmware\` (opt-level s, fat LTO, overflow checks on), target \`$TARGET\`."
    } >>"$GITHUB_STEP_SUMMARY"
fi

if [ "$flash" -gt "$FLASH_BUDGET" ]; then
    echo "error: the image uses $flash bytes of flash, over the $FLASH_BUDGET-byte budget" >&2
    exit 1
fi
echo "image: $ELF"
