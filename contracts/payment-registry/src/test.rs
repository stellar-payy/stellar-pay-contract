#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String};

use crate::contract::{PaymentRegistry, PaymentRegistryClient};
use crate::error::PaymentRegistryError;

// === Test Setup

fn setup(env: &Env) -> (PaymentRegistryClient<'_>, Address) {
    env.mock_all_auths();

    let contract_id = env.register(PaymentRegistry, ());
    let client = PaymentRegistryClient::new(env, &contract_id);
    let admin = Address::generate(env);

    client.init(&admin);

    return (client, admin);
}

fn sample_payment_id(env: &Env, seed: u8) -> BytesN<16> {
    return BytesN::from_array(env, &[seed; 16]);
}

fn sample_tx_hash(env: &Env, seed: u8) -> BytesN<32> {
    return BytesN::from_array(env, &[seed; 32]);
}

// === Tests

#[test]
fn record_and_read_back_payment() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let payment_id = sample_payment_id(&env, 1);
    let tx_hash = sample_tx_hash(&env, 2);
    let destination = Address::generate(&env);
    let reference = String::from_str(&env, "invoice-001");
    let amount: i128 = 10_000_000;

    client.record_payment(
        &admin,
        &payment_id,
        &tx_hash,
        &amount,
        &destination,
        &reference,
    );

    let record = client.get_payment(&payment_id);
    assert_eq!(record.amount, amount);
    assert_eq!(record.destination, destination);
    assert_eq!(record.stellar_tx_hash, tx_hash);
    assert_eq!(record.reference, reference);
}

#[test]
fn duplicate_payment_id_is_rejected() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let payment_id = sample_payment_id(&env, 1);
    let tx_hash = sample_tx_hash(&env, 2);
    let destination = Address::generate(&env);
    let reference = String::from_str(&env, "invoice-001");

    client.record_payment(
        &admin,
        &payment_id,
        &tx_hash,
        &100i128,
        &destination,
        &reference,
    );

    let result = client.try_record_payment(
        &admin,
        &payment_id,
        &tx_hash,
        &100i128,
        &destination,
        &reference,
    );
    assert_eq!(result, Err(Ok(PaymentRegistryError::AlreadyRecorded)));
}

#[test]
fn non_admin_caller_is_unauthorized() {
    let env = Env::default();
    let (client, _admin) = setup(&env);

    // mock_all_auths() bypasses real signature verification, so this
    // exercises the caller == admin business rule specifically, not
    // require_auth() itself.
    let non_admin = Address::generate(&env);
    let payment_id = sample_payment_id(&env, 1);
    let tx_hash = sample_tx_hash(&env, 2);
    let destination = Address::generate(&env);
    let reference = String::from_str(&env, "invoice-001");

    let result = client.try_record_payment(
        &non_admin,
        &payment_id,
        &tx_hash,
        &100i128,
        &destination,
        &reference,
    );
    assert_eq!(result, Err(Ok(PaymentRegistryError::Unauthorized)));
}

#[test]
fn get_payment_on_unrecorded_id_is_not_found() {
    let env = Env::default();
    let (client, _admin) = setup(&env);

    let payment_id = sample_payment_id(&env, 9);

    let result = client.try_get_payment(&payment_id);
    assert_eq!(result, Err(Ok(PaymentRegistryError::NotFound)));
}

#[test]
fn double_init_is_rejected() {
    let env = Env::default();
    let (client, _admin) = setup(&env);

    let another_admin = Address::generate(&env);
    let result = client.try_init(&another_admin);
    assert_eq!(result, Err(Ok(PaymentRegistryError::AlreadyInitialized)));
}

#[test]
fn zero_or_negative_amount_is_rejected() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let payment_id = sample_payment_id(&env, 1);
    let tx_hash = sample_tx_hash(&env, 2);
    let destination = Address::generate(&env);
    let reference = String::from_str(&env, "invoice-001");

    let zero_result = client.try_record_payment(
        &admin,
        &payment_id,
        &tx_hash,
        &0i128,
        &destination,
        &reference,
    );
    assert_eq!(zero_result, Err(Ok(PaymentRegistryError::InvalidAmount)));

    let negative_result = client.try_record_payment(
        &admin,
        &payment_id,
        &tx_hash,
        &-5i128,
        &destination,
        &reference,
    );
    assert_eq!(
        negative_result,
        Err(Ok(PaymentRegistryError::InvalidAmount))
    );
}

#[test]
fn bump_ttl_succeeds_on_existing_record() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let payment_id = sample_payment_id(&env, 1);
    let tx_hash = sample_tx_hash(&env, 2);
    let destination = Address::generate(&env);
    let reference = String::from_str(&env, "invoice-001");

    client.record_payment(
        &admin,
        &payment_id,
        &tx_hash,
        &100i128,
        &destination,
        &reference,
    );

    // Unauthenticated by design: no auth mocking is required for this call.
    client.bump_ttl(&payment_id);
}

#[test]
fn bump_ttl_on_unrecorded_id_is_not_found() {
    let env = Env::default();
    let (client, _admin) = setup(&env);

    let payment_id = sample_payment_id(&env, 9);

    let result = client.try_bump_ttl(&payment_id);
    assert_eq!(result, Err(Ok(PaymentRegistryError::NotFound)));
}
