#!/usr/bin/env bash
# Default gate — the generic end-of-run chain for a Rust change, used when the target
# project ships no gate of its own. The dispatcher (gate.sh) runs this as a fallback.
#
# Reuses the ~/software-factory/gate.sh chain, minus that project's own demo witnesses:
# build+test, clippy (-D warnings), hex-lint (crate-role edges), effect-audit --strict
# (functional-core purity). Fail-closed. A missing tool is a loud SKIP, never a silent
# pass. Exits 0 when every step passed, 1 otherwise — it prints NO verdict line; the
# dispatcher renders the canonical GATE GREEN / GATE RED so every gate is uniform.
#
# Run with the target project's worktree as the current directory:
#   (cd <worktree> && /path/to/stages/04-verify/default-gate.sh)
set -uo pipefail

fail=0
step() { # step <label> <cmd...>
  echo "=== $1 ==="
  if "${@:2}"; then echo "  OK: $1"; else echo "  FAIL: $1" >&2; fail=1; fi
  echo
}

step "tests"  cargo test --workspace --quiet
step "clippy" cargo clippy --workspace --all-targets --quiet -- -D warnings

if command -v hex-lint >/dev/null 2>&1; then
  step "hex-lint (crate-role edges)" hex-lint
else
  echo "=== hex-lint ==="; echo "  SKIP: hex-lint not on PATH (cargo install from ~/code/tools/hex-lint)" >&2; echo
fi

if command -v effect-audit >/dev/null 2>&1; then
  step "effect-audit (functional-core purity)" effect-audit --strict --require-domain
else
  echo "=== effect-audit ==="; echo "  SKIP: effect-audit not on PATH (cargo install from ~/code/tools/effect-audit)" >&2; echo
fi

exit "$fail"
