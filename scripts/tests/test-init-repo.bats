#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  TMP_REPO="$(mktemp -d)"
  cd "$TMP_REPO"
  git init -q
  git config user.name "Test User"
  git config user.email "test@example.com"

  mkdir -p scripts docs/templates
  cp "$ROOT/scripts/init-repo.sh" scripts/init-repo.sh
  chmod +x scripts/init-repo.sh
  cp "$ROOT/docs/templates/README.template.md" docs/templates/README.template.md
  cp "$ROOT/docs/templates/AGENTS.template.md" docs/templates/AGENTS.template.md
}

teardown() {
  rm -rf "$TMP_REPO"
}

@test "init-repo requires explicit mode" {
  run scripts/init-repo.sh
  [ "$status" -eq 2 ]
  [[ "$output" == *"explicit mode required"* ]]
}

@test "init-repo apply creates starter files from templates" {
  run scripts/init-repo.sh --apply --project alpha
  [ "$status" -eq 0 ]
  [ -f README.md ]
  [ -f AGENTS.md ]

  run rg '^# alpha$' README.md
  [ "$status" -eq 0 ]

  run rg '^# AGENTS$' AGENTS.md
  [ "$status" -eq 0 ]
}

@test "init-repo check fails when starter files are missing" {
  run scripts/init-repo.sh --check
  [ "$status" -eq 1 ]
  [[ "$output" == *"missing README.md"* ]]
}

@test "init-repo apply does not overwrite without force" {
  cat > README.md <<'EOF'
# custom
EOF
  cat > AGENTS.md <<'EOF'
# custom-agents
EOF

  run scripts/init-repo.sh --apply --project alpha
  [ "$status" -eq 0 ]

  run rg '^# custom$' README.md
  [ "$status" -eq 0 ]
  run rg '^# custom-agents$' AGENTS.md
  [ "$status" -eq 0 ]
}

@test "init-repo apply overwrites with force" {
  cat > README.md <<'EOF'
# custom
EOF
  cat > AGENTS.md <<'EOF'
# custom-agents
EOF

  run scripts/init-repo.sh --apply --force --project beta
  [ "$status" -eq 0 ]

  run rg '^# beta$' README.md
  [ "$status" -eq 0 ]
  run rg '^# AGENTS$' AGENTS.md
  [ "$status" -eq 0 ]
}

@test "init-repo apply reads project name from config file" {
  cat > project.init.yaml <<'EOF'
version: 1
project:
  name: config-project
EOF

  run scripts/init-repo.sh --apply --config project.init.yaml
  [ "$status" -eq 0 ]

  run rg '^# config-project$' README.md
  [ "$status" -eq 0 ]
}

@test "init-repo cli project flag overrides config project name" {
  cat > project.init.yaml <<'EOF'
version: 1
project:
  name: config-project
EOF

  run scripts/init-repo.sh --apply --config project.init.yaml --project cli-project
  [ "$status" -eq 0 ]

  run rg '^# cli-project$' README.md
  [ "$status" -eq 0 ]
}

@test "init-repo dry-run does not write starter files" {
  run scripts/init-repo.sh --dry-run --project dry-run-project
  [ "$status" -eq 0 ]
  [[ "$output" == *"would write README.md"* ]]
  [[ "$output" == *"would write AGENTS.md"* ]]

  [ ! -f README.md ]
  [ ! -f AGENTS.md ]
}

@test "init-repo interactive writes config with prompted project name" {
  run bash -c "printf 'interactive-project\n' | scripts/init-repo.sh --interactive --config project.init.yaml"
  [ "$status" -eq 0 ]
  [ -f project.init.yaml ]

  run rg '^  name: interactive-project$' project.init.yaml
  [ "$status" -eq 0 ]
}
