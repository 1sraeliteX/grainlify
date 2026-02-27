#![no_std]
//! Minimal Soroban escrow demo: lock, release, and refund.
//! Parity with main contracts/bounty_escrow where applicable; see soroban/PARITY.md.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, Env,
, BytesN};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, BytesN, Env,
    String, Symbol, Vec,
};

mod identity;
pub use identity::*;

mod reentrancy_guard;

#[contracterror]
#[derive(Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    BountyExists = 3,
    BountyNotFound = 4,
    FundsNotLocked = 5,
    DeadlineNotPassed = 6,
    Unauthorized = 7,
    InsufficientBalance = 8,
    ContractDeprecated = 9,
    // Identity-related errors
    InvalidSignature = 100,
    ClaimExpired = 101,
    UnauthorizedIssuer = 102,
    InvalidClaimFormat = 103,
    TransactionExceedsLimit = 104,
    InvalidRiskScore = 105,
    InvalidTier = 106,
    JurisdictionPaused = 107,
    JurisdictionKycRequired = 108,
    JurisdictionAmountExceeded = 109,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Locked,
    Released,
    Refunded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowJurisdictionConfig {
    pub tag: Option<String>,
    pub requires_kyc: bool,
    pub enforce_identity_limits: bool,
    pub lock_paused: bool,
    pub release_paused: bool,
    pub refund_paused: bool,
    pub max_lock_amount: Option<i128>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum OptionalJurisdiction {
    None,
    Some(EscrowJurisdictionConfig),
}


#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub depositor: Address,
    pub amount: i128,
    pub remaining_amount: i128,
    pub status: EscrowStatus,
    pub deadline: u64,
}

/// Search criteria for paginated escrow queries.
/// Status is a u32 code: 0=any, 1=Locked, 2=Released, 3=Refunded.
/// Depositor is optional; `None` means "match any".
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowSearchCriteria {
    pub status_filter: u32,
    pub depositor: Option<Address>,
}

/// A single escrow record in search results (flattened).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowRecord {
    pub bounty_id: u64,
    pub depositor: Address,
    pub amount: i128,
    pub remaining_amount: i128,
    pub status: EscrowStatus,
    pub deadline: u64,
}

/// A single page of escrow search results.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowPage {
    /// Matched escrow records.
    pub records: Vec<EscrowRecord>,
    /// Cursor for the next page (`None` if this is the last page).
    pub next_cursor: Option<u64>,
    /// Whether more results exist beyond this page.
    pub has_more: bool,
    pub jurisdiction: OptionalJurisdiction,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowJurisdictionEvent {
    pub version: u32,
    pub bounty_id: u64,
    pub operation: Symbol,
    pub jurisdiction_tag: Option<String>,
    pub requires_kyc: bool,
    pub enforce_identity_limits: bool,
    pub lock_paused: bool,
    pub release_paused: bool,
    pub refund_paused: bool,
    pub max_lock_amount: Option<i128>,
    pub timestamp: u64,
}

/// Maximum page size for paginated queries.
const MAX_PAGE_SIZE: u32 = 20;
/// Kill-switch state: when deprecated is true, new escrows are blocked; existing can complete or migrate.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeprecationState {
    pub deprecated: bool,
    pub migration_target: Option<Address>,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Escrow(u64),
    /// Jurisdiction config stored separately (avoids Option<ContractType> XDR issue).
    EscrowJurisdiction(u64),
    /// Persistent Vec<u64> index of all bounty IDs.
    EscrowIndex,
    DeprecationState,
    // Identity-related storage keys
    AddressIdentity(Address),
    AuthorizedIssuer(Address),
    TierLimits,
    RiskThresholds,
    ReentrancyGuard,
}

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    fn emit_jurisdiction_event(
        env: &Env,
        bounty_id: u64,
        operation: Symbol,
        jurisdiction: &OptionalJurisdiction,
    ) {
        let (
            jurisdiction_tag,
            requires_kyc,
            enforce_identity_limits,
            lock_paused,
            release_paused,
            refund_paused,
            max_lock_amount,
        ) = if let OptionalJurisdiction::Some(cfg) = jurisdiction {
            (
                cfg.tag.clone(),
                cfg.requires_kyc,
                cfg.enforce_identity_limits,
                cfg.lock_paused,
                cfg.release_paused,
                cfg.refund_paused,
                cfg.max_lock_amount,
            )
        } else {
            (None, false, true, false, false, false, None)
        };

        env.events().publish(
            (symbol_short!("juris"), operation.clone(), bounty_id),
            EscrowJurisdictionEvent {
                version: 2,
                bounty_id,
                operation,
                jurisdiction_tag,
                requires_kyc,
                enforce_identity_limits,
                lock_paused,
                release_paused,
                refund_paused,
                max_lock_amount,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    fn enforce_lock_jurisdiction(
        env: &Env,
        depositor: &Address,
        amount: i128,
        jurisdiction: &OptionalJurisdiction,
    ) -> Result<(), Error> {
        if let OptionalJurisdiction::Some(cfg) = jurisdiction {
            if cfg.lock_paused {
                return Err(Error::JurisdictionPaused);
            }
            if cfg.requires_kyc && !Self::is_claim_valid(env.clone(), depositor.clone()) {
                return Err(Error::JurisdictionKycRequired);
            }
            if let Some(max_lock_amount) = cfg.max_lock_amount {
                if amount > max_lock_amount {
                    return Err(Error::JurisdictionAmountExceeded);
                }
            }
            if cfg.enforce_identity_limits {
                return Self::enforce_transaction_limit(env, depositor, amount);
            }
            return Ok(());
        }

        Self::enforce_transaction_limit(env, depositor, amount)
    }

    fn enforce_release_jurisdiction(
        env: &Env,
        contributor: &Address,
        amount: i128,
        jurisdiction: &OptionalJurisdiction,
    ) -> Result<(), Error> {
        if let OptionalJurisdiction::Some(cfg) = jurisdiction {
            if cfg.release_paused {
                return Err(Error::JurisdictionPaused);
            }
            if cfg.requires_kyc && !Self::is_claim_valid(env.clone(), contributor.clone()) {
                return Err(Error::JurisdictionKycRequired);
            }
            if cfg.enforce_identity_limits {
                return Self::enforce_transaction_limit(env, contributor, amount);
            }
            return Ok(());
        }

        Self::enforce_transaction_limit(env, contributor, amount)
    }

    fn enforce_refund_jurisdiction(
        env: &Env,
        depositor: &Address,
        jurisdiction: &OptionalJurisdiction,
    ) -> Result<(), Error> {
        if let OptionalJurisdiction::Some(cfg) = jurisdiction {
            if cfg.refund_paused {
                return Err(Error::JurisdictionPaused);
            }
            if cfg.requires_kyc && !Self::is_claim_valid(env.clone(), depositor.clone()) {
                return Err(Error::JurisdictionKycRequired);
            }
        }
        Ok(())
    }

    /// Initialize with admin and token. Call once.
    pub fn init(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);

        // Initialize default tier limits and risk thresholds
        let default_limits = TierLimits::default();
        let default_thresholds = RiskThresholds::default();
        env.storage()
            .persistent()
            .set(&DataKey::TierLimits, &default_limits);
        env.storage()
            .persistent()
            .set(&DataKey::RiskThresholds, &default_thresholds);

        Ok(())
    }

    /// Set or update an authorized claim issuer (admin only)
    pub fn set_authorized_issuer(env: Env, issuer: Address, authorized: bool) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&DataKey::AuthorizedIssuer(issuer.clone()), &authorized);

        // Emit event for issuer management
        env.events().publish(
            (soroban_sdk::symbol_short!("issuer"), issuer.clone()),
            if authorized {
                soroban_sdk::symbol_short!("add")
            } else {
                soroban_sdk::symbol_short!("remove")
            },
        );

        Ok(())
    }

    /// Configure tier-based transaction limits (admin only)
    pub fn set_tier_limits(
        env: Env,
        unverified: i128,
        basic: i128,
        verified: i128,
        premium: i128,
    ) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        let limits = TierLimits {
            unverified_limit: unverified,
            basic_limit: basic,
            verified_limit: verified,
            premium_limit: premium,
        };

        env.storage()
            .persistent()
            .set(&DataKey::TierLimits, &limits);
        Ok(())
    }

    /// Configure risk-based adjustments (admin only)
    pub fn set_risk_thresholds(
        env: Env,
        high_risk_threshold: u32,
        high_risk_multiplier: u32,
    ) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        let thresholds = RiskThresholds {
            high_risk_threshold,
            high_risk_multiplier,
        };

        env.storage()
            .persistent()
            .set(&DataKey::RiskThresholds, &thresholds);
        Ok(())
    }

    /// Submit an identity claim for verification and storage
    pub fn submit_identity_claim(
        env: Env,
        claim: IdentityClaim,
        signature: BytesN<64>,
        issuer_pubkey: BytesN<32>,
    ) -> Result<(), Error> {
        // Require authentication from the address in the claim
        claim.address.require_auth();

        // Check if contract is initialized
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        // Validate claim format
        identity::validate_claim(&claim)?;

        // Check if claim has expired
        if identity::is_claim_expired(&env, claim.expiry) {
            env.events().publish(
                (soroban_sdk::symbol_short!("claim"), claim.address.clone()),
                soroban_sdk::symbol_short!("expired"),
            );
            return Err(Error::ClaimExpired);
        }

        // Check if issuer is authorized
        let is_authorized: bool = env
            .storage()
            .persistent()
            .get(&DataKey::AuthorizedIssuer(claim.issuer.clone()))
            .unwrap_or(false);

        if !is_authorized {
            env.events().publish(
                (soroban_sdk::symbol_short!("claim"), claim.address.clone()),
                soroban_sdk::symbol_short!("unauth"),
            );
            return Err(Error::UnauthorizedIssuer);
        }

        // Verify claim signature
        identity::verify_claim_signature(&env, &claim, &signature, &issuer_pubkey)?;

        // Store identity data for the address
        let now = env.ledger().timestamp();
        let identity_data = AddressIdentity {
            tier: claim.tier.clone(),
            risk_score: claim.risk_score,
            expiry: claim.expiry,
            last_updated: now,
        };

        env.storage().persistent().set(
            &DataKey::AddressIdentity(claim.address.clone()),
            &identity_data,
        );

        // Emit event for successful claim submission
        env.events().publish(
            (soroban_sdk::symbol_short!("claim"), claim.address.clone()),
            (claim.tier, claim.risk_score, claim.expiry),
        );

        Ok(())
    }

    /// Query identity data for an address
    pub fn get_address_identity(env: Env, address: Address) -> AddressIdentity {
        let identity: Option<AddressIdentity> = env
            .storage()
            .persistent()
            .get(&DataKey::AddressIdentity(address));

        match identity {
            Some(id) => {
                // Check if claim has expired
                if identity::is_claim_expired(&env, id.expiry) {
                    // Return default unverified tier
                    AddressIdentity::default()
                } else {
                    id
                }
            }
            None => AddressIdentity::default(),
        }
    }

    /// Query effective transaction limit for an address
    pub fn get_effective_limit(env: Env, address: Address) -> i128 {
        let identity = Self::get_address_identity(env.clone(), address);

        let tier_limits: TierLimits = env
            .storage()
            .persistent()
            .get(&DataKey::TierLimits)
            .unwrap_or_default();

        let risk_thresholds: RiskThresholds = env
            .storage()
            .persistent()
            .get(&DataKey::RiskThresholds)
            .unwrap_or_default();

        identity::calculate_effective_limit(&env, &identity, &tier_limits, &risk_thresholds)
    }

    /// Check if an address has a valid (non-expired) claim
    pub fn is_claim_valid(env: Env, address: Address) -> bool {
        let identity: Option<AddressIdentity> = env
            .storage()
            .persistent()
            .get(&DataKey::AddressIdentity(address));

        match identity {
            Some(id) => !identity::is_claim_expired(&env, id.expiry),
            None => false,
        }
    }

    /// Internal: Enforce transaction limit for an address
    fn enforce_transaction_limit(env: &Env, address: &Address, amount: i128) -> Result<(), Error> {
        let effective_limit = Self::get_effective_limit(env.clone(), address.clone());

        if amount > effective_limit {
            // Emit event for limit enforcement failure
            env.events().publish(
                (soroban_sdk::symbol_short!("limit"), address.clone()),
                (
                    soroban_sdk::symbol_short!("exceed"),
                    amount,
                    effective_limit,
                ),
            );
            return Err(Error::TransactionExceedsLimit);
        }

        // Emit event for successful limit check
        env.events().publish(
            (soroban_sdk::symbol_short!("limit"), address.clone()),
            (soroban_sdk::symbol_short!("pass"), amount, effective_limit),
        );

        Ok(())
    }

    /// Lock funds: depositor must be authorized; tokens transferred from depositor to contract.
    /// Fails with ContractDeprecated when the contract has been deprecated (kill switch).
    ///
    /// # Reentrancy
    /// Protected by reentrancy guard. Escrow state is written before the
    /// inbound token transfer (CEI pattern).
    pub fn lock_funds(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), Error> {
        Self::lock_funds_with_jurisdiction(env, depositor, bounty_id, amount, deadline, None)
        Self::lock_funds_with_jurisdiction(env, depositor, bounty_id, amount, deadline, OptionalJurisdiction::None)
    }

    /// Lock funds with optional jurisdiction controls.
    pub fn lock_funds_with_jurisdiction(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
        jurisdiction: OptionalJurisdiction,
    ) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        depositor.require_auth();
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        if amount <= 0 {
            return Err(Error::InsufficientBalance);
        }
        if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyExists);
        }

        Self::enforce_lock_jurisdiction(&env, &depositor, amount, &jurisdiction)?;

        // EFFECTS: write escrow state before external call
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount,
            remaining_amount: amount,
            status: EscrowStatus::Locked,
            deadline,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Store jurisdiction config separately (avoids Option<ContractType> XDR issue)
        if let Some(ref juris) = jurisdiction {
            env.storage()
                .persistent()
                .set(&DataKey::EscrowJurisdiction(bounty_id), juris);
        }

        // Append bounty_id to the global index for paginated queries
        let mut index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or_else(|| Vec::new(&env));
        index.push_back(bounty_id);
        env.storage()
            .persistent()
            .set(&DataKey::EscrowIndex, &index);

        // INTERACTION: external token transfer is last
        let token = env
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::Token)
            .unwrap();
        let contract = env.current_contract_address();
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&depositor, &contract, &amount);

        Self::emit_jurisdiction_event(&env, bounty_id, symbol_short!("lock"), &jurisdiction);

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Release funds to contributor. Admin must be authorized. Fails if already released or refunded.
    ///
    /// # Reentrancy
    /// Protected by reentrancy guard. Escrow state is updated to
    /// `Released` *before* the outbound token transfer (CEI pattern).
    pub fn release_funds(env: Env, bounty_id: u64, contributor: Address) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }
        if escrow.remaining_amount <= 0 {
            return Err(Error::InsufficientBalance);
        }

        let jurisdiction: Option<EscrowJurisdictionConfig> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowJurisdiction(bounty_id));

        Self::enforce_release_jurisdiction(
            &env,
            &contributor,
            escrow.remaining_amount,
            &jurisdiction,
        )?;

        // EFFECTS: update state before external call (CEI)
        let release_amount = escrow.remaining_amount;
        escrow.remaining_amount = 0;
        escrow.status = EscrowStatus::Released;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // INTERACTION: external token transfer is last
        let token = env
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::Token)
            .unwrap();
        let contract = env.current_contract_address();
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&contract, &contributor, &release_amount);

        Self::emit_jurisdiction_event(
            &env,
            bounty_id,
            symbol_short!("release"),
            &jurisdiction,
        );

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Refund remaining funds to depositor. Allowed after deadline.
    ///
    /// # Reentrancy
    /// Protected by reentrancy guard. Escrow state is updated to
    /// `Refunded` *before* the outbound token transfer (CEI pattern).
    pub fn refund(env: Env, bounty_id: u64) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }
        let now = env.ledger().timestamp();
        if now < escrow.deadline {
            return Err(Error::DeadlineNotPassed);
        }
        if escrow.remaining_amount <= 0 {
            return Err(Error::InsufficientBalance);
        }
        let jurisdiction: Option<EscrowJurisdictionConfig> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowJurisdiction(bounty_id));
        Self::enforce_refund_jurisdiction(&env, &escrow.depositor, &jurisdiction)?;

        // EFFECTS: update state before external call (CEI)
        let amount = escrow.remaining_amount;
        let depositor = escrow.depositor.clone();
        escrow.remaining_amount = 0;
        escrow.status = EscrowStatus::Refunded;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // INTERACTION: external token transfer is last
        let token = env
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::Token)
            .unwrap();
        let contract = env.current_contract_address();
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&contract, &depositor, &amount);

        Self::emit_jurisdiction_event(
            &env,
            bounty_id,
            symbol_short!("refund"),
            &jurisdiction,
        );

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Read escrow state (for tests).
    pub fn get_escrow(env: Env, bounty_id: u64) -> Result<Escrow, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .ok_or(Error::BountyNotFound)
    }

    fn get_deprecation_state(env: &Env) -> DeprecationState {
        env.storage()
            .instance()
            .get(&DataKey::DeprecationState)
            .unwrap_or(DeprecationState {
                deprecated: false,
                migration_target: None,
            })
    }

    /// Set deprecation (kill switch) and optional migration target. Admin only.
    /// When deprecated is true, new lock_funds are blocked; release and refund remain allowed.
    pub fn set_deprecated(
        env: Env,
        deprecated: bool,
        migration_target: Option<Address>,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let state = DeprecationState {
            deprecated,
            migration_target: migration_target.clone(),
        };
        env.storage().instance().set(&DataKey::DeprecationState, &state);
        env.events().publish(
            (symbol_short!("deprec"),),
            (state.deprecated, state.migration_target, admin, env.ledger().timestamp()),
        );
        Ok(())
    }

    /// View: returns whether the contract is deprecated and the optional migration target.
    pub fn get_deprecation_status(env: Env) -> DeprecationState {
        Self::get_deprecation_state(&env)
    }

    /// Read jurisdiction configuration for an escrow.
    pub fn get_escrow_jurisdiction(
        env: Env,
        bounty_id: u64,
    ) -> Result<Option<EscrowJurisdictionConfig>, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::EscrowJurisdiction(bounty_id)))
    }

    /// Return the total number of escrows tracked in the index.
    pub fn get_escrow_count(env: Env) -> u32 {
        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or_else(|| Vec::new(&env));
        index.len()
    }

    /// Paginated search over escrows.
    ///
    /// * `criteria` – `status_filter`: 0=any, 1=Locked, 2=Released, 3=Refunded.
    ///                `depositor`: optional address filter.
    /// * `cursor`   – pass the `next_cursor` from a previous `EscrowPage` to continue;
    ///                `None` starts from the beginning.
    /// * `limit`    – max results per page (capped at `MAX_PAGE_SIZE`).
    ///
    /// Returns an `EscrowPage` with matching records, the next cursor, and a
    /// `has_more` flag.
    pub fn get_escrows(
        env: Env,
        criteria: EscrowSearchCriteria,
        cursor: Option<u64>,
        limit: u32,
    ) -> EscrowPage {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_SIZE {
            MAX_PAGE_SIZE
        } else {
            limit
        };

        // Convert u32 status code to EscrowStatus for matching
        let status_match = match criteria.status_filter {
            1 => Some(EscrowStatus::Locked),
            2 => Some(EscrowStatus::Released),
            3 => Some(EscrowStatus::Refunded),
            _ => None, // 0 or anything else = match any
        };

        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or_else(|| Vec::new(&env));

        let mut records: Vec<EscrowRecord> = Vec::new(&env);
        let mut past_cursor = cursor.is_none();
        let mut next_cursor: Option<u64> = None;
        let mut has_more = false;

        for i in 0..index.len() {
            let id = index.get(i).unwrap();

            // Skip until we pass the cursor
            if !past_cursor {
                if Some(id) == cursor {
                    past_cursor = true;
                }
                continue;
            }

            // Fetch the escrow record
            let escrow_opt: Option<Escrow> = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(id));
            if escrow_opt.is_none() {
                continue;
            }
            let escrow = escrow_opt.unwrap();

            // Apply status filter
            if let Some(ref status) = status_match {
                if escrow.status != *status {
                    continue;
                }
            }

            // Apply depositor filter
            if let Some(ref depositor) = criteria.depositor {
                if escrow.depositor != *depositor {
                    continue;
                }
            }

            // Check if we already have enough results
            if records.len() >= effective_limit {
                has_more = true;
                break;
            }

            next_cursor = Some(id);
            records.push_back(EscrowRecord {
                bounty_id: id,
                depositor: escrow.depositor,
                amount: escrow.amount,
                remaining_amount: escrow.remaining_amount,
                status: escrow.status,
                deadline: escrow.deadline,
            });
        }

        if !has_more {
            next_cursor = None;
        }

        EscrowPage {
            records,
            next_cursor,
            has_more,
        }
    ) -> Result<OptionalJurisdiction, Error> {
        let escrow = Self::get_escrow(env, bounty_id)?;
        Ok(escrow.jurisdiction)
    }
}

mod identity_test;
mod test;
mod test_search;
