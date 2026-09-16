#!/usr/bin/env bash
set -euo pipefail

# === Build the payment-registry contract to wasm

cd "$(dirname "${BASH_SOURCE[0]}")/.."

stellar contract build

echo "Build output: target/wasm32-unknown-unknown/release/payment_registry.wasm"
