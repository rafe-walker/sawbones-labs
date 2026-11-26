#!/usr/bin/env bash
# regression_208_no_build_artifacts: fail if any tracked file is build output
# (Python caches, Rust target/, object files). Issue #208.
set -euo pipefail
bad="$(git ls-files | grep -E '(^|/)(__pycache__|target)/|\.(pyc|pyo|o|rlib|rmeta)$' || true)"
if [ -n "${bad}" ]; then
  echo "::error::Build output is committed (#208):"
  echo "${bad}"
  exit 1
fi
echo "No build output is committed."
