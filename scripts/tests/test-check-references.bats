#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "references check script exists and supports changed and range modes" {
  run rg '^Usage:' "$ROOT/scripts/check-references.sh"
  [ "$status" -eq 0 ]

  run rg -- '--changed' "$ROOT/scripts/check-references.sh"
  [ "$status" -eq 0 ]

  run rg -- '--range' "$ROOT/scripts/check-references.sh"
  [ "$status" -eq 0 ]
}

@test "fast feedback and policy workflow invoke references check" {
  run rg 'scripts/check-references\.sh --changed' "$ROOT/scripts/check-fast-feedback.sh"
  [ "$status" -eq 0 ]

  run rg 'scripts/check-references\.sh --range "\$\{\{ steps\.range\.outputs\.range \}\}"' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]
}
