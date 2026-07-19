#!/usr/bin/env bash
# 04-verify gate — DISPATCHER. The deterministic edge: the change ships only if this
# goes green. Run with the target project's worktree as the current directory:
#   (cd <worktree> && /path/to/stages/04-verify/gate.sh)
#
# The quality bar is PROJECT knowledge, not ALGORITHM knowledge (ADR-0007). So this
# dispatcher prefers the project's OWN gate and keeps the champion generic:
#
#   .da/gate present & executable  -> run it (the project owns its bar)
#   .da/gate present, not +x       -> FAIL closed (never a silent fallback)
#   no .da/gate                    -> run default-gate.sh (generic host chain)
#
# Whichever sub-gate runs, it only produces an exit code; THIS file renders the single
# canonical `GATE GREEN` / `GATE RED` verdict, so the gate report is uniform across
# projects. A provenance header records which gate ran and its sha256 — an arm cannot
# silently weaken its own bar without the change showing in result.patch.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
project_gate=".da/gate"

if [[ -x "$project_gate" ]]; then
  echo "=== gate: project $project_gate ==="
  echo "  cwd:    $(pwd)"
  echo "  sha256: $(sha256sum "$project_gate" | cut -d' ' -f1)"
  echo
  "$project_gate"
  rc=$?
elif [[ -e "$project_gate" ]]; then
  echo "=== gate: project $project_gate ===" >&2
  echo "  FAIL: $project_gate exists but is not executable — refusing to fall back" >&2
  echo "        (a non-executable project gate is a setup error, not a reason to skip it)" >&2
  rc=1
else
  echo "=== gate: default host chain (project ships no $project_gate) ==="
  echo "  runner: $here/default-gate.sh"
  echo
  "$here/default-gate.sh"
  rc=$?
fi

echo
if [[ "$rc" -eq 0 ]]; then
  echo "GATE GREEN"
else
  echo "GATE RED — do not ship" >&2
fi
exit "$rc"
