#!/usr/bin/env bash
set -euo pipefail

# Nightly regression runner for the autonomous agent layer.
# It exits non-zero if no candidate passes autonomous gates.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

mkdir -p artifacts

if [[ ! -f artifacts/baseline.json ]]; then
  echo "Baseline not found at artifacts/baseline.json; generating one..."
  cargo run -p articos-agentic -- \
    --mode single \
    --scheduler fifo \
    --workload mixed \
    --slots 4 \
    --tasks 80 \
    --seed 42 \
    --output artifacts/baseline.json
fi

cargo run -p articos-agentic -- \
  --mode nightly \
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

echo "Nightly regression completed successfully."
