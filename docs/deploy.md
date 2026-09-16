# Deploying payment-registry

This is the authoritative manual command sequence. `scripts/*.sh` wrap the
same commands for convenience, but everything below works standalone with
just `stellar-cli` installed, no scripts required.

## 1. Install stellar-cli

```bash
cargo install --locked stellar-cli
stellar --version
```

## 2. Create a deploy identity

```bash
stellar keys generate deployer --network testnet
stellar keys address deployer
```

This creates a local identity named `deployer` and prints its public
address (`G...`).

## 3. Fund the identity on testnet

```bash
stellar keys fund deployer --network testnet
```

This calls testnet friendbot. Confirm the account exists:

```bash
stellar keys address deployer
curl "https://horizon-testnet.stellar.org/accounts/$(stellar keys address deployer)"
```

## 4. Create (or reuse) an admin identity

The admin is the account allowed to call `record_payment`. In production
this is the backend's dedicated on-chain writer key, not the payment
receiving address (see `docs/integration.md` and the root `README.md` for
why they are kept separate).

```bash
stellar keys generate admin --network testnet
stellar keys fund admin --network testnet
stellar keys address admin
```

## 5. Build the contract

```bash
cd "stellar-pay-contract"
stellar contract build
```

Wasm output lands at:

```
target/wasm32-unknown-unknown/release/payment_registry.wasm
```

## 6. Deploy

```bash
stellar contract deploy \
    --wasm target/wasm32-unknown-unknown/release/payment_registry.wasm \
    --source deployer \
    --network testnet
```

This prints the deployed contract id (`C...`). Export it for the next
steps:

```bash
export CONTRACT_ID=<the printed contract id>
```

## 7. Initialize

```bash
stellar contract invoke \
    --id "$CONTRACT_ID" \
    --source deployer \
    --network testnet \
    -- \
    init \
    --admin "$(stellar keys address admin)"
```

`init` is one-time. A second call against the same contract id returns
`AlreadyInitialized`.

## 8. Sanity invoke

Record a test payment and read it back:

```bash
stellar contract invoke \
    --id "$CONTRACT_ID" \
    --source admin \
    --network testnet \
    -- \
    record_payment \
    --caller "$(stellar keys address admin)" \
    --payment_id 000102030405060708090a0b0c0d0e0f \
    --stellar_tx_hash 0001020304050607080910111213141516171819202122232425262728293031 \
    --amount 1000000000 \
    --destination "$(stellar keys address admin)" \
    --reference "sanity-check"

stellar contract invoke \
    --id "$CONTRACT_ID" \
    --source deployer \
    --network testnet \
    -- \
    get_payment \
    --payment_id 000102030405060708090a0b0c0d0e0f
```

`--stellar_tx_hash` must be exactly 64 hex characters (32 bytes).
`--payment_id` must be exactly 32 hex characters (16 bytes). A second
`record_payment` invocation with the same `--payment_id` returns
`AlreadyRecorded`, confirming duplicate-write protection.

## Notes

- `--source` here refers to the account whose signature pays and, for
  `record_payment`, satisfies `caller.require_auth()`. The `caller`
  argument and `--source` account must match, or Soroban will reject the
  transaction before the contract's own `Unauthorized` business check
  ever runs.
- `stellar-cli` simulates each invocation automatically before submission,
  so no manual fee estimation is required.
