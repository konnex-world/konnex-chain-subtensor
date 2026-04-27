#!/usr/bin/env bash
set -euo pipefail

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/workspaces/knx-subtensor/target}"

cargo build -p node-subtensor --release --features runtime-benchmarks
"${CARGO_TARGET_DIR}/release/node-subtensor" benchmark pallet \
  --chain=local \
  --pallet=pallet_konnex_escrow \
  --extrinsic="*" \
  --steps=50 \
  --repeat=20 \
  --output=pallets/konnex-escrow/src/weights.rs \
  --template=./.maintain/frame-weight-template.hbs
