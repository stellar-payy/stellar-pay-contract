# Integration guide for stellar-pay-backend

This document is the exact interface `stellar-pay-backend`'s `crates/onchain`
needs to implement its `OnChainRecorder`. Nothing here is implemented in
this repo beyond the contract itself; this is documentation for the sibling
backend repo's future Soroban RPC client work.

## Configuration

The backend needs three values, sourced from `.env`:

- `STELLAR_CONTRACT_ID` - the `C...` contract id from `scripts/deploy_testnet.sh`
  or the manual deploy in `docs/deploy.md`.
- `STELLAR_SOROBAN_RPC_URL` - the Soroban RPC endpoint for the target network
  (for example `https://soroban-testnet.stellar.org`).
- `STELLAR_ONCHAIN_WRITER_SECRET` - the secret key of the account initialized
  as this contract's `admin`. This must be a distinct keypair from
  `STELLAR_PAYMENT_ADDRESS`: the payment-receiving address stays low-activity,
  while the on-chain writer is a hot signer that submits a transaction on
  every confirmed payment and therefore has a narrower blast radius if
  compromised.

## Function signature

```rust
pub fn record_payment(
    env: Env,
    caller: Address,
    payment_id: BytesN<16>,
    stellar_tx_hash: BytesN<32>,
    amount: i128,
    destination: Address,
    reference: String,
) -> Result<(), PaymentRegistryError>;
```

- `caller` must be the account whose secret key signs the transaction (the
  `STELLAR_ONCHAIN_WRITER_SECRET` account), and it must equal the contract's
  stored admin, or the call returns `Unauthorized`.
- A call with a `payment_id` that was already recorded returns
  `AlreadyRecorded`. Treat that as equivalent to success: it means an
  earlier attempt landed even though the backend didn't observe a
  confirmation (for example after a network timeout), so retries are safe.
- `amount <= 0` returns `InvalidAmount`.

`get_payment(payment_id: BytesN<16>) -> Result<PaymentRecord, PaymentRegistryError>`
and `bump_ttl(payment_id: BytesN<16>) -> Result<(), PaymentRegistryError>` are
also available; `bump_ttl` takes no auth and can be called by anything that
wants to keep an old record from being archived off the live ledger.

## Argument encoding from `Payment`

| Contract argument   | Backend source                                   | Encoding                                              |
|---------------------|---------------------------------------------------|--------------------------------------------------------|
| `payment_id`         | `Payment.id: Uuid`                                | `Uuid::as_bytes()` gives `[u8; 16]` directly, no padding or hashing - construct `BytesN::<16>` from it as-is. |
| `stellar_tx_hash`    | the Horizon-confirmed transaction hash (hex string) | hex-decode to `[u8; 32]`, then `BytesN::<32>::from_array`. |
| `amount`             | `Payment.amount: String` (decimal XLM, e.g. `"10.5000000"`) | convert to stroops with the algorithm below. |
| `destination`        | `Payment.destination` (a Stellar `G...` address)  | parse into a Soroban `Address` via the RPC client's account-id conversion. |
| `reference`          | `Payment.reference` or equivalent free-text field | pass through as a Soroban `String`. |

## Amount-to-stroops conversion (exact algorithm)

The backend performs this conversion once, before calling the contract. The
contract itself only re-validates `amount > 0` as defense-in-depth; it does
not re-derive stroops from a decimal string.

1. Validate the decimal string matches `^\d+\.\d{1,7}$`. Reject anything
   else (missing decimal point, more than 7 fractional digits, negative
   sign, exponents, etc).
2. Split the string on `.` into `integer_part` and `fractional_part`.
3. Right-pad `fractional_part` with trailing zeros until it is exactly 7
   digits long.
4. `stroops = integer_part.parse::<i128>() * 10_000_000 + fractional_part.parse::<i128>()`.

Example: `"10.5"` -> integer `"10"`, fractional `"5"` padded to `"5000000"`
-> `10 * 10_000_000 + 5_000_000 = 105_000_000` stroops.

This mirrors the standard Stellar/Soroban convention of 7 decimal places of
precision (1 XLM = 10,000,000 stroops).

## Signing and submission flow

The backend calls this contract through Soroban RPC, not through
`stellar-cli`, in production. The flow is:

1. Build the `record_payment` invocation as a Soroban host function call
   inside a transaction envelope.
2. Call Soroban RPC `simulateTransaction` to obtain the required resource
   footprint and fees. Do not guess these values.
3. Apply the simulation's resource data to the transaction, then sign it
   with `STELLAR_ONCHAIN_WRITER_SECRET`.
4. Submit via Soroban RPC `sendTransaction`.
5. Poll `getTransaction` (with backoff) until the status is no longer
   `PENDING`. `SUCCESS` means the record landed; `FAILED` should be
   inspected for the specific `PaymentRegistryError` (an `AlreadyRecorded`
   result there is not a failure from the backend's point of view - see
   above).

Which Rust Soroban RPC client library the backend uses for this flow is an
open decision on the backend side, deliberately not resolved in this repo -
see the "Cross-repo findings" section of the scaffold plan and
`stellar-pay-backend`'s `crates/onchain` for the current `NullOnChainRecorder`
placeholder this will replace.

## Best-effort semantics

This contract call happens after a payment is already `Confirmed` via
Horizon. Horizon verification is authoritative; the on-chain record is an
independently-auditable copy of an already-settled fact, not a gate on
payment completion. A failed or slow `record_payment` call must never block
or roll back the backend's own confirmation flow - log and move on.
