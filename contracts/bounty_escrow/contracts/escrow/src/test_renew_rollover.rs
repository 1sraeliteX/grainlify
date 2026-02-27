use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, token, Address, Env};

// ---------------------------------------------------------------------------
// Test setup helper
// ---------------------------------------------------------------------------

struct RenewTestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    _token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> RenewTestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        let token_admin_addr = Address::generate(&env);

        let token_id = env
            .register_stellar_asset_contract_v2(token_admin_addr.clone())
            .address();
        let token = token::Client::new(&env, &token_id);
        let token_admin = token::StellarAssetClient::new(&env, &token_id);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let escrow = BountyEscrowContractClient::new(&env, &contract_id);
        escrow.init(&admin, &token_id);

        // Give depositor plenty of tokens
        token_admin.mint(&depositor, &10_000_000);

        Self {
            env,
            admin,
            depositor,
            contributor,
            _token: token,
            token_admin,
            escrow,
        }
    }

    /// Lock a bounty and return the bounty_id
    fn lock_bounty(&self, bounty_id: u64, amount: i128, deadline: u64) {
        self.escrow
            .lock_funds(&self.depositor, &bounty_id, &amount, &deadline);
    }
}

// ===========================================================================
// Renew Escrow Tests
// ===========================================================================

#[test]
fn test_renew_escrow_extends_deadline() {
    let s = RenewTestSetup::new();
    let bounty_id = 100_u64;
    let amount = 5_000_i128;
    let initial_deadline = s.env.ledger().timestamp() + 1_000;

    s.lock_bounty(bounty_id, amount, initial_deadline);

    // Renew with extended deadline, no additional funds
    let new_deadline = initial_deadline + 2_000;
    s.escrow.renew_escrow(&bounty_id, &new_deadline, &0_i128);

    let escrow = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.deadline, new_deadline);
    assert_eq!(escrow.amount, amount); // amount unchanged
    assert_eq!(escrow.remaining_amount, amount); // no funds released
    assert_eq!(escrow.status, EscrowStatus::Locked);
}

#[test]
fn test_renew_escrow_with_topup() {
    let s = RenewTestSetup::new();
    let bounty_id = 101_u64;
    let amount = 5_000_i128;
    let initial_deadline = s.env.ledger().timestamp() + 1_000;

    s.lock_bounty(bounty_id, amount, initial_deadline);

    let new_deadline = initial_deadline + 2_000;
    let topup = 3_000_i128;
    s.escrow.renew_escrow(&bounty_id, &new_deadline, &topup);

    let escrow = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.deadline, new_deadline);
    assert_eq!(escrow.amount, amount + topup);
    assert_eq!(escrow.remaining_amount, amount + topup);
}

#[test]
fn test_renew_escrow_multiple_renewals() {
    let s = RenewTestSetup::new();
    let bounty_id = 102_u64;
    let amount = 5_000_i128;
    let d1 = s.env.ledger().timestamp() + 1_000;

    s.lock_bounty(bounty_id, amount, d1);

    // Renew twice
    let d2 = d1 + 1_000;
    s.escrow.renew_escrow(&bounty_id, &d2, &1_000);

    let d3 = d2 + 1_000;
    s.escrow.renew_escrow(&bounty_id, &d3, &500);

    let escrow = s.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.deadline, d3);
    assert_eq!(escrow.amount, amount + 1_000 + 500);

    // Check renewal history has 2 entries
    let history = s.escrow.get_renewal_history(&bounty_id);
    assert_eq!(history.len(), 2);
    assert_eq!(history.get(0).unwrap().cycle, 1);
    assert_eq!(history.get(0).unwrap().old_deadline, d1);
    assert_eq!(history.get(0).unwrap().new_deadline, d2);
    assert_eq!(history.get(0).unwrap().additional_amount, 1_000);
    assert_eq!(history.get(1).unwrap().cycle, 2);
    assert_eq!(history.get(1).unwrap().old_deadline, d2);
    assert_eq!(history.get(1).unwrap().new_deadline, d3);
    assert_eq!(history.get(1).unwrap().additional_amount, 500);
}

#[test]
#[should_panic(expected = "Error(Contract, #37)")] // RenewalNotAllowed
fn test_renew_released_escrow_fails() {
    let s = RenewTestSetup::new();
    let bounty_id = 103_u64;
    let amount = 5_000_i128;
    let deadline = s.env.ledger().timestamp() + 1_000;

    s.lock_bounty(bounty_id, amount, deadline);
    s.escrow.release_funds(&bounty_id, &s.contributor);

    // Should fail: escrow is Released
    s.escrow.renew_escrow(&bounty_id, &(deadline + 1_000), &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #37)")] // RenewalNotAllowed
fn test_renew_refunded_escrow_fails() {
    let s = RenewTestSetup::new();
    let bounty_id = 104_u64;
    let amount = 5_000_i128;
    let deadline = s.env.ledger().timestamp() + 100;

    s.lock_bounty(bounty_id, amount, deadline);

    // Advance past deadline & refund
    s.env.ledger().set_timestamp(deadline + 1);
    s.escrow.refund(&bounty_id);

    // Should fail: escrow is Refunded
    s.escrow.renew_escrow(&bounty_id, &(deadline + 5_000), &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #38)")] // InvalidRenewal
fn test_renew_with_past_deadline_fails() {
    let s = RenewTestSetup::new();
    let bounty_id = 105_u64;
    let amount = 5_000_i128;
    let deadline = s.env.ledger().timestamp() + 2_000;

    s.lock_bounty(bounty_id, amount, deadline);

    // New deadline same as old → must fail
    s.escrow.renew_escrow(&bounty_id, &deadline, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")] // InvalidAmount
fn test_renew_with_negative_amount_fails() {
    let s = RenewTestSetup::new();
    let bounty_id = 106_u64;
    let amount = 5_000_i128;
    let deadline = s.env.ledger().timestamp() + 2_000;

    s.lock_bounty(bounty_id, amount, deadline);

    s.escrow
        .renew_escrow(&bounty_id, &(deadline + 1_000), &(-100_i128));
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")] // BountyNotFound
fn test_renew_nonexistent_escrow_fails() {
    let s = RenewTestSetup::new();
    s.escrow.renew_escrow(&999_u64, &5_000, &0);
}

// ===========================================================================
// Create Next Cycle Tests
// ===========================================================================

#[test]
fn test_create_next_cycle_basic() {
    let s = RenewTestSetup::new();
    let bounty_id_1 = 200_u64;
    let bounty_id_2 = 201_u64;
    let amount = 5_000_i128;
    let deadline_1 = s.env.ledger().timestamp() + 1_000;
    let deadline_2 = s.env.ledger().timestamp() + 3_000;

    // Lock and release the first cycle
    s.lock_bounty(bounty_id_1, amount, deadline_1);
    s.escrow.release_funds(&bounty_id_1, &s.contributor);

    // Create next cycle
    s.escrow
        .create_next_cycle(&bounty_id_1, &bounty_id_2, &amount, &deadline_2);

    // Verify new escrow
    let new_escrow = s.escrow.get_escrow_info(&bounty_id_2);
    assert_eq!(new_escrow.status, EscrowStatus::Locked);
    assert_eq!(new_escrow.amount, amount);
    assert_eq!(new_escrow.deadline, deadline_2);
    assert_eq!(new_escrow.depositor, s.depositor);

    // Verify cycle links
    let link_1 = s.escrow.get_cycle_info(&bounty_id_1);
    assert_eq!(link_1.next_id, bounty_id_2);
    assert_eq!(link_1.previous_id, 0); // first cycle has no predecessor

    let link_2 = s.escrow.get_cycle_info(&bounty_id_2);
    assert_eq!(link_2.previous_id, bounty_id_1);
    assert_eq!(link_2.next_id, 0); // latest cycle has no successor
    assert_eq!(link_2.cycle, 1);
}

#[test]
fn test_create_next_cycle_after_refund() {
    let s = RenewTestSetup::new();
    let bounty_id_1 = 210_u64;
    let bounty_id_2 = 211_u64;
    let amount = 5_000_i128;
    let deadline = s.env.ledger().timestamp() + 100;

    s.lock_bounty(bounty_id_1, amount, deadline);

    // Advance past deadline and refund
    s.env.ledger().set_timestamp(deadline + 1);
    s.escrow.refund(&bounty_id_1);

    // Create next cycle from refunded escrow — allowed
    let new_deadline = s.env.ledger().timestamp() + 5_000;
    s.escrow
        .create_next_cycle(&bounty_id_1, &bounty_id_2, &amount, &new_deadline);

    let new_escrow = s.escrow.get_escrow_info(&bounty_id_2);
    assert_eq!(new_escrow.status, EscrowStatus::Locked);
}

#[test]
fn test_create_three_cycle_chain() {
    let s = RenewTestSetup::new();
    let id_1 = 300_u64;
    let id_2 = 301_u64;
    let id_3 = 302_u64;
    let amount = 2_000_i128;
    let base_time = s.env.ledger().timestamp();

    // Cycle 1: lock -> release
    s.lock_bounty(id_1, amount, base_time + 1_000);
    s.escrow.release_funds(&id_1, &s.contributor);

    // Cycle 2: create from cycle 1 -> release
    s.escrow
        .create_next_cycle(&id_1, &id_2, &amount, &(base_time + 2_000));
    s.escrow.release_funds(&id_2, &s.contributor);

    // Cycle 3: create from cycle 2
    s.escrow
        .create_next_cycle(&id_2, &id_3, &amount, &(base_time + 3_000));

    // Verify full chain
    let link_1 = s.escrow.get_cycle_info(&id_1);
    assert_eq!(link_1.previous_id, 0);
    assert_eq!(link_1.next_id, id_2);

    let link_2 = s.escrow.get_cycle_info(&id_2);
    assert_eq!(link_2.previous_id, id_1);
    assert_eq!(link_2.next_id, id_3);
    assert_eq!(link_2.cycle, 1);

    let link_3 = s.escrow.get_cycle_info(&id_3);
    assert_eq!(link_3.previous_id, id_2);
    assert_eq!(link_3.next_id, 0);
    assert_eq!(link_3.cycle, 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #37)")] // RenewalNotAllowed
fn test_create_next_cycle_from_locked_escrow_fails() {
    let s = RenewTestSetup::new();
    let bounty_id_1 = 400_u64;
    let bounty_id_2 = 401_u64;
    let amount = 5_000_i128;
    let deadline = s.env.ledger().timestamp() + 1_000;

    s.lock_bounty(bounty_id_1, amount, deadline);

    // Should fail: previous is still Locked
    s.escrow
        .create_next_cycle(&bounty_id_1, &bounty_id_2, &amount, &(deadline + 1_000));
}

#[test]
#[should_panic(expected = "Error(Contract, #37)")] // RenewalNotAllowed
fn test_create_duplicate_successor_fails() {
    let s = RenewTestSetup::new();
    let id_1 = 500_u64;
    let id_2 = 501_u64;
    let id_3 = 502_u64;
    let amount = 5_000_i128;
    let deadline = s.env.ledger().timestamp() + 1_000;

    s.lock_bounty(id_1, amount, deadline);
    s.escrow.release_funds(&id_1, &s.contributor);

    // First successor ok
    let new_deadline = s.env.ledger().timestamp() + 5_000;
    s.escrow
        .create_next_cycle(&id_1, &id_2, &amount, &new_deadline);

    // Second successor from same predecessor: should fail
    s.escrow
        .create_next_cycle(&id_1, &id_3, &amount, &new_deadline);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // BountyExists
fn test_create_next_cycle_with_existing_bounty_id_fails() {
    let s = RenewTestSetup::new();
    let id_1 = 600_u64;
    let amount = 5_000_i128;
    let deadline = s.env.ledger().timestamp() + 1_000;

    s.lock_bounty(id_1, amount, deadline);
    s.escrow.release_funds(&id_1, &s.contributor);

    // Try to create next cycle using same bounty_id
    let new_deadline = s.env.ledger().timestamp() + 5_000;
    s.escrow
        .create_next_cycle(&id_1, &id_1, &amount, &new_deadline);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")] // InvalidAmount
fn test_create_next_cycle_with_zero_amount_fails() {
    let s = RenewTestSetup::new();
    let id_1 = 700_u64;
    let id_2 = 701_u64;
    let amount = 5_000_i128;
    let deadline = s.env.ledger().timestamp() + 1_000;

    s.lock_bounty(id_1, amount, deadline);
    s.escrow.release_funds(&id_1, &s.contributor);

    let new_deadline = s.env.ledger().timestamp() + 5_000;
    s.escrow.create_next_cycle(&id_1, &id_2, &0, &new_deadline);
}

// ===========================================================================
// View Function Tests
// ===========================================================================

#[test]
fn test_get_cycle_info_no_renewal() {
    let s = RenewTestSetup::new();
    let bounty_id = 800_u64;
    let deadline = s.env.ledger().timestamp() + 1_000;
    s.lock_bounty(bounty_id, 5_000, deadline);

    // Escrow with no renewal history → default CycleLink
    let link = s.escrow.get_cycle_info(&bounty_id);
    assert_eq!(link.previous_id, 0);
    assert_eq!(link.next_id, 0);
    assert_eq!(link.cycle, 1);
}

#[test]
fn test_get_renewal_history_empty() {
    let s = RenewTestSetup::new();
    let bounty_id = 801_u64;
    let deadline = s.env.ledger().timestamp() + 1_000;
    s.lock_bounty(bounty_id, 5_000, deadline);

    let history = s.escrow.get_renewal_history(&bounty_id);
    assert_eq!(history.len(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")] // BountyNotFound
fn test_get_cycle_info_nonexistent_fails() {
    let s = RenewTestSetup::new();
    s.escrow.get_cycle_info(&999_u64);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")] // BountyNotFound
fn test_get_renewal_history_nonexistent_fails() {
    let s = RenewTestSetup::new();
    s.escrow.get_renewal_history(&999_u64);
}

// ===========================================================================
// Combined Renew + Cycle Tests
// ===========================================================================

#[test]
fn test_renew_then_release_then_new_cycle() {
    let s = RenewTestSetup::new();
    let id_1 = 900_u64;
    let id_2 = 901_u64;
    let amount = 5_000_i128;
    let d1 = s.env.ledger().timestamp() + 1_000;

    // Lock, renew twice, then release
    s.lock_bounty(id_1, amount, d1);

    let d2 = d1 + 1_000;
    s.escrow.renew_escrow(&id_1, &d2, &0);

    let d3 = d2 + 1_000;
    s.escrow.renew_escrow(&id_1, &d3, &2_000);

    // Verify renewal history before release
    let history = s.escrow.get_renewal_history(&id_1);
    assert_eq!(history.len(), 2);

    s.escrow.release_funds(&id_1, &s.contributor);

    // Now create next cycle from released escrow
    let new_deadline = s.env.ledger().timestamp() + 10_000;
    s.escrow
        .create_next_cycle(&id_1, &id_2, &amount, &new_deadline);

    // Verify chain
    let link = s.escrow.get_cycle_info(&id_2);
    assert_eq!(link.previous_id, id_1);

    // Renewal history still available on original
    let history_after = s.escrow.get_renewal_history(&id_1);
    assert_eq!(history_after.len(), 2);
}
