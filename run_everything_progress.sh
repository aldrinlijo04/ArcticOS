#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

TOTAL_STEPS=8
CURRENT_STEP=0

log() {
  printf "[%s] %s\n" "$(date +"%H:%M:%S")" "$1"
}

progress() {
  local title="$1"
  CURRENT_STEP=$((CURRENT_STEP + 1))
  local percent=$((CURRENT_STEP * 100 / TOTAL_STEPS))
  printf "\n[%02d/%02d | %3d%%] %s\n" "$CURRENT_STEP" "$TOTAL_STEPS" "$percent" "$title"
}

run_step() {
  local title="$1"
  shift
  progress "$title"
  "$@"
  log "Done: $title"
}

mkdir -p artifacts

run_step "Preflight: verify cargo toolchain" cargo --version

run_step "Build all workspace crates" cargo build --workspace

run_step "Run scheduler + agentic test suites" cargo test -p articos-agentic -p articos-scheduler

run_step "Generate baseline artifact" \
  cargo run -p articos-agentic -- \
    --mode single \
    --scheduler fifo \
    --workload mixed \
    --slots 4 \
    --tasks 80 \
    --seed 42 \
    --output artifacts/baseline.json

run_step "Run autonomous sweep with gates and reports" \
  cargo run -p articos-agentic -- \
    --mode autonomous \
    --tasks 80 \
    --schedulers fifo,priority \
    --workloads uniform,mixed,bursty,priority-flood \
    --slots-list 2,4,8 \
    --seeds 41,42,43 \
    --baseline artifacts/baseline.json \
    --history-db artifacts/agentic_runs.db \
    --output artifacts/best_candidate.json \
    --report-json artifacts/autonomous_report.json \
    --report-md artifacts/autonomous_report.md

run_step "Run nightly regression check" bash agentic/nightly_regression.sh

run_step "Quick artifact health check" ls -lh \
  artifacts/baseline.json \
  artifacts/best_candidate.json \
  artifacts/autonomous_report.json \
  artifacts/autonomous_report.md \
  artifacts/agentic_runs.db

run_step "Print recommendation snapshot" grep -E 'recommendation|best_run_id|total_runs|passed_runs|failed_runs' artifacts/autonomous_report.json

printf "\n✅ All steps completed successfully.\n"
printf "Artifacts:\n"
printf "  - artifacts/baseline.json\n"
printf "  - artifacts/best_candidate.json\n"
printf "  - artifacts/autonomous_report.json\n"
printf "  - artifacts/autonomous_report.md\n"
printf "  - artifacts/agentic_runs.db\n"
