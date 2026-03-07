#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  TMP_REPO="$(mktemp -d)"
  cd "$TMP_REPO"
  git init -q
  git config user.name "Test User"
  git config user.email "test@example.com"
  mkdir -p .githooks scripts
  cp "$ROOT/.githooks/pre-commit" .githooks/pre-commit
  chmod +x .githooks/pre-commit

  cat > scripts/check-changelog-staged.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER
  cat > scripts/check-rust-policy.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER
  cat > scripts/check-rust-tests.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER
  cat > scripts/check-sync-manifest.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER
  cat > scripts/doc-lint.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER
  cat > scripts/check-doc-terms.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER
  cat > scripts/check-tasks-registry.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER
  cat > scripts/check-tasks-sync.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER

  chmod +x scripts/check-changelog-staged.sh scripts/check-rust-policy.sh scripts/check-rust-tests.sh \
    scripts/check-sync-manifest.sh scripts/doc-lint.sh scripts/check-doc-terms.sh \
    scripts/check-tasks-registry.sh scripts/check-tasks-sync.sh
}

teardown() {
  rm -rf "$TMP_REPO"
}

@test "pre-commit auto-refreshes when marker is stale" {
  cat > scripts/check-fast-feedback-marker.sh <<'INNER'
#!/usr/bin/env bash
if [[ -f .marker_ok ]]; then
  echo "fast-feedback-marker: OK"
  exit 0
fi
echo "fast-feedback-marker: stale"
exit 1
INNER
  cat > scripts/check-fast-feedback.sh <<'INNER'
#!/usr/bin/env bash
touch .marker_ok
exit 0
INNER
  chmod +x scripts/check-fast-feedback-marker.sh scripts/check-fast-feedback.sh

  run .githooks/pre-commit
  [ "$status" -eq 0 ]
}
