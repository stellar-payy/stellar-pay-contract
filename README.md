# stellar-pay-contract

A Soroban smart contract that acts as an **on-chain payment registry** for
the `stellar-pay` project - an immutable, independently-auditable record of
XLM payments that have already been verified off-chain.

## What this is, and what it is not

`stellar-pay`'s core v0.1 payment flow is classic Stellar payments: native
XLM, detected and verified via Horizon, not through a smart contract. That
flow, implemented in the sibling `stellar-pay-backend` repo, stays
authoritative for whether a payment happened.

This contract is a separate, optional piece. After the backend confirms a
payment as `Confirmed` via Horizon, it best-effort calls this contract's
`record_payment` to write a permanent, on-chain audit entry. If that call
fails or is slow, the payment is still considered confirmed - this contract
never gates or verifies payments, it only records already-verified
outcomes.

## Relationship to `stellar-pay-backend`

- `stellar-pay-backend`'s `crates/onchain` crate defines an
  `OnChainRecorder` trait and currently ships a `NullOnChainRecorder` (logs
  and returns `Ok(())`). Wiring it to call this contract for real over
  Soroban RPC is documented in `docs/integration.md` and is explicitly the
  next step once a Soroban RPC client approach is chosen on the backend
  side - it is not implemented in this repo.
- The two repos are independent git repositories with independent Rust
  toolchains (this one pins `edition = "2021"` for `wasm32`/`stellar-cli`
  compatibility, unrelated to the backend's edition choice).

## Architecture

This contract has exactly one write path and it is always reached from the
backend's reconciliation worker, never directly from a merchant or payer:

```
  stellar-pay-backend                         stellar-payy-contract (this repo)
  --------------------                        ------------------------------
  ReconciliationWorker
    detects payment as Confirmed
    via Horizon (authoritative)
           |
           v
  OnChainRecorder::record_payment  ---Soroban RPC--->  PaymentRegistry::record_payment
    (best-effort, logged on error)                       caller.require_auth()
           |                                             caller == admin?  else Unauthorized
           | (never blocks/rolls back                    payment_id already recorded?
           |  the backend's own confirmation)                else AlreadyRecorded
           v                                             write PaymentRecord, bump TTL
  payment stays Confirmed                                 publish PAY_REC event
  regardless of this call's outcome
```

Storage is two keyed spaces in the contract's persistent storage:

```
DataKey::Admin              -> Address                (instance storage, set once by init)
DataKey::Payment(payment_id) -> PaymentRecord          (persistent storage, one entry per payment)
```

`record_payment`'s actual control flow, including the auth/admin split and
the TTL bump on write:

```rust
pub fn record_payment(
    env: Env,
    caller: Address,
    payment_id: BytesN<16>,
    stellar_tx_hash: BytesN<32>,
    amount: i128,
    destination: Address,
    reference: String,
) -> Result<(), PaymentRegistryError> {
    caller.require_auth();

    let admin: Address = match env.storage().instance().get(&DataKey::Admin) {
        Some(admin) => admin,
        None => return Err(PaymentRegistryError::NotInitialized),
    };

    if caller != admin {
        return Err(PaymentRegistryError::Unauthorized);
    }

    if amount <= 0 {
        return Err(PaymentRegistryError::InvalidAmount);
    }

    let key = DataKey::Payment(payment_id.clone());
    if env.storage().persistent().has(&key) {
        return Err(PaymentRegistryError::AlreadyRecorded);
    }

    let record = PaymentRecord {
        stellar_tx_hash,
        amount,
        destination,
        reference,
        recorded_at: env.ledger().timestamp(),
    };

    env.storage().persistent().set(&key, &record);
    env.storage()
        .persistent()
        .extend_ttl(&key, PAYMENT_LIFETIME_THRESHOLD, PAYMENT_BUMP_AMOUNT);

    event::publish_payment_recorded(&env, payment_id, &record);
    return Ok(());
}
```

`caller.require_auth()` proves signature control over `caller`; whether that
caller is actually the admin is a separate, catchable business rule
(`Unauthorized`), never folded into the auth check itself. Payment records
are bumped on a ~365 day TTL cycle (vs. ~30 days for the admin-holding
contract instance) since they are the permanent audit trail this contract
exists for; `bump_ttl` lets anyone renew a record indefinitely before it
would otherwise become eligible for archival off the live ledger.

## Repository layout

```
stellar-pay-contract/
├── Cargo.toml                       workspace manifest + release profile
├── contracts/
│   └── payment-registry/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                module wiring, re-exports
│           ├── types.rs              DataKey, PaymentRecord
│           ├── error.rs              PaymentRegistryError
│           ├── event.rs              PAY_REC event
│           ├── contract.rs           #[contract] impl
│           └── test.rs               testutils-based tests
├── scripts/
│   ├── build.sh
│   ├── deploy_testnet.sh
│   └── init_testnet.sh
└── docs/
    ├── deploy.md                     manual deploy command sequence
    └── integration.md                backend integration interface
```

## Public interface

```rust
fn init(env: Env, admin: Address) -> Result<(), PaymentRegistryError>;

fn record_payment(
    env: Env,
    caller: Address,
    payment_id: BytesN<16>,
    stellar_tx_hash: BytesN<32>,
    amount: i128,
    destination: Address,
    reference: String,
) -> Result<(), PaymentRegistryError>;

fn get_payment(env: Env, payment_id: BytesN<16>) -> Result<PaymentRecord, PaymentRegistryError>;

fn bump_ttl(env: Env, payment_id: BytesN<16>) -> Result<(), PaymentRegistryError>;
```

- `payment_id` is `BytesN<16>` - the raw bytes of a UUID, used purely as a
  lookup key, not a secret. `stellar_tx_hash` is a real 32-byte SHA-256
  transaction hash.
- `init` is one-time; a second call returns `AlreadyInitialized`.
- `record_payment` requires `caller.require_auth()` (proves signature
  control) and separately checks `caller == admin` as a catchable business
  rule returning `Unauthorized` (not a panic). Duplicate `payment_id`
  returns `AlreadyRecorded`, so a backend retry after an ambiguous network
  failure is always safe - it either succeeds once or observes
  `AlreadyRecorded`.
- `get_payment` returns `NotFound` for an unrecorded id.
- `bump_ttl` is deliberately unauthenticated: extending the liveness of an
  already-public audit record is not a privileged action, and this
  contract's entire purpose is to be a durable, permanent audit trail.
- A `PAY_REC` event is published on every successful `record_payment`, with
  `payment_id` as the event topic.

See `docs/integration.md` for the exact field-encoding and amount-to-stroops
conversion the backend must use when calling this contract.

## Build

```bash
cargo build --workspace
```

Wasm target:

```bash
stellar contract build
# or: cargo build --target wasm32-unknown-unknown --release
```

## Test

```bash
cargo test --workspace
```

All tests run fully offline against `soroban-sdk`'s `testutils` - no
network or deployed contract required.

## Deploy

See `docs/deploy.md` for the full manual command sequence (identity
creation, testnet funding, build, deploy, init, sanity invoke), or use the
equivalent wrapper scripts:

```bash
./scripts/build.sh
NETWORK=testnet SOURCE_ACCOUNT=deployer ./scripts/deploy_testnet.sh
NETWORK=testnet SOURCE_ACCOUNT=deployer ADMIN_ADDRESS=<G...> CONTRACT_ID=<C...> ./scripts/init_testnet.sh
```

## Known limitations

- **Single fixed admin, no rotation.** v0.1 has exactly one admin address,
  set once at `init` and never changeable. Compromising or losing that
  key means deploying a new contract instance. Admin rotation is a
  known, deliberate gap, not an oversight.
- **Best-effort recording, not a payment gate.** This contract has no
  visibility into whether a payment actually happened - it trusts the
  caller (the backend, authenticated as admin) that a payment was already
  confirmed via Horizon. It is an audit trail, not a source of truth for
  payment status.
- **No admin-side query for "all payments."** `get_payment` is a
  point lookup by `payment_id`; there is no on-chain enumeration. Building
  a payment list means indexing the `PAY_REC` events off-chain.
