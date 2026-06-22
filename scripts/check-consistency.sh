#!/usr/bin/env bash
# Lightweight consistency check for hi workspace.
# Does NOT bind CI — run locally before commits.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

ERRORS=0

fail() {
  echo "CONSISTENCY: $1"
  ERRORS=$((ERRORS + 1))
}

echo "==> Checking required harness files..."
for f in AGENTS.md ARCHITECTURE.md docs/architecture/LAYERS.md docs/SECURITY.md; do
  [[ -f "$f" ]] || fail "Missing required file: $f (create or restore from harness-init)"
done

echo "==> Checking golden-principles..."
GP_COUNT=$(find docs/golden-principles -name '*.md' 2>/dev/null | wc -l | tr -d ' ')
[[ "$GP_COUNT" -ge 2 ]] || fail "Expected >= 2 golden-principles docs, found $GP_COUNT"

echo "==> Checking workspace crates match LAYERS.md..."
EXPECTED_CRATES="core ai tui gateway app"
for d in $EXPECTED_CRATES; do
  [[ -d "$d" ]] || fail "Missing crate directory: $d/ (update docs/architecture/LAYERS.md if intentional)"
done

echo "==> Checking AGENTS.md references existing paths..."
for ref in docs/architecture/LAYERS.md docs/golden-principles/IMPORTS.md docs/design-docs/core-beliefs.md; do
  [[ -e "$ref" ]] || fail "AGENTS.md links to missing path: $ref"
done

echo "==> Running architecture boundary tests..."
cargo test -p architecture-tests --quiet

echo "==> Running workspace unit tests..."
cargo test --workspace --quiet

echo "==> Running clippy..."
cargo clippy --workspace --quiet -- -D warnings

if [[ "$ERRORS" -gt 0 ]]; then
  echo ""
  echo "Consistency check failed with $ERRORS issue(s)."
  echo "Fix drift in AGENTS.md / ARCHITECTURE.md / docs/architecture/LAYERS.md as needed."
  exit 1
fi

echo ""
echo "All consistency checks passed."
