set shell := ["bash", "-euo", "pipefail", "-c"]

setup:
    scripts/bootstrap-dev.sh --apply

init-repo:
    scripts/init-repo.sh --apply

verify:
    scripts/check-fast-feedback.sh

fast-feedback:
    scripts/check-fast-feedback.sh

merge-gate:
    scripts/check-merge-result.sh

shell-quality:
    scripts/check-shell-quality.sh --all

workflow-lint:
    scripts/check-workflows.sh

rust-hygiene:
    scripts/check-rust-hygiene.sh --advisory --check all
