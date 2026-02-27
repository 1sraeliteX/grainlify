#![no_std]
use soroban_sdk::{
<<<<<<< HEAD
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, Env, String,
    Vec,
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, Env, String,
    Vec,
=======
    contract, contracterror, contractimpl, contracttype, symbol_short, symbol_short, token, Address, Env,
    String,
    Vec,
>>>>>>> upstream
};

const MAX_BATCH_SIZE: u32 = 20;
const PROGRAM_REGISTERED: soroban_sdk::Symbol = symbol_short!("prg_reg");

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    ProgramExists = 3,
    ProgramNotFound = 4,
    Unauthorized = 5,
    InvalidBatchSize = 6,
    DuplicateProgramId = 7,
    InvalidAmount = 8,
    InvalidName = 9,
    ContractDeprecated = 10,
    JurisdictionKycRequired = 10,
    JurisdictionFundingLimitExceeded = 11,
    JurisdictionPaused = 12,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgramStatus {
    Active,
    Completed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub admin: Address,
    pub name: String,
    pub total_funding: i128,
    pub status: ProgramStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramJurisdictionConfig {
    pub tag: Option<String>,
    pub requires_kyc: bool,
    pub max_funding: Option<i128>,
    pub registration_paused: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum OptionalJurisdiction {
    None,
    Some(ProgramJurisdictionConfig),
}


#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub admin: Address,
    pub name: String,
    pub total_funding: i128,
    pub status: ProgramStatus,
    pub jurisdiction: OptionalJurisdiction,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramRegistrationItem {
    pub program_id: u64,
    pub admin: Address,
    pub name: String,
    pub total_funding: i128,
}

/// Kill-switch state: when deprecated is true, new program registrations are blocked.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeprecationState {
    pub deprecated: bool,
    pub migration_target: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramRegistrationWithJurisdictionItem {
    pub program_id: u64,
    pub admin: Address,
    pub name: String,
    pub total_funding: i128,
    pub juris_tag: Option<String>,
    pub juris_requires_kyc: bool,
    pub juris_max_funding: Option<i128>,
    pub juris_registration_paused: bool,
    pub jurisdiction: OptionalJurisdiction,
    pub kyc_attested: Option<bool>,
} 

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramRegisteredEvent {
    pub version: u32,
    pub program_id: u64,
    pub admin: Address,
    pub total_funding: i128,
    pub jurisdiction_tag: Option<String>,
    pub requires_kyc: bool,
    pub max_funding: Option<i128>,
    pub registration_paused: bool,
    pub timestamp: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Program(u64),
    /// Jurisdiction config stored separately (avoids Option<ContractType> XDR issue).
    ProgramJurisdiction(u64),
    /// Persistent Vec<u64> index of all program IDs.
    ProgramIndex,
}

/// Maximum page size for paginated queries.
const MAX_PAGE_SIZE: u32 = 20;

/// Search criteria for paginated program queries.
/// Status is a u32 code: 0=any, 1=Active, 2=Completed, 3=Cancelled.
/// Admin is optional; `None` means "match any".
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramSearchCriteria {
    pub status_filter: u32,
    pub admin: Option<Address>,
}

/// A single program record in search results (flattened).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramRecord {
    pub program_id: u64,
    pub admin: Address,
    pub name: String,
    pub total_funding: i128,
    pub status: ProgramStatus,
}

/// A single page of program search results.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramPage {
    /// Matched program records.
    pub records: Vec<ProgramRecord>,
    /// Cursor for the next page (`None` if this is the last page).
    pub next_cursor: Option<u64>,
    /// Whether more results exist beyond this page.
    pub has_more: bool,
    DeprecationState,
}

#[contract]
pub struct ProgramEscrowContract;

#[contractimpl]
impl ProgramEscrowContract {
    fn validate_program_input(name: &String, total_funding: i128) -> Result<(), Error> {
        if total_funding <= 0 {
            return Err(Error::InvalidAmount);
        }
        if name.len() == 0 {
            return Err(Error::InvalidName);
        }
        Ok(())
    }

    fn enforce_jurisdiction_rules(
        jurisdiction: &OptionalJurisdiction,
        total_funding: i128,
        kyc_attested: Option<bool>,
    ) -> Result<(), Error> {
        if let OptionalJurisdiction::Some(config) = jurisdiction {
            if config.registration_paused {
                return Err(Error::JurisdictionPaused);
            }

            if let Some(max_funding) = config.max_funding {
                if total_funding > max_funding {
                    return Err(Error::JurisdictionFundingLimitExceeded);
                }
            }

            if config.requires_kyc && !kyc_attested.unwrap_or(false) {
                return Err(Error::JurisdictionKycRequired);
            }
        }
        Ok(())
    }

    fn emit_program_registered(
        env: &Env,
        program_id: u64,
        admin: Address,
        total_funding: i128,
        jurisdiction: &OptionalJurisdiction,
    ) {
        let (jurisdiction_tag, requires_kyc, max_funding, registration_paused) =
            if let OptionalJurisdiction::Some(config) = jurisdiction {
                (
                    config.tag.clone(),
                    config.requires_kyc,
                    config.max_funding,
                    config.registration_paused,
                )
            } else {
                (None, false, None, false)
            };

        env.events().publish(
            (PROGRAM_REGISTERED, program_id),
            ProgramRegisteredEvent {
                version: 2,
                program_id,
                admin,
                total_funding,
                jurisdiction_tag,
                requires_kyc,
                max_funding,
                registration_paused,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    /// Initialize the contract with an admin and token address. Call once.
    pub fn init(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        Ok(())
    }

    /// Register a single program. Fails with ContractDeprecated when the contract has been deprecated.
    pub fn register_program(
        env: Env,
        program_id: u64,
        admin: Address,
        name: String,
        total_funding: i128,
    ) -> Result<(), Error> {
        Self::register_program_juris(
        Self::register_prog_w_juris(
            env,
            program_id,
            admin,
            name,
            total_funding,
            None,
            false,
            None,
            false,
            OptionalJurisdiction::None,
            None,
        )
    }

    /// Register a single program with optional jurisdiction controls.
    pub fn register_program_juris(
    pub fn register_prog_w_juris(
        env: Env,
        program_id: u64,
        admin: Address,
        name: String,
        total_funding: i128,
        juris_tag: Option<String>,
        juris_requires_kyc: bool,
        juris_max_funding: Option<i128>,
        juris_registration_paused: bool,
        jurisdiction: OptionalJurisdiction,
        kyc_attested: Option<bool>,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        if Self::get_deprecation_state(&env).deprecated {
            return Err(Error::ContractDeprecated);
        }
        let contract_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        contract_admin.require_auth();

        if env
            .storage()
            .persistent()
            .has(&DataKey::Program(program_id))
        {
            return Err(Error::ProgramExists);
        }

        Self::validate_program_input(&name, total_funding)?;
        
        let has_juris = juris_tag.is_some() || juris_requires_kyc || juris_max_funding.is_some() || juris_registration_paused;
        let jurisdiction = if has_juris {
            Some(ProgramJurisdictionConfig {
                tag: juris_tag.clone(),
                requires_kyc: juris_requires_kyc,
                max_funding: juris_max_funding.clone(),
                registration_paused: juris_registration_paused,
            })
        } else {
            None
        };
        
        Self::enforce_jurisdiction_rules(&jurisdiction, total_funding, kyc_attested)?;

        // Transfer funding from the program admin to the contract
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        admin.require_auth();
        token_client.transfer(&admin, &env.current_contract_address(), &total_funding);

        let program = Program {
            admin: admin.clone(),
            name,
            total_funding,
            status: ProgramStatus::Active,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Program(program_id), &program);

        // Store jurisdiction config separately
        if let Some(ref juris) = jurisdiction {
            env.storage()
                .persistent()
                .set(&DataKey::ProgramJurisdiction(program_id), juris);
        }

        // Append program_id to the global index for paginated queries
        let mut index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::ProgramIndex)
            .unwrap_or_else(|| Vec::new(&env));
        index.push_back(program_id);
        env.storage()
            .persistent()
            .set(&DataKey::ProgramIndex, &index);

        Self::emit_program_registered(&env, program_id, admin, total_funding, &jurisdiction);
        Ok(())
    }

    /// Batch register multiple programs in a single transaction.
    ///
    /// This operation is atomic — if any item fails validation, the entire
    /// batch is rejected and no programs are registered.
    ///
    /// # Errors
    /// * `InvalidBatchSize` — batch is empty or exceeds `MAX_BATCH_SIZE`
    /// * `ProgramExists` — a program_id already exists in storage
    /// * `DuplicateProgramId` — duplicate program_ids within the batch
    /// * `InvalidAmount` — zero or negative funding amount
    /// * `InvalidName` — empty program name
    /// * `NotInitialized` — contract has not been initialized
    pub fn batch_register_programs(
        env: Env,
        items: Vec<ProgramRegistrationItem>,
    ) -> Result<u32, Error> {
        let batch_size = items.len() as u32;
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        if Self::get_deprecation_state(&env).deprecated {
            return Err(Error::ContractDeprecated);
        }
        let contract_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        contract_admin.require_auth();

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        let contract_address = env.current_contract_address();

        // --- Validation pass (all-or-nothing) ---
        for item in items.iter() {
            if env
                .storage()
                .persistent()
                .has(&DataKey::Program(item.program_id))
            {
                return Err(Error::ProgramExists);
            }
            Self::validate_program_input(&item.name, item.total_funding)?;

            // Detect duplicate program_ids within the batch
            let mut count = 0u32;
            for other in items.iter() {
                if other.program_id == item.program_id {
                    count += 1;
                }
            }
            if count > 1 {
                return Err(Error::DuplicateProgramId);
            }
        }

        // Collect unique admins and require auth once per admin
        let mut seen_admins: Vec<Address> = Vec::new(&env);
        for item in items.iter() {
            let mut found = false;
            for seen in seen_admins.iter() {
                if seen == item.admin {
                    found = true;
                    break;
                }
            }
            if !found {
                seen_admins.push_back(item.admin.clone());
                item.admin.require_auth();
            }
        }

        // --- Processing pass (atomic) ---
        let mut registered_count = 0u32;
        for item in items.iter() {
            token_client.transfer(&item.admin, &contract_address, &item.total_funding);

            let program = Program {
                admin: item.admin.clone(),
                name: item.name.clone(),
                total_funding: item.total_funding,
                status: ProgramStatus::Active,
                jurisdiction: OptionalJurisdiction::None,
            };
            env.storage()
                .persistent()
                .set(&DataKey::Program(item.program_id), &program);

            // Append to the global index
            let mut index: Vec<u64> = env
                .storage()
                .persistent()
                .get(&DataKey::ProgramIndex)
                .unwrap_or_else(|| Vec::new(&env));
            index.push_back(item.program_id);
            env.storage()
                .persistent()
                .set(&DataKey::ProgramIndex, &index);

            Self::emit_program_registered(
                &env,
                item.program_id,
                item.admin.clone(),
                item.total_funding,
                &OptionalJurisdiction::None,
            );
            registered_count += 1;
        }

        Ok(registered_count)
    }

    /// Batch register programs with optional jurisdiction controls.
    pub fn batch_register_juris(
    pub fn batch_reg_progs_w_juris(
        env: Env,
        items: Vec<ProgramRegistrationWithJurisdictionItem>,
    ) -> Result<u32, Error> {
        let batch_size = items.len() as u32;
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let contract_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        contract_admin.require_auth();

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        let contract_address = env.current_contract_address();

        for item in items.iter() {
            if env
                .storage()
                .persistent()
                .has(&DataKey::Program(item.program_id))
            {
                return Err(Error::ProgramExists);
            }
            Self::validate_program_input(&item.name, item.total_funding)?;
            let has_juris = item.juris_tag.is_some()
                || item.juris_requires_kyc
                || item.juris_max_funding.is_some()
                || item.juris_registration_paused;
            let item_jurisdiction = if has_juris {
                Some(ProgramJurisdictionConfig {
                    tag: item.juris_tag.clone(),
                    requires_kyc: item.juris_requires_kyc,
                    max_funding: item.juris_max_funding.clone(),
                    registration_paused: item.juris_registration_paused,
                })
            } else {
                None
            };
            
            Self::enforce_jurisdiction_rules(
                &item_jurisdiction,
                item.total_funding,
                item.kyc_attested,
            )?;

            let mut count = 0u32;
            for other in items.iter() {
                if other.program_id == item.program_id {
                    count += 1;
                }
            }
            if count > 1 {
                return Err(Error::DuplicateProgramId);
            }
        }

        let mut seen_admins: Vec<Address> = Vec::new(&env);
        for item in items.iter() {
            let mut found = false;
            for seen in seen_admins.iter() {
                if seen == item.admin {
                    found = true;
                    break;
                }
            }
            if !found {
                seen_admins.push_back(item.admin.clone());
                item.admin.require_auth();
            }
        }

        let mut registered_count = 0u32;
        for item in items.iter() {
            token_client.transfer(&item.admin, &contract_address, &item.total_funding);

            let program = Program {
                admin: item.admin.clone(),
                name: item.name.clone(),
                total_funding: item.total_funding,
                status: ProgramStatus::Active,
            };
            env.storage()
                .persistent()
                .set(&DataKey::Program(item.program_id), &program);

            let has_juris = item.juris_tag.is_some()
                || item.juris_requires_kyc
                || item.juris_max_funding.is_some()
                || item.juris_registration_paused;
            let item_jurisdiction = if has_juris {
                Some(ProgramJurisdictionConfig {
                    tag: item.juris_tag.clone(),
                    requires_kyc: item.juris_requires_kyc,
                    max_funding: item.juris_max_funding.clone(),
                    registration_paused: item.juris_registration_paused,
                })
            } else {
                None
            };

            if let Some(ref juris) = item_jurisdiction {
                env.storage()
                    .persistent()
                    .set(&DataKey::ProgramJurisdiction(item.program_id), juris);
            }

            // Append to the global index
            let mut idx: Vec<u64> = env
                .storage()
                .persistent()
                .get(&DataKey::ProgramIndex)
                .unwrap_or_else(|| Vec::new(&env));
            idx.push_back(item.program_id);
            env.storage()
                .persistent()
                .set(&DataKey::ProgramIndex, &idx);

            Self::emit_program_registered(
                &env,
                item.program_id,
                item.admin.clone(),
                item.total_funding,
                &item_jurisdiction,
            );

            registered_count += 1;
        }

        Ok(registered_count)
    }

    /// Read a program's state.
    pub fn get_program(env: Env, program_id: u64) -> Result<Program, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Program(program_id))
            .ok_or(Error::ProgramNotFound)
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
    /// When deprecated is true, new register_program and batch_register_programs are blocked.
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
        env.storage()
            .instance()
            .set(&DataKey::DeprecationState, &state);
        env.events().publish(
            (symbol_short!("deprec"),),
            (
                state.deprecated,
                state.migration_target,
                admin,
                env.ledger().timestamp(),
            ),
        );
        Ok(())
    }

    /// View: returns whether the contract is deprecated and the optional migration target.
    pub fn get_deprecation_status(env: Env) -> DeprecationState {
        Self::get_deprecation_state(&env)
    }

    /// Read jurisdiction configuration for a program.
    pub fn get_program_jurisdiction(
        env: Env,
        program_id: u64,
    ) -> Result<Option<ProgramJurisdictionConfig>, Error> {
        if !env.storage().persistent().has(&DataKey::Program(program_id)) {
            return Err(Error::ProgramNotFound);
        }
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::ProgramJurisdiction(program_id)))
    }

    /// Return the total number of programs tracked in the index.
    pub fn get_program_count(env: Env) -> u32 {
        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::ProgramIndex)
            .unwrap_or_else(|| Vec::new(&env));
        index.len()
    }

    /// Paginated search over programs.
    ///
    /// * `criteria` – `status_filter`: 0=any, 1=Active, 2=Completed, 3=Cancelled.
    ///                `admin`: optional address filter.
    /// * `cursor`   – pass the `next_cursor` from a previous `ProgramPage` to continue;
    ///                `None` starts from the beginning.
    /// * `limit`    – max results per page (capped at `MAX_PAGE_SIZE`).
    ///
    /// Returns a `ProgramPage` with matching records, the next cursor, and a
    /// `has_more` flag.
    pub fn get_programs(
        env: Env,
        criteria: ProgramSearchCriteria,
        cursor: Option<u64>,
        limit: u32,
    ) -> ProgramPage {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_SIZE {
            MAX_PAGE_SIZE
        } else {
            limit
        };

        // Convert u32 status code to ProgramStatus for matching
        let status_match = match criteria.status_filter {
            1 => Some(ProgramStatus::Active),
            2 => Some(ProgramStatus::Completed),
            3 => Some(ProgramStatus::Cancelled),
            _ => None, // 0 or anything else = match any
        };

        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::ProgramIndex)
            .unwrap_or_else(|| Vec::new(&env));

        let mut records: Vec<ProgramRecord> = Vec::new(&env);
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

            // Fetch the program record
            let program_opt: Option<Program> = env
                .storage()
                .persistent()
                .get(&DataKey::Program(id));
            if program_opt.is_none() {
                continue;
            }
            let program = program_opt.unwrap();

            // Apply status filter
            if let Some(ref status) = status_match {
                if program.status != *status {
                    continue;
                }
            }

            // Apply admin filter
            if let Some(ref admin) = criteria.admin {
                if program.admin != *admin {
                    continue;
                }
            }

            // Check if we already have enough results
            if records.len() >= effective_limit {
                has_more = true;
                break;
            }

            next_cursor = Some(id);
            records.push_back(ProgramRecord {
                program_id: id,
                admin: program.admin,
                name: program.name,
                total_funding: program.total_funding,
                status: program.status,
            });
        }

        if !has_more {
            next_cursor = None;
        }

        ProgramPage {
            records,
            next_cursor,
            has_more,
        }
    ) -> Result<OptionalJurisdiction, Error> {
        let program = Self::get_program(env, program_id)?;
        Ok(program.jurisdiction)
    }
}

mod test;
mod test_search;
