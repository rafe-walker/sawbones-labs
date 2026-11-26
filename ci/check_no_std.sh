#!/usr/bin/env bash
# Build every crate listed in ci/no_std_crates.txt for the microcontroller target.
set -euo pipefail
cd "$(dirname "$0")/.."
target="thumbv8m.main-none-eabihf"
rustup target add "$target" >/dev/null 2>&1 || true
status=0
while IFS= read -r line || [ -n "$line" ]; do
  line="${line%%#*}"
  [ -z "${line//[[:space:]]/}" ] && continue
  read -r -a words <<<"$line"
  crate="${words[0]}"
  flags=("${words[@]:1}")
  echo "::group::no_std build: $crate ${flags[*]:-}"
  if ! cargo build --locked --target "$target" -p "$crate" ${flags[@]+"${flags[@]}"}; then
    echo "::error::$crate failed to build for $target"
    status=1
  fi
  echo "::endgroup::"
done < ci/no_std_crates.txt
exit "$status"
