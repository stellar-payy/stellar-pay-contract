use soroban_sdk::{contractevent, Address, BytesN, Env, String};

use crate::types::PaymentRecord;

// === Payment Recorded Event

// Published once per successful record_payment call. payment_id is the
// event topic, so off-chain indexers can filter directly on it, and the
// rest of the record travels as event data.
#[contractevent(topics = ["PAY_REC"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentRecordedEvent {
    #[topic]
    pub payment_id: BytesN<16>,
    pub stellar_tx_hash: BytesN<32>,
    pub amount: i128,
    pub destination: Address,
    pub reference: String,
    pub recorded_at: u64,
}

pub fn publish_payment_recorded(env: &Env, payment_id: BytesN<16>, record: &PaymentRecord) {
    let event = PaymentRecordedEvent {
        payment_id,
        stellar_tx_hash: record.stellar_tx_hash.clone(),
        amount: record.amount,
        destination: record.destination.clone(),
        reference: record.reference.clone(),
        recorded_at: record.recorded_at,
    };

    event.publish(env);

    return;
}
