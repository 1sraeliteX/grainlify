use crate::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Env, Address, String,
};

fn setup_contract() -> (Env, ProgramEscrowContract, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContract::new(&env, &contract_id);

    let admin = Address::generate(&env);

    // Initialize contract
    client.initialize_contract(&admin);

    (env, client, admin)
}

#[test]
fn test_configure_timelock() {
    let (env, client, _admin) = setup_contract();

    // Test configuring timelock with valid delay
    let delay = 7200; // 2 hours
    client.configure_timelock(&delay, &true);

    let config = client.get_timelock_config();
    assert_eq!(config.delay, delay);
    assert_eq!(config.is_enabled, true);

    // Test configuring timelock with disabled
    client.configure_timelock(&86400, &false);
    let config = client.get_timelock_config();
    assert_eq!(config.delay, 86400);
    assert_eq!(config.is_enabled, false);
}

#[test]
#[should_panic(expected = "Delay below minimum")]
fn test_configure_timelock_below_minimum() {
    let (env, client, _admin) = setup_contract();

    // Try to configure with delay below minimum (3599 < 3600)
    client.configure_timelock(&3599, &true);
}

#[test]
#[should_panic(expected = "Delay above maximum")]
fn test_configure_timelock_above_maximum() {
    let (env, client, _admin) = setup_contract();

    // Try to configure with delay above maximum (2592001 > 2592000)
    client.configure_timelock(&2592001, &true);
}

#[test]
fn test_propose_admin_action_immediate_execution_when_disabled() {
    let (env, client, admin) = setup_contract();

    // Ensure timelock is disabled
    client.configure_timelock(&86400, &false);

    // Propose admin change - should execute immediately
    let action_id = client.propose_admin_action(&ActionType::ChangeAdmin);

    // Should return 0 to signal immediate execution
    assert_eq!(action_id, 0);
}

#[test]
fn test_propose_admin_action_creates_pending_when_enabled() {
    let (env, client, admin) = setup_contract();

    // Enable timelock with 2-hour delay
    let delay = 7200;
    client.configure_timelock(&delay, &true);

    let current_timestamp = env.ledger().timestamp();

    // Propose admin change
    let action_id = client.propose_admin_action(&ActionType::ChangeAdmin);

    // Should return a non-zero action ID
    assert!(action_id > 0);

    // Verify pending action
    let action = client.get_action(&action_id).expect("Action not found");
    assert_eq!(action.action_id, action_id);
    assert_eq!(action.action_type, ActionType::ChangeAdmin);
    assert_eq!(action.proposed_by, admin);
    assert_eq!(action.proposed_at, current_timestamp);
    assert_eq!(action.execute_after, current_timestamp + delay);
    assert_eq!(action.status, ActionStatus::Pending);
}

#[test]
#[should_panic(expected = "Timelock not elapsed")]
fn test_execute_before_delay_reverts() {
    let (env, client, _admin) = setup_contract();

    // Enable timelock with 2-hour delay
    let delay = 7200;
    client.configure_timelock(&delay, &true);

    // Propose admin change
    let action_id = client.propose_admin_action(&ActionType::ChangeAdmin);

    // Try to execute immediately (before delay)
    client.execute_after_delay(&action_id);
}

#[test]
fn test_execute_at_exact_delay_succeeds() {
    let (env, client, _admin) = setup_contract();

    // Enable timelock with 2-hour delay
    let delay = 7200;
    client.configure_timelock(&delay, &true);

    let start_timestamp = env.ledger().timestamp();

    // Propose admin change
    let action_id = client.propose_admin_action(&ActionType::ChangeAdmin);

    // Advance time exactly to the execute_after timestamp
    env.ledger().set_timestamp(start_timestamp + delay);

    // Execute should succeed
    client.execute_after_delay(&action_id);

    // Action should be marked as executed
    let action = client.get_action(&action_id).expect("Action not found");
    assert_eq!(action.status, ActionStatus::Executed);
}

#[test]
fn test_cancel_pending_action() {
    let (env, client, _admin) = setup_contract();

    // Enable timelock
    client.configure_timelock(&7200, &true);

    // Propose admin change
    let action_id = client.propose_admin_action(&ActionType::ChangeAdmin);

    // Cancel the action
    client.cancel_admin_action(&action_id);

    // Action should be marked as cancelled
    let action = client.get_action(&action_id).expect("Action not found");
    assert_eq!(action.status, ActionStatus::Cancelled);
}

#[test]
#[should_panic(expected = "Action not pending")]
fn test_execute_cancelled_action_reverts() {
    let (env, client, _admin) = setup_contract();

    // Enable timelock
    client.configure_timelock(&7200, &true);

    // Propose admin change
    let action_id = client.propose_admin_action(&ActionType::ChangeAdmin);

    // Cancel the action
    client.cancel_admin_action(&action_id);

    // Try to execute cancelled action
    client.execute_after_delay(&action_id);
}

#[test]
#[should_panic(expected = "Timelock enabled - use propose_admin_action")]
fn test_direct_admin_call_blocked_when_enabled() {
    let (env, client, _admin) = setup_contract();

    // Enable timelock
    client.configure_timelock(&7200, &true);

    // Try direct admin call - should be blocked
    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);
}

#[test]
fn test_direct_admin_call_works_when_disabled() {
    let (env, client, _admin) = setup_contract();

    // Ensure timelock is disabled
    client.configure_timelock(&7200, &false);

    // Direct admin call should work
    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    // Verify admin is changed
    let current_admin = client.get_admin().expect("Admin not set");
    assert_eq!(current_admin, new_admin);
}

#[test]
fn test_get_pending_actions_ordered_by_time() {
    let (env, client, _admin) = setup_contract();

    // Enable timelock
    client.configure_timelock(&7200, &true);

    // Propose first action
    let action_id1 = client.propose_admin_action(&ActionType::ChangeAdmin);

    // Advance time a bit
    env.ledger().set_timestamp(env.ledger().timestamp() + 100);

    // Propose second action
    let action_id2 = client.propose_admin_action(&ActionType::SetPauseState);

    // Get pending actions
    let pending = client.get_pending_actions();
    assert_eq!(pending.len(), 2);

    // Should be ordered by proposed_at (earliest first)
    assert_eq!(pending.get(0).unwrap().action_id, action_id1);
    assert_eq!(pending.get(1).unwrap().action_id, action_id2);
}
