use soroban_sdk::{contracttype, Address, BytesN, String};

// === Storage Keys

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Payment(BytesN<16>),
}

// === Payment Record

// A single immutable, independently-auditable on-chain record of an
// already-verified off-chain payment. This contract does not verify
// payments itself, it only records outcomes the backend has already
// confirmed via Horizon.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentRecord {
    pub stellar_tx_hash: BytesN<32>,
    pub amount: i128,
    pub destination: Address,
    pub reference: String,
    pub recorded_at: u64,
}
