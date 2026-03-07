#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  TMP_WORK="$(mktemp -d)"
}

teardown() {
  rm -rf "$TMP_WORK"
}

@test "init-from-backbone requires --project when --dest is omitted" {
  run "$ROOT/scripts/init-from-backbone.sh"
  [ "$status" -eq 2 ]
  [[ "$output" == *"--project is required when --dest is omitted"* ]]
}

@test "init-from-backbone creates a new repo and initializes starter files" {
  dest="$TMP_WORK/new-project"

  run bash -lc "cd '$TMP_WORK' && '$ROOT/scripts/init-from-backbone.sh' --dest '$dest' --project omega"
  [ "$status" -eq 0 ]

  [ -d "$dest/.git" ]
  [ -f "$dest/README.md" ]
  [ -f "$dest/AGENTS.md" ]

  run rg '^# omega$' "$dest/README.md"
  [ "$status" -eq 0 ]
}

@test "init-from-backbone resets backbone-specific governance artifacts" {
  dest="$TMP_WORK/new-project-governance"

  run "$ROOT/scripts/init-from-backbone.sh" --dest "$dest" --project kappa
  [ "$status" -eq 0 ]

  run rg '^## Unreleased$' "$dest/CHANGELOG.md"
  [ "$status" -eq 0 ]

  run wc -l "$dest/docs/tasks/tasks.csv"
  [ "$status" -eq 0 ]
  [[ "$output" == *"1 "* ]]

  run bash -lc "find '$dest/docs/specs' '$dest/docs/plans' -type f -name '*.md' | wc -l"
  [ "$status" -eq 0 ]
  [[ "$output" == "0" ]]
}

@test "init-from-backbone uses project name as default destination when --dest is omitted" {
  run bash -lc "cd '$TMP_WORK' && '$ROOT/scripts/init-from-backbone.sh' --project zeta"
  [ "$status" -eq 0 ]

  [ -d "$TMP_WORK/zeta/.git" ]
  run rg '^# zeta$' "$TMP_WORK/zeta/README.md"
  [ "$status" -eq 0 ]
}

@test "init-from-backbone fails when destination directory already exists without --force" {
  dest="$TMP_WORK/existing"
  mkdir -p "$dest"

  run "$ROOT/scripts/init-from-backbone.sh" --dest "$dest" --project omega
  [ "$status" -eq 1 ]
  [[ "$output" == *"destination already exists"* ]]
  [[ "$output" == *"use --force"* ]]
}

@test "init-from-backbone does not copy local target artifacts" {
  dest="$TMP_WORK/no-target-copy"
  mkdir -p "$ROOT/target"
  printf 'scratch-build-output\n' >"$ROOT/target/init-from-backbone-fixture.txt"

  run "$ROOT/scripts/init-from-backbone.sh" --dest "$dest" --project sigma
  rm -f "$ROOT/target/init-from-backbone-fixture.txt"
  [ "$status" -eq 0 ]

  [ ! -e "$dest/target/init-from-backbone-fixture.txt" ]
}
