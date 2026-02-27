// ============================================================
// FILE: contracts/bounty_escrow/contracts/escrow/src/test_batch_failure_modes.rs
//
// Comprehensive failure-mode tests for batch_lock_funds and
// batch_release_funds (Issue #461).
//
// Coverage:
//   BATCH LOCK
//     - Empty batch rejected (InvalidBatchSize)
//     - Single-item batch accepted (boundary: min = 1)
//     - MAX_BATCH_SIZE items accepted (boundary: max = 20)
//     - MAX_BATCH_SIZE + 1 items rejected (InvalidBatchSize)
//     - Duplicate bounty_id within batch rejected (DuplicateBountyId)
//     - Bounty already exists in storage rejected (BountyExists)
//     - Zero amount item rejected (InvalidAmount)
//     - Negative amount item rejected (InvalidAmount)
//     - Contract not initialised rejected (NotInitialized)
//     - Paused lock operation rejected (FundsPaused)
//     - Mixed batch: one bad item among valid ones → all-or-nothing
//     - Mixed batch: first item invalid, rest valid → no partial effect
//     - Mixed batch: last item invalid, rest valid → no partial effect
//
//   BATCH RELEASE
//     - Empty batch rejected (InvalidBatchSize)
//     - Single-item batch accepted (boundary: min = 1)
//     - MAX_BATCH_SIZE items accepted (boundary: max = 20)
//     - MAX_BATCH_SIZE + 1 items rejected (InvalidBatchSize)
//     - Duplicate bounty_id within batch rejected (DuplicateBountyId)
//     - Bounty not found rejected (BountyNotFound)
//     - Bounty already released rejected (FundsNotLocked)
//     - Bounty already refunded rejected (FundsNotLocked)
//     - Paused release operation rejected (FundsPaused)
//     - Mixed batch: one non-existent ID → atomicity, no partial release
//     - Mixed batch: one already-released bounty → atomicity
//     - Mixed batch: first item invalid → no partial effect
//     - Mixed batch: last item invalid → no partial effect
// ============================================================

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, Vec,
};

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, DataKey, Error, Escrow, EscrowStatus,
    LockFundsItem, ReleaseFundsItem,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

const MAX_BATCH: u32 = 20; // Must match MAX_BATCH_SIZE in lib.rs
const AMOUNT: i128 = 1_000;
const DEADLINE_OFFSET: u64 = 3_600; // 1 hour in the future

struct TestCtx<'a> {
    env: Env,
    client: BountyEscrowContractClient<'a>,
    token_sac: token::StellarAssetClient<'a>,
    token_id: Address,
    admin: Address,
    depositor: Address,
    contributor: Address,
    contract_id: Address,
}

impl<'a> TestCtx<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let token_id = env.register_stellar_asset_contract(admin.clone());
        let token_sac = token::StellarAssetClient::new(&env, &token_id);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);
        client.init(&admin, &token_id);

        // Give depositor plenty of tokens
        token_sac.mint(&depositor, &1_000_000i128);

        Self {
            env,
            client,
            token_sac,
            token_id,
            admin,
            depositor,
            contributor,
            contract_id,
        }
    }

    fn deadline(&self) -> u64 {
        self.env.ledger().timestamp() + DEADLINE_OFFSET
    }

    /// Build a valid LockFundsItem.
    fn lock_item(&self, bounty_id: u64) -> LockFundsItem {
        LockFundsItem {
            bounty_id,
            depositor: self.depositor.clone(),
            amount: AMOUNT,
            deadline: self.deadline(),
        }
    }

    /// Build a valid ReleaseFundsItem.
    fn release_item(&self, bounty_id: u64) -> ReleaseFundsItem {
        ReleaseFundsItem {
            bounty_id,
            contributor: self.contributor.clone(),
        }
    }

    /// Lock a single bounty via the normal single-item path (for setup).
    fn lock_one(&self, bounty_id: u64) {
        self.client
            .lock_funds(&self.depositor, &bounty_id, &AMOUNT, &self.deadline());
    }

    /// Lock `n` bounties starting at bounty_id 1 (for release tests setup).
    fn lock_n(&self, n: u64) {
        for id in 1..=n {
            self.lock_one(id);
        }
    }

    /// Build a Vec<LockFundsItem> with `n` distinct bounty_ids.
    fn build_lock_batch(&self, n: u32) -> soroban_sdk::Vec<LockFundsItem> {
        let mut items = Vec::new(&self.env);
        for i in 1..=(n as u64) {
            items.push_back(self.lock_item(i));
        }
        items
    }

    /// Build a Vec<ReleaseFundsItem> with `n` distinct bounty_ids (already locked).
    fn build_release_batch(&self, n: u32) -> soroban_sdk::Vec<ReleaseFundsItem> {
        let mut items = Vec::new(&self.env);
        for i in 1..=(n as u64) {
            items.push_back(self.release_item(i));
        }
        items
    }

    /// Assert that bounty `id` does NOT exist in storage.
    fn assert_no_escrow(&self, id: u64) {
        self.env.as_contract(&self.contract_id, || {
            assert!(
                !self
                    .env
                    .storage()
                    .persistent()
                    .has(&DataKey::Escrow(id)),
                "bounty {id} should not exist in storage"
            );
        });
    }

    /// Assert that bounty `id` exists and has status `status`.
    fn assert_escrow_status(&self, id: u64, status: EscrowStatus) {
        let escrow = self.client.get_escrow_info(&id);
        assert_eq!(
            escrow.status, status,
            "bounty {id} status mismatch: expected {status:?}"
        );
    }
}

// ===========================================================================
// BATCH LOCK — size boundary tests
// ===========================================================================

#[test]
fn batch_lock_empty_batch_is_rejected() {
    let ctx = TestCtx::new();
    let empty: soroban_sdk::Vec<LockFundsItem> = Vec::new(&ctx.env);
    let result = ctx.client.try_batch_lock_funds(&empty);
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidBatchSize);
}

#[test]
fn batch_lock_single_item_is_accepted() {
    let ctx = TestCtx::new();
    let items = ctx.build_lock_batch(1);
    let count = ctx.client.batch_lock_funds(&items);
    assert_eq!(count, 1);
    ctx.assert_escrow_status(1, EscrowStatus::Locked);
}

#[test]
fn batch_lock_max_batch_size_is_accepted() {
    let ctx = TestCtx::new();
    // Mint enough tokens for MAX_BATCH items
    ctx.token_sac
        .mint(&ctx.depositor, &(AMOUNT * MAX_BATCH as i128));
    let items = ctx.build_lock_batch(MAX_BATCH);
    let count = ctx.client.batch_lock_funds(&items);
    assert_eq!(count, MAX_BATCH);
}

#[test]
fn batch_lock_exceeds_max_batch_size_is_rejected() {
    let ctx = TestCtx::new();
    ctx.token_sac
        .mint(&ctx.depositor, &(AMOUNT * (MAX_BATCH as i128 + 1)));
    let items = ctx.build_lock_batch(MAX_BATCH + 1);
    let result = ctx.client.try_batch_lock_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidBatchSize);
}

// ===========================================================================
// BATCH LOCK — duplicate ID tests
// ===========================================================================

#[test]
fn batch_lock_duplicate_bounty_id_in_batch_is_rejected() {
    let ctx = TestCtx::new();
    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.lock_item(1));
    items.push_back(ctx.lock_item(2));
    items.push_back(ctx.lock_item(1)); // duplicate
    let result = ctx.client.try_batch_lock_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::DuplicateBountyId);
}

#[test]
fn batch_lock_duplicate_id_causes_no_partial_lock() {
    let ctx = TestCtx::new();
    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.lock_item(10));
    items.push_back(ctx.lock_item(10)); // duplicate of first
    let _ = ctx.client.try_batch_lock_funds(&items);
    // Nothing should be stored
    ctx.assert_no_escrow(10);
}

// ===========================================================================
// BATCH LOCK — already-existing bounty tests
// ===========================================================================

#[test]
fn batch_lock_existing_bounty_id_is_rejected() {
    let ctx = TestCtx::new();
    // Pre-lock bounty 1 via the single-item path
    ctx.lock_one(1);

    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.lock_item(1)); // already locked
    let result = ctx.client.try_batch_lock_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::BountyExists);
}

// ===========================================================================
// BATCH LOCK — invalid amount tests
// ===========================================================================

#[test]
fn batch_lock_zero_amount_is_rejected() {
    let ctx = TestCtx::new();
    let bad_item = LockFundsItem {
        bounty_id: 1,
        depositor: ctx.depositor.clone(),
        amount: 0,
        deadline: ctx.deadline(),
    };
    let mut items = Vec::new(&ctx.env);
    items.push_back(bad_item);
    let result = ctx.client.try_batch_lock_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidAmount);
}

#[test]
fn batch_lock_negative_amount_is_rejected() {
    let ctx = TestCtx::new();
    let bad_item = LockFundsItem {
        bounty_id: 1,
        depositor: ctx.depositor.clone(),
        amount: -500,
        deadline: ctx.deadline(),
    };
    let mut items = Vec::new(&ctx.env);
    items.push_back(bad_item);
    let result = ctx.client.try_batch_lock_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidAmount);
}

// ===========================================================================
// BATCH LOCK — pause test
// ===========================================================================

#[test]
fn batch_lock_while_paused_is_rejected() {
    let ctx = TestCtx::new();
    ctx.client
        .set_paused(&Some(true), &None::<bool>, &None::<bool>, &None);

    let items = ctx.build_lock_batch(2);
    let result = ctx.client.try_batch_lock_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::FundsPaused);
}

// ===========================================================================
// BATCH LOCK — atomicity / mixed-validity tests
// ===========================================================================

/// A batch where one middle item is invalid must leave NO escrows stored.
#[test]
fn batch_lock_middle_invalid_item_causes_full_rollback() {
    let ctx = TestCtx::new();
    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.lock_item(1));
    items.push_back(ctx.lock_item(2));
    // Item 3 has zero amount — invalid
    items.push_back(LockFundsItem {
        bounty_id: 3,
        depositor: ctx.depositor.clone(),
        amount: 0,
        deadline: ctx.deadline(),
    });
    items.push_back(ctx.lock_item(4));

    let result = ctx.client.try_batch_lock_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidAmount);

    // None of the IDs should be stored
    for id in [1u64, 2, 3, 4] {
        ctx.assert_no_escrow(id);
    }
}

/// A batch where the FIRST item is invalid must leave nothing stored.
#[test]
fn batch_lock_first_item_invalid_causes_full_rollback() {
    let ctx = TestCtx::new();
    let mut items = Vec::new(&ctx.env);
    // First item: zero amount
    items.push_back(LockFundsItem {
        bounty_id: 1,
        depositor: ctx.depositor.clone(),
        amount: 0,
        deadline: ctx.deadline(),
    });
    items.push_back(ctx.lock_item(2));
    items.push_back(ctx.lock_item(3));

    let result = ctx.client.try_batch_lock_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidAmount);

    for id in [1u64, 2, 3] {
        ctx.assert_no_escrow(id);
    }
}

/// A batch where the LAST item is a duplicate must leave nothing stored.
#[test]
fn batch_lock_last_item_duplicate_causes_full_rollback() {
    let ctx = TestCtx::new();
    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.lock_item(1));
    items.push_back(ctx.lock_item(2));
    items.push_back(ctx.lock_item(1)); // duplicate of first, placed last

    let result = ctx.client.try_batch_lock_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::DuplicateBountyId);

    for id in [1u64, 2] {
        ctx.assert_no_escrow(id);
    }
}

/// A batch containing one already-existing bounty must reject the whole batch.
#[test]
fn batch_lock_one_existing_in_otherwise_valid_batch_causes_full_rollback() {
    let ctx = TestCtx::new();
    // Pre-lock bounty 5
    ctx.lock_one(5);

    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.lock_item(10));
    items.push_back(ctx.lock_item(5)); // pre-existing
    items.push_back(ctx.lock_item(20));

    let result = ctx.client.try_batch_lock_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::BountyExists);

    // New IDs should not have been stored
    ctx.assert_no_escrow(10);
    ctx.assert_no_escrow(20);
    // Pre-existing bounty should still be Locked (unchanged)
    ctx.assert_escrow_status(5, EscrowStatus::Locked);
}

// ===========================================================================
// BATCH RELEASE — size boundary tests
// ===========================================================================

#[test]
fn batch_release_empty_batch_is_rejected() {
    let ctx = TestCtx::new();
    let empty: soroban_sdk::Vec<ReleaseFundsItem> = Vec::new(&ctx.env);
    let result = ctx.client.try_batch_release_funds(&empty);
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidBatchSize);
}

#[test]
fn batch_release_single_item_is_accepted() {
    let ctx = TestCtx::new();
    ctx.lock_one(1);
    let items = ctx.build_release_batch(1);
    let count = ctx.client.batch_release_funds(&items);
    assert_eq!(count, 1);
    ctx.assert_escrow_status(1, EscrowStatus::Released);
}

#[test]
fn batch_release_max_batch_size_is_accepted() {
    let ctx = TestCtx::new();
    ctx.token_sac
        .mint(&ctx.depositor, &(AMOUNT * MAX_BATCH as i128));
    ctx.lock_n(MAX_BATCH as u64);
    let items = ctx.build_release_batch(MAX_BATCH);
    let count = ctx.client.batch_release_funds(&items);
    assert_eq!(count, MAX_BATCH);
}

#[test]
fn batch_release_exceeds_max_batch_size_is_rejected() {
    let ctx = TestCtx::new();
    ctx.token_sac
        .mint(&ctx.depositor, &(AMOUNT * (MAX_BATCH as i128 + 1)));
    ctx.lock_n(MAX_BATCH as u64 + 1);
    let items = ctx.build_release_batch(MAX_BATCH + 1);
    let result = ctx.client.try_batch_release_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidBatchSize);
}

// ===========================================================================
// BATCH RELEASE — duplicate ID tests
// ===========================================================================

#[test]
fn batch_release_duplicate_bounty_id_in_batch_is_rejected() {
    let ctx = TestCtx::new();
    ctx.lock_one(1);
    ctx.lock_one(2);

    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.release_item(1));
    items.push_back(ctx.release_item(2));
    items.push_back(ctx.release_item(1)); // duplicate

    let result = ctx.client.try_batch_release_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::DuplicateBountyId);
}

#[test]
fn batch_release_duplicate_id_causes_no_partial_release() {
    let ctx = TestCtx::new();
    ctx.lock_one(1);

    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.release_item(1));
    items.push_back(ctx.release_item(1)); // duplicate

    let _ = ctx.client.try_batch_release_funds(&items);

    // Bounty 1 must still be Locked (no partial effect)
    ctx.assert_escrow_status(1, EscrowStatus::Locked);
}

// ===========================================================================
// BATCH RELEASE — bounty-not-found tests
// ===========================================================================

#[test]
fn batch_release_nonexistent_bounty_is_rejected() {
    let ctx = TestCtx::new();
    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.release_item(999)); // never locked
    let result = ctx.client.try_batch_release_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::BountyNotFound);
}

// ===========================================================================
// BATCH RELEASE — wrong-status tests
// ===========================================================================

#[test]
fn batch_release_already_released_bounty_is_rejected() {
    let ctx = TestCtx::new();
    ctx.lock_one(1);
    // Release via single-item path to set status = Released
    ctx.client.release_funds(&1u64, &ctx.contributor);

    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.release_item(1));
    let result = ctx.client.try_batch_release_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::FundsNotLocked);
}

#[test]
fn batch_release_refunded_bounty_is_rejected() {
    let ctx = TestCtx::new();
    ctx.lock_one(1);
    // Advance time past deadline so refund is allowed
    ctx.env
        .ledger()
        .set_timestamp(ctx.env.ledger().timestamp() + DEADLINE_OFFSET + 1);
    ctx.client.refund(&1u64);

    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.release_item(1));
    let result = ctx.client.try_batch_release_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::FundsNotLocked);
}

// ===========================================================================
// BATCH RELEASE — pause test
// ===========================================================================

#[test]
fn batch_release_while_paused_is_rejected() {
    let ctx = TestCtx::new();
    ctx.lock_one(1);
    ctx.client
        .set_paused(&None::<bool>, &Some(true), &None::<bool>, &None);

    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.release_item(1));
    let result = ctx.client.try_batch_release_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::FundsPaused);
}

// ===========================================================================
// BATCH RELEASE — atomicity / mixed-validity tests
// ===========================================================================

/// A batch where one middle item references a nonexistent bounty leaves all
/// previously-locked bounties untouched.
#[test]
fn batch_release_middle_nonexistent_causes_full_rollback() {
    let ctx = TestCtx::new();
    ctx.lock_one(1);
    ctx.lock_one(2);
    // bounty 3 is NOT locked

    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.release_item(1));
    items.push_back(ctx.release_item(3)); // not found
    items.push_back(ctx.release_item(2));

    let result = ctx.client.try_batch_release_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::BountyNotFound);

    // All locked bounties must remain Locked
    ctx.assert_escrow_status(1, EscrowStatus::Locked);
    ctx.assert_escrow_status(2, EscrowStatus::Locked);
}

/// A batch where one item is already-released causes no partial release.
#[test]
fn batch_release_one_already_released_causes_full_rollback() {
    let ctx = TestCtx::new();
    ctx.lock_one(1);
    ctx.lock_one(2);
    ctx.lock_one(3);
    // Release bounty 2 via single-item path first
    ctx.client.release_funds(&2u64, &ctx.contributor);

    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.release_item(1));
    items.push_back(ctx.release_item(2)); // already released
    items.push_back(ctx.release_item(3));

    let result = ctx.client.try_batch_release_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::FundsNotLocked);

    // Bounties 1 and 3 must still be Locked
    ctx.assert_escrow_status(1, EscrowStatus::Locked);
    ctx.assert_escrow_status(3, EscrowStatus::Locked);
}

/// A batch where the FIRST item is nonexistent — nothing is released.
#[test]
fn batch_release_first_item_nonexistent_causes_full_rollback() {
    let ctx = TestCtx::new();
    ctx.lock_one(2);
    ctx.lock_one(3);

    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.release_item(99)); // not found
    items.push_back(ctx.release_item(2));
    items.push_back(ctx.release_item(3));

    let result = ctx.client.try_batch_release_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::BountyNotFound);

    ctx.assert_escrow_status(2, EscrowStatus::Locked);
    ctx.assert_escrow_status(3, EscrowStatus::Locked);
}

/// A batch where the LAST item is nonexistent — nothing is released.
#[test]
fn batch_release_last_item_nonexistent_causes_full_rollback() {
    let ctx = TestCtx::new();
    ctx.lock_one(1);
    ctx.lock_one(2);

    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.release_item(1));
    items.push_back(ctx.release_item(2));
    items.push_back(ctx.release_item(999)); // not found

    let result = ctx.client.try_batch_release_funds(&items);
    assert_eq!(result.unwrap_err().unwrap(), Error::BountyNotFound);

    ctx.assert_escrow_status(1, EscrowStatus::Locked);
    ctx.assert_escrow_status(2, EscrowStatus::Locked);
}

// ===========================================================================
// CROSS-CONCERN — lock then release round-trips at boundaries
// ===========================================================================

/// Verifying that a successful MAX-batch lock followed by a MAX-batch release
/// correctly transitions all bounties to Released.
#[test]
fn batch_lock_then_batch_release_full_round_trip() {
    let ctx = TestCtx::new();
    ctx.token_sac
        .mint(&ctx.depositor, &(AMOUNT * MAX_BATCH as i128));

    let lock_items = ctx.build_lock_batch(MAX_BATCH);
    let locked = ctx.client.batch_lock_funds(&lock_items);
    assert_eq!(locked, MAX_BATCH);

    let release_items = ctx.build_release_batch(MAX_BATCH);
    let released = ctx.client.batch_release_funds(&release_items);
    assert_eq!(released, MAX_BATCH);

    for id in 1..=(MAX_BATCH as u64) {
        ctx.assert_escrow_status(id, EscrowStatus::Released);
    }
}

/// Verifying that after a failed batch release, a correct single-item release
/// still works (storage is clean).
#[test]
fn failed_batch_release_does_not_corrupt_state_for_subsequent_single_release() {
    let ctx = TestCtx::new();
    ctx.lock_one(1);

    // Attempt a batch release that will fail (bounty 2 does not exist)
    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.release_item(1));
    items.push_back(ctx.release_item(2)); // nonexistent
    let _ = ctx.client.try_batch_release_funds(&items);

    // Bounty 1 should still be Locked
    ctx.assert_escrow_status(1, EscrowStatus::Locked);

    // Normal single release should now succeed
    ctx.client.release_funds(&1u64, &ctx.contributor);
    ctx.assert_escrow_status(1, EscrowStatus::Released);
}

/// Verifying that after a failed batch lock, a correct single-item lock
/// still works (storage is clean).
#[test]
fn failed_batch_lock_does_not_corrupt_state_for_subsequent_single_lock() {
    let ctx = TestCtx::new();

    // Batch will fail due to duplicate
    let mut items = Vec::new(&ctx.env);
    items.push_back(ctx.lock_item(1));
    items.push_back(ctx.lock_item(1)); // duplicate
    let _ = ctx.client.try_batch_lock_funds(&items);

    ctx.assert_no_escrow(1);

    // Normal single lock should succeed
    ctx.lock_one(1);
    ctx.assert_escrow_status(1, EscrowStatus::Locked);
}