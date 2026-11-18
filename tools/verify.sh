#!/usr/bin/env bash
# Check the proofs in src/fit.rs with Verus (docs/VERIFICATION.md). Not part of the PR gate:
# it needs the Verus release named below, which an ordinary build does not.
#
#   VERUS=/path/to/verus ./tools/verify.sh     (default: `verus` on PATH, then ~/.local/share/verus)
#
# Verus is a release binary with its own Rust toolchain (it says which `rustup install` it needs).
# The proofs were last checked with Verus 0.2026.11.02.8bf68ee (toolchain 1.99.1).
set -euo pipefail
cd "$(dirname "$0")/.."
verus="${VERUS:-}"
if [ -z "$verus" ]; then
  verus="$(command -v verus || true)"
fi
if [ -z "$verus" ]; then
  verus="$(ls "$HOME"/.local/share/verus/*/verus 2>/dev/null | head -n 1 || true)"
fi
if [ -z "$verus" ] || [ ! -x "$verus" ]; then
  echo "verify.sh: Verus not found; download a release from https://github.com/verus-lang/verus/releases and set VERUS=/path/to/verus" >&2
  exit 2
fi
"$verus" --version
# The file is a crate of its own for Verus; the same file is `mod fit` in the ordinary build.
"$verus" --crate-type lib src/fit.rs
