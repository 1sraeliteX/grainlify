//! Mock hook contract for integration testing
//!
//! This contract provides a simple hook implementation for testing the hook system
//! without requiring a full insurance or reserve pool implementation.
//!
//! ## Features
//!
//! - Track hook calls received
//! - Store last event type and amount
//! - Configurable failure mode for testing error scenarios
//! - Statistics and call history
//!
//! ## Usage
//!
//! ```ignore
//! let mock = Address::random(&env);
//! contract.set_hook_address(&env, &admin, &Some(mock.clone()));
//!
//! // Perform operations that trigger hooks
//! contract.open_dispute(&env, bounty_id, &disputer, &reason);
//!
//! // Verify hook was called
//! let stats = MockHook::get_stats(&env, &mock);
//! assert_eq!(stats.calls_received, 1);
//! ```

#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, String, Symbol, Vec,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HookEventType {
    DisputeOpened = 0,
    LargeRelease = 1,
    Refund = 2,
    DisputeResolved = 3,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct HookCall {
    pub event_type: HookEventType,
    pub bounty_id: u64,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct MockHookStats {
    pub calls_received: u32,
    pub last_event: HookEventType,
    pub last_bounty_id: u64,
    pub last_amount: i128,
    pub last_timestamp: u64,
    pub should_fail: bool,
    pub fail_message: String,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct HookCallRecord {
    pub event_type: HookEventType,
    pub bounty_id: u64,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MockHookKey {
    Stats,
    CallHistory,
    CallCount,
}

#[contract]
pub struct MockHook;

#[contractimpl]
impl MockHook {
    /// Handle hook call from escrow contract
    pub fn handle_hook(env: Env, call: HookCall) -> Result<(), String> {
        let key = Symbol::new(&env, "mock_stats");
        let mut stats: MockHookStats = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(MockHookStats {
                calls_received: 0,
                last_event: call.event_type.clone(),
                last_bounty_id: 0,
                last_amount: 0,
                last_timestamp: 0,
                should_fail: false,
                fail_message: String::from_str(&env, "Mock hook failure"),
            });

        // Update statistics
        stats.calls_received += 1;
        stats.last_event = call.event_type.clone();
        stats.last_bounty_id = call.bounty_id;
        stats.last_amount = call.amount;
        stats.last_timestamp = call.timestamp;

        env.storage().persistent().set(&key, &stats);

        // Record call in history
        let history_key = Symbol::new(&env, "call_history");
        let mut history: Vec<HookCallRecord> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or(Vec::new(&env));

        history.push_back(HookCallRecord {
            event_type: call.event_type,
            bounty_id: call.bounty_id,
            amount: call.amount,
            timestamp: call.timestamp,
        });

        env.storage().persistent().set(&history_key, &history);

        // Return error if configured to fail
        if stats.should_fail {
            Err(stats.fail_message)
        } else {
            Ok(())
        }
    }

    /// Get current statistics
    pub fn get_stats(env: Env) -> MockHookStats {
        let key = Symbol::new(&env, "mock_stats");
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(MockHookStats {
                calls_received: 0,
                last_event: HookEventType::DisputeOpened,
                last_bounty_id: 0,
                last_amount: 0,
                last_timestamp: 0,
                should_fail: false,
                fail_message: String::from_str(&env, ""),
            })
    }

    /// Get call history
    pub fn get_call_history(env: Env) -> Vec<HookCallRecord> {
        let history_key = Symbol::new(&env, "call_history");
        env.storage()
            .persistent()
            .get(&history_key)
            .unwrap_or(Vec::new(&env))
    }

    /// Configure mock to fail on next call
    pub fn set_fail(env: Env, should_fail: bool, message: String) {
        let key = Symbol::new(&env, "mock_stats");
        let mut stats = Self::get_stats(env.clone());
        stats.should_fail = should_fail;
        stats.fail_message = message;
        env.storage().persistent().set(&key, &stats);
    }

    /// Reset statistics and history
    pub fn reset(env: Env) {
        env.storage()
            .persistent()
            .remove(&Symbol::new(&env, "mock_stats"));
        env.storage()
            .persistent()
            .remove(&Symbol::new(&env, "call_history"));
    }

    /// Get number of calls received
    pub fn get_call_count(env: Env) -> u32 {
        Self::get_stats(env).calls_received
    }

    /// Check if last call matched expected values
    pub fn last_call_matches(
        env: Env,
        event_type: HookEventType,
        bounty_id: u64,
        amount: i128,
    ) -> bool {
        let stats = Self::get_stats(env);
        stats.last_event == event_type && stats.last_bounty_id == bounty_id && stats.last_amount == amount
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_hook_records_call() {
        let env = Env::default();

        let call = HookCall {
            event_type: HookEventType::DisputeOpened,
            bounty_id: 123,
            amount: 1_000_000,
            timestamp: 1000,
        };

        let result = MockHook::handle_hook(env.clone(), call.clone());
        assert!(result.is_ok());

        let stats = MockHook::get_stats(env.clone());
        assert_eq!(stats.calls_received, 1);
        assert_eq!(stats.last_event, HookEventType::DisputeOpened);
        assert_eq!(stats.last_bounty_id, 123);
        assert_eq!(stats.last_amount, 1_000_000);
    }

    #[test]
    fn test_mock_hook_failure() {
        let env = Env::default();

        MockHook::set_fail(
            env.clone(),
            true,
            String::from_str(&env, "Test failure"),
        );

        let call = HookCall {
            event_type: HookEventType::DisputeOpened,
            bounty_id: 123,
            amount: 1_000_000,
            timestamp: 1000,
        };

        let result = MockHook::handle_hook(env.clone(), call);
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_hook_reset() {
        let env = Env::default();

        let call = HookCall {
            event_type: HookEventType::DisputeOpened,
            bounty_id: 123,
            amount: 1_000_000,
            timestamp: 1000,
        };

        let _ = MockHook::handle_hook(env.clone(), call);
        assert_eq!(MockHook::get_call_count(env.clone()), 1);

        MockHook::reset(env.clone());
        assert_eq!(MockHook::get_call_count(env), 0);
    }
}
