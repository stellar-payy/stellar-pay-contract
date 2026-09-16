#!/usr/bin/env bash
set -euo pipefail

# === Initialize a deployed payment-registry contract with its admin
#
# Required env vars:
#   SOURCE_ACCOUNT - a funded stellar-cli identity name (see docs/deploy.md)
#   ADMIN_ADDRESS  - the admin G... address to store in the contract
#   CONTRACT_ID    - the contract id returned by scripts/deploy_testnet.sh
# Optional env vars:
#   NETWORK - defaults to "testnet"

NETWORK="${NETWORK:-testnet}"
SOURCE_ACCOUNT="${SOURCE_ACCOUNT:?SOURCE_ACCOUNT env var must be set to a funded stellar-cli identity name}"
ADMIN_ADDRESS="${ADMIN_ADDRESS:?ADMIN_ADDRESS env var must be set to the admin G... address}"
CONTRACT_ID="${CONTRACT_ID:?CONTRACT_ID env var must be set to the deployed contract id}"

stellar contract invoke \
    --id "$CONTRACT_ID" \
    --source "$SOURCE_ACCOUNT" \
    --network "$NETWORK" \
    -- \
    init \
    --admin "$ADMIN_ADDRESS"

echo "Initialized contract $CONTRACT_ID with admin $ADMIN_ADDRESS"
