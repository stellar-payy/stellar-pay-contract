#!/usr/bin/env bash
set -euo pipefail

# === Deploy the payment-registry wasm to a network (default testnet)
#
# Required env vars:
#   SOURCE_ACCOUNT - a funded stellar-cli identity name (see docs/deploy.md)
# Optional env vars:
#   NETWORK - defaults to "testnet"

cd "$(dirname "${BASH_SOURCE[0]}")/.."

NETWORK="${NETWORK:-testnet}"
SOURCE_ACCOUNT="${SOURCE_ACCOUNT:?SOURCE_ACCOUNT env var must be set to a funded stellar-cli identity name}"

WASM_PATH="target/wasm32-unknown-unknown/release/payment_registry.wasm"

if [ ! -f "$WASM_PATH" ]; then
    echo "Wasm not found at $WASM_PATH, run scripts/build.sh first" >&2
    exit 1
fi

CONTRACT_ID=$(stellar contract deploy \
    --wasm "$WASM_PATH" \
    --source "$SOURCE_ACCOUNT" \
    --network "$NETWORK")

echo "Deployed contract id: $CONTRACT_ID"
echo "Export it before running scripts/init_testnet.sh, e.g.:"
echo "  export CONTRACT_ID=$CONTRACT_ID"
