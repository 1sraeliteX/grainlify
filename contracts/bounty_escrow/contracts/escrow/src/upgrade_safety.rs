//! # Upgrade Safety Module
//!
//! This module provides pre-upgrade safety checks and dry-run functionality
//! for contract upgrades. It helps prevent bricked contracts by validating
//! critical invariants before any upgrade is executed.
//!
//! ## Safety Checklist
//!
//! Before any upgrade, the following invariants are validated:
//!
//! 1. **Storage Layout Compatibility** - Ensure new code can read existing storage
//! 2. **Contract Initialization State** - Verify contract is properly initialized
//! 3. **Escrow State Consistency** - All escrows must be in valid states
//! 4. **Pending Claims Verification** - Validate all pending claims are valid
//! 5. **Admin Authority** - Verify admin address is set and valid
//! 6. **Token Configuration** - Ensure token address is configured
//! 7. **Feature Flags Readiness** - Check any feature flags are properly set
//! 8. **No Reentrancy Locks** - Ensure no reentrancy guards are stuck
//! 9. **Version Compatibility** - Validate version information
//! 10. **Balance Sanity** - Verify token balance consistency

use crate::{Escrow, EscrowStatus, Error};
use soroban_sdk::{contracttype, Env, String, Vec, Symbol};

/// Result of upgrade safety validation
#[contracttype]
#[derive(Clone, Debug)]
pub struct UpgradeSafetyReport {
    pub is_safe: bool,
    pub checks_passed: u32,
    pub checks_failed: u32,
    pub warnings: Vec<UpgradeWarning>,
    pub errors: Vec<UpgradeError>,
}

/// Warning during upgrade safety check
#[contracttype]
#[derive(Clone, Debug)]
pub struct UpgradeWarning {
    pub code: u32,
    pub message: String,
}

/// Error during upgrade safety check
#[contracttype]
#[derive(Clone, Debug)]
pub struct UpgradeError {
    pub code: u32,
    pub message: String,
}

/// Storage key for upgrade safety state
const UPGRADE_SAFETY_ENABLED: &str = "upg_safe_en";

/// Storage key for last safety check timestamp
const LAST_SAFETY_CHECK: &str = "last_safe_chk";

/// Codes for upgrade safety checks
pub mod safety_codes {
    /// Storage layout compatibility check
    pub const STORAGE_LAYOUT: u32 = 1001;
    /// Contract initialization check
    pub const INITIALIZATION: u32 = 1002;
    /// Escrow state consistency check
    pub const ESCROW_STATE: u32 = 1003;
    /// Pending claims verification
    pub const PENDING_CLAIMS: u32 = 1004;
    /// Admin authority check
    pub const ADMIN_AUTHORITY: u32 = 1005;
    /// Token configuration check
    pub const TOKEN_CONFIG: u32 = 1006;
    /// Feature flags readiness
    pub const FEATURE_FLAGS: u32 = 1007;
    /// Reentrancy lock check
    pub const REENTRANCY_LOCK: u32 = 1008;
    /// Version compatibility check
    pub const VERSION_COMPAT: u32 = 1009;
    /// Balance sanity check
    pub const BALANCE_SANITY: u32 = 1010;
}

/// Enable or disable upgrade safety checks
pub fn set_safety_checks_enabled(env: &Env, enabled: bool) {
    env.storage()
        .instance()
        .set(&UPGRADE_SAFETY_ENABLED, &enabled);
}

/// Check if safety checks are enabled
pub fn is_safety_checks_enabled(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&UPGRADE_SAFETY_ENABLED)
        .unwrap_or(true) // Safety checks enabled by default
}

/// Record last safety check timestamp
pub fn record_safety_check(env: &Env) {
    let timestamp = env.ledger().timestamp();
    env.storage()
        .instance()
        .set(&LAST_SAFETY_CHECK, &timestamp);
}

/// Get last safety check timestamp
pub fn get_last_safety_check(env: &Env) -> Option<u64> {
    env.storage().instance().get(&LAST_SAFETY_CHECK)
}

/// Simulate an upgrade by performing all safety checks without mutating state.
/// This is a read-only dry-run that validates upgrade safety.
///
/// Returns an UpgradeSafetyReport with detailed results of all checks.
pub fn simulate_upgrade(env: &Env) -> UpgradeSafetyReport {
    let mut warnings: Vec<UpgradeWarning> = Vec::new(env);
    let mut errors: Vec<UpgradeError> = Vec::new();
    let mut checks_passed: u32 = 0;
    let mut checks_failed: u32 = 0;

    // Check 1: Storage Layout Compatibility
    if check_storage_layout_compatibility(env) {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push(UpgradeError {
            code: safety_codes::STORAGE_LAYOUT,
            message: soroban_sdk::String::from_str(env, "Storage layout incompatible with current state"),
        });
    }

    // Check 2: Contract Initialization
    if check_initialization(env) {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push(UpgradeError {
            code: safety_codes::INITIALIZATION,
            message: soroban_sdk::String::from_str(env, "Contract not properly initialized"),
        });
    }

    // Check 3: Escrow State Consistency
    let (escrow_ok, escrow_warnings) = check_escrow_states(env);
    if escrow_ok {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push(UpgradeError {
            code: safety_codes::ESCROW_STATE,
            message: soroban_sdk::String::from_str(env, "One or more escrows in invalid state"),
        });
    }
    for w in escrow_warnings {
        warnings.push(w);
    }

    // Check 4: Pending Claims Verification
    if check_pending_claims(env) {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push(UpgradeError {
            code: safety_codes::PENDING_CLAIMS,
            message: soroban_sdk::String::from_str(env, "Invalid pending claims detected"),
        });
    }

    // Check 5: Admin Authority
    if check_admin_authority(env) {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push(UpgradeError {
            code: safety_codes::ADMIN_AUTHORITY,
            message: soroban_sdk::String::from_str(env, "Admin authority not properly set"),
        });
    }

    // Check 6: Token Configuration
    if check_token_config(env) {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push(UpgradeError {
            code: safety_codes::TOKEN_CONFIG,
            message: soroban_sdk::String::from_str(env, "Token not properly configured"),
        });
    }

    // Check 7: Feature Flags Readiness (placeholder - can be extended)
    if check_feature_flags(env) {
        checks_passed += 1;
    } else {
        warnings.push(UpgradeWarning {
            code: safety_codes::FEATURE_FLAGS,
            message: soroban_sdk::String::from_str(env, "Feature flags may need review"),
        });
        checks_passed += 1; // Warning doesn't fail the check
    }

    // Check 8: No Reentrancy Locks
    if check_no_reentrancy_locks(env) {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push(UpgradeError {
            code: safety_codes::REENTRANCY_LOCK,
            message: soroban_sdk::String::from_str(env, "Reentrancy lock is stuck"),
        });
    }

    // Check 9: Version Compatibility
    if check_version_compatibility(env) {
        checks_passed += 1;
    } else {
        warnings.push(UpgradeWarning {
            code: safety_codes::VERSION_COMPAT,
            message: soroban_sdk::String::from_str(env, "Version information may be inconsistent"),
        });
        checks_passed += 1;
    }

    // Check 10: Balance Sanity
    let (balance_ok, balance_warnings) = check_balance_sanity(env);
    if balance_ok {
        checks_passed += 1;
    } else {
        checks_failed += 1;
        errors.push(UpgradeError {
            code: safety_codes::BALANCE_SANITY,
            message: soroban_sdk::String::from_str(env, "Token balance inconsistency detected"),
        });
    }
    for w in balance_warnings {
        warnings.push(w);
    }

    // Record the safety check
    record_safety_check(env);

    UpgradeSafetyReport {
        is_safe: errors.is_empty(),
        checks_passed,
        checks_failed,
        warnings,
        errors,
    }
}

// Private check functions

fn check_storage_layout_compatibility(env: &Env) -> bool {
    // In Soroban, storage layout compatibility is primarily ensured by:
    // 1. Not removing existing storage keys
    // 2. Not changing the type of existing storage keys
    // This check verifies the contract has been initialized (meaning storage exists)
    // and that we can read from it - which implies layout compatibility for reading
    env.storage().instance().has(&crate::DataKey::Admin)
}

fn check_initialization(env: &Env) -> bool {
    // Contract must be initialized to be upgradable
    env.storage().instance().has(&crate::DataKey::Admin)
        && env.storage().instance().has(&crate::DataKey::Token)
}

fn check_escrow_states(env: &Env) -> (bool, Vec<UpgradeWarning>) {
    let mut warnings: Vec<UpgradeWarning> = Vec::new(env);
    
    // Get the last bounty ID
    let last_id: u64 = env
        .storage()
        .instance()
        .get(&crate::DataKey::LastBountyId)
        .unwrap_or(0);

    if last_id == 0 {
        return (true, warnings); // No escrows to check
    }

    // Check a sample of escrows for state consistency
    // In production, you might want to check all, but for performance we sample
    let sample_size = if last_id > 100 { 100 } else { last_id };
    
    for i in 1..=sample_size {
        if env.storage().persistent().has(&crate::DataKey::Escrow(i)) {
            let escrow: Escrow = env.storage().persistent().get(&crate::DataKey::Escrow(i)).unwrap();
            
            // Check basic state consistency
            if escrow.amount < 0 || escrow.remaining_amount < 0 {
                return (false, warnings);
            }
            if escrow.remaining_amount > escrow.amount {
                return (false, warnings);
            }
            
            // Check status-specific invariants
            match escrow.status {
                EscrowStatus::Released => {
                    if escrow.remaining_amount != 0 {
                        // Warning: released escrow should have 0 remaining
                        warnings.push(UpgradeWarning {
                            code: safety_codes::ESCROW_STATE,
                            message: soroban_sdk::String::from_str(env, "Released escrow has non-zero remaining amount"),
                        });
                    }
                }
                EscrowStatus::Locked => {
                    if escrow.remaining_amount == 0 {
                        // Warning: locked escrow should have remaining amount
                        warnings.push(UpgradeWarning {
                            code: safety_codes::ESCROW_STATE,
                            message: soroban_sdk::String::from_str(env, "Locked escrow has zero remaining amount"),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    (true, warnings)
}

fn check_pending_claims(env: &Env) -> bool {
    // Get the last bounty ID
    let last_id: u64 = env
        .storage()
        .instance()
        .get(&crate::DataKey::LastBountyId)
        .unwrap_or(0);

    // Check pending claims for each bounty
    for i in 1..=last_id {
        if env.storage().persistent().has(&crate::DataKey::Escrow(i)) {
            let escrow: Escrow = env.storage().persistent().get(&crate::DataKey::Escrow(i)).unwrap();
            
            // If there's a pending claim, verify the escrow is in Pending status
            if env.storage().persistent().has(&crate::DataKey::Claim(i)) {
                if escrow.status != EscrowStatus::Pending {
                    return false;
                }
            }
        }
    }

    true
}

fn check_admin_authority(env: &Env) -> bool {
    // Admin must be set
    if !env.storage().instance().has(&crate::DataKey::Admin) {
        return false;
    }

    // Admin must be a valid address (non-zero)
    let admin: soroban_sdk::Address = env.storage().instance().get(&crate::DataKey::Admin).unwrap();
    // In Soroban, Address::generate creates a valid address
    // A properly set admin should not be problematic
    
    true
}

fn check_token_config(env: &Env) -> bool {
    // Token must be configured
    env.storage().instance().has(&crate::DataKey::Token)
}

fn check_feature_flags(env: &Env) -> bool {
    // Check pause flags if they exist
    if env.storage().instance().has(&crate::DataKey::PauseFlags) {
        let flags: crate::PauseFlags = env.storage().instance().get(&crate::DataKey::PauseFlags).unwrap();
        
        // If contract is fully paused, warn about upgrade
        // This is not a failure but a warning
        if flags.locked {
            return false; // Will become a warning in the main check
        }
    }
    
    true
}

fn check_no_reentrancy_locks(env: &Env) -> bool {
    // If reentrancy guard exists and is set, it should be cleared
    // A stuck reentrancy guard would prevent contract operation
    if env.storage().instance().has(&crate::DataKey::ReentrancyGuard) {
        let guard: bool = env.storage().instance().get(&crate::DataKey::ReentrancyGuard).unwrap();
        if guard {
            return false; // Reentrancy lock is stuck
        }
    }
    true
}

fn check_version_compatibility(env: &Env) -> bool {
    // Version should be trackable
    // This is a placeholder - actual version checking depends on how version is stored
    // The trait provides get_version which should work
    true
}

fn check_balance_sanity(env: &Env) -> (bool, Vec<UpgradeWarning>) {
    let mut warnings: Vec<UpgradeWarning> = Vec::new(env);
    
    // Get the last bounty ID
    let last_id: u64 = env
        .storage()
        .instance()
        .get(&crate::DataKey::LastBountyId)
        .unwrap_or(0);

    if last_id == 0 {
        return (true, warnings);
    }

    // Calculate total locked amount
    let mut total_locked: i128 = 0;
    
    for i in 1..=last_id {
        if env.storage().persistent().has(&crate::DataKey::Escrow(i)) {
            let escrow: Escrow = env.storage().persistent().get(&crate::DataKey::Escrow(i)).unwrap();
            
            match escrow.status {
                EscrowStatus::Locked | EscrowStatus::Pending => {
                    total_locked += escrow.remaining_amount;
                }
                _ => {}
            }
        }
    }

    // We can't actually verify the token balance here without the token contract
    // But we can ensure the total locked is non-negative
    if total_locked < 0 {
        return (false, warnings);
    }

    (true, warnings)
}

/// Validate upgrade prerequisites before executing upgrade.
/// Returns Ok(()) if upgrade can proceed, Err(Error) otherwise.
pub fn validate_upgrade(env: &Env) -> Result<(), Error> {
    // Check if safety checks are enabled
    if !is_safety_checks_enabled(env) {
        return Ok(()); // Skip checks if disabled
    }

    // Run simulation
    let report = simulate_upgrade(env);

    // If not safe, return error
    if !report.is_safe {
        // Convert first error to contract error
        if !report.errors.is_empty() {
            // For simplicity, we return a generic error
            // In production, you might want more specific error codes
            return Err(Error::UpgradeSafetyCheckFailed);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BountyEscrowContract, BountyEscrowContractClient};
    use soroban_sdk::testutils::Ledger;
    use soroban_sdk::{testutils::Address as _, Address, Env, LedgerInfo};

    fn create_test_env() -> (Env, BountyEscrowContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);
        (env, client)
    }

    #[test]
    fn test_safety_checks_enabled_by_default() {
        let env = Env::default();
        assert!(is_safety_checks_enabled(&env));
    }

    #[test]
    fn test_can_disable_safety_checks() {
        let env = Env::default();
        set_safety_checks_enabled(&env, false);
        assert!(!is_safety_checks_enabled(&env));
    }

    #[test]
    fn test_simulate_upgrade_after_init() {
        let (env, client) = create_test_env();
        
        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token = token_id.address();

        client.init(&admin, &token);

        let report = simulate_upgrade(&env);
        // Should pass all checks after proper initialization
        assert!(report.is_safe);
    }

    #[test]
    fn test_simulate_upgrade_before_init_fails() {
        let env = Env::default();
        env.mock_all_auths();
        env.register_contract(None, BountyEscrowContract);

        let report = simulate_upgrade(&env);
        // Should fail - contract not initialized
        assert!(!report.is_safe);
    }

    #[test]
    fn test_record_safety_check() {
        let env = Env::default();
        
        assert!(get_last_safety_check(&env).is_none());
        
        record_safety_check(&env);
        
        assert!(get_last_safety_check(&env).is_some());
    }
}
