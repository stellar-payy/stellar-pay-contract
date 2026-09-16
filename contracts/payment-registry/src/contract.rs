use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, String};

use crate::error::PaymentRegistryError;
use crate::event;
use crate::types::{DataKey, PaymentRecord};

// === Storage TTL Constants

// Approximate ledger close time is 5 seconds, so ~17280 ledgers per day.
// The contract's instance (holding the admin) is bumped on a routine
// 30 day cycle. Payment records are the permanent audit trail this
// contract exists for, so they are bumped on a much longer, ~1 year
// cycle, and bump_ttl lets anyone renew a record indefinitely before it
// would otherwise be eligible for archival.
const DAY_IN_LEDGERS: u32 = 17280;
const INSTANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;
const PAYMENT_BUMP_AMOUNT: u32 = 365 * DAY_IN_LEDGERS;
const PAYMENT_LIFETIME_THRESHOLD: u32 = PAYMENT_BUMP_AMOUNT - DAY_IN_LEDGERS;

#[contract]
pub struct PaymentRegistry;

#[contractimpl]
impl PaymentRegistry {
    // One-time setup. Rejects a second call so the admin cannot be
    // silently replaced after deployment.
    pub fn init(env: Env, admin: Address) -> Result<(), PaymentRegistryError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(PaymentRegistryError::AlreadyInitialized);
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        return Ok(());
    }

    // Writes an immutable record of an already-verified off-chain payment.
    // caller.require_auth() proves signature control over caller; whether
    // that caller is actually the admin is a separate, catchable business
    // rule below, not folded into the auth check itself.
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
        env.storage().persistent().extend_ttl(
            &key,
            PAYMENT_LIFETIME_THRESHOLD,
            PAYMENT_BUMP_AMOUNT,
        );

        event::publish_payment_recorded(&env, payment_id, &record);

        return Ok(());
    }

    pub fn get_payment(
        env: Env,
        payment_id: BytesN<16>,
    ) -> Result<PaymentRecord, PaymentRegistryError> {
        let key = DataKey::Payment(payment_id);
        let record: Option<PaymentRecord> = env.storage().persistent().get(&key);

        return match record {
            Some(record) => Ok(record),
            None => Err(PaymentRegistryError::NotFound),
        };
    }

    // Deliberately unauthenticated: extending the liveness of an
    // already-public audit record is not a privileged action, and this
    // contract's entire purpose is to keep that record from being
    // archived off the live ledger.
    pub fn bump_ttl(env: Env, payment_id: BytesN<16>) -> Result<(), PaymentRegistryError> {
        let key = DataKey::Payment(payment_id);

        if !env.storage().persistent().has(&key) {
            return Err(PaymentRegistryError::NotFound);
        }

        env.storage().persistent().extend_ttl(
            &key,
            PAYMENT_LIFETIME_THRESHOLD,
            PAYMENT_BUMP_AMOUNT,
        );

        return Ok(());
    }
}
