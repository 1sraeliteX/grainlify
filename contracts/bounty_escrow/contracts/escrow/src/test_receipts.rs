//! Tests for optional require-receipt (Issue #677): receipt generation and on-chain verification.

use super::*;
use crate::events::CriticalOperationOutcome;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use token;

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>, Address) {
    let contract = e.register_stellar_asset_contract_v2(admin.clone());
    let addr = contract.address();
    (
        token::Client::new(e, &addr),
        token::StellarAssetClient::new(e, &addr),
        addr,
    )
}

/// Release then verify receipt: receipt is stored and verify_receipt returns it with correct fields.
#[test]
fn test_receipt_emitted_and_verifiable_after_release() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token_client, token_admin, token_address) = create_token_contract(&env, &admin);

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    client.init(&admin, &token_address);

    token_admin.mint(&depositor, &10_000);
    let bounty_id = 1u64;
    let amount = 3_000i128;
    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    let before_ts = env.ledger().timestamp();
    env.ledger().set_timestamp(before_ts + 100);
    client.release_funds(&bounty_id, &contributor);
    let after_ts = env.ledger().timestamp();

    // First critical operation is release -> receipt_id 1
    let receipt = client.verify_receipt(&1);
    let r = receipt.unwrap();
    assert_eq!(r.receipt_id, 1);
    assert_eq!(r.outcome, CriticalOperationOutcome::Released);
    assert_eq!(r.bounty_id, bounty_id);
    assert_eq!(r.amount, amount);
    assert_eq!(r.party, contributor);
    assert!(r.timestamp >= before_ts && r.timestamp <= after_ts);
}

/// Refund then verify receipt: receipt is stored with outcome Refunded.
#[test]
fn test_receipt_emitted_and_verifiable_after_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token_client, token_admin, token_address) = create_token_contract(&env, &admin);

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    client.init(&admin, &token_address);

    token_admin.mint(&depositor, &10_000);
    let bounty_id = 2u64;
    let amount = 2_000i128;
    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    env.ledger().set_timestamp(env.ledger().timestamp() + 2000);
    client.refund(&bounty_id);

    let receipt = client.verify_receipt(&1);
    let r = receipt.unwrap();
    assert_eq!(r.receipt_id, 1);
    assert_eq!(r.outcome, CriticalOperationOutcome::Refunded);
    assert_eq!(r.bounty_id, bounty_id);
    assert_eq!(r.amount, amount);
    assert_eq!(r.party, depositor);
}

/// Multiple operations produce multiple receipts with monotonic ids; verify_receipt(nonexistent) is None.
#[test]
fn test_multiple_receipts_and_verify_nonexistent() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token_client, token_admin, token_address) = create_token_contract(&env, &admin);

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    client.init(&admin, &token_address);

    token_admin.mint(&depositor, &20_000);
    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &1, &5_000, &deadline);
    client.lock_funds(&depositor, &2, &5_000, &deadline);

    client.release_funds(&1, &contributor);
    let r1 = client.verify_receipt(&1).unwrap();
    assert_eq!(r1.outcome, CriticalOperationOutcome::Released);
    assert_eq!(r1.bounty_id, 1);

    env.ledger().set_timestamp(env.ledger().timestamp() + 2000);
    client.refund(&2);
    let r2 = client.verify_receipt(&2).unwrap();
    assert_eq!(r2.outcome, CriticalOperationOutcome::Refunded);
    assert_eq!(r2.bounty_id, 2);

    assert!(client.verify_receipt(&0).is_none());
    assert!(client.verify_receipt(&99).is_none());
}
