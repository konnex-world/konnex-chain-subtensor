#!/usr/bin/env bash
set -euo pipefail

docker compose -f docker-compose.build-dev.yml run --rm compile \
  bash -lc "export CARGO_TARGET_DIR=/build/target && ./pallets/konnex-escrow/scripts/benchmark.sh"
