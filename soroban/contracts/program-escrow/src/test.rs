#![cfg(test)]

extern crate std;
use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{testutils::Events, TryFromVal, token, vec, Address, Env, String, Symbol};

/// Sets up a test environment with contract, token, admin, and program_admin.
/// Expands variables directly into the calling scope to avoid lifetime issues.
macro_rules! setup {
    ($env:ident, $client:ident, $contract_id:ident, $admin:ident,
     $program_admin:ident, $token_client:ident, $token_admin:ident,
     $initial_balance:expr) => {
        let $env = Env::default();
        $env.mock_all_auths();

        let $contract_id = $env.register(ProgramEscrowContract, ());
        let $client = ProgramEscrowContractClient::new(&$env, &$contract_id);

        let $admin = Address::generate(&$env);
        let $program_admin = Address::generate(&$env);

        let token_contract = $env.register_stellar_asset_contract_v2($admin.clone());
        let token_addr = token_contract.address();
        let $token_client = token::Client::new(&$env, &token_addr);
        let $token_admin = token::StellarAssetClient::new(&$env, &token_addr);

        let _ = $client.init(&$admin, &token_addr);
        $token_admin.mint(&$program_admin, &$initial_balance);
    };
}


fn has_event_topic(env: &Env, topic_name: &str) -> bool {
    use soroban_sdk::IntoVal;
    let expected: soroban_sdk::Val = Symbol::new(env, topic_name).into_val(env);
    let events = env.events().all();
    std::println!("Events len: {}", events.len());
    for (_contract, topics, _data) in events.iter() {
        if topics.len() > 0 {
            let first = topics.get(0).unwrap();
            std::println!("Topic 0: {:?}, expected: {:?}", first, expected);
            if first.get_payload() == expected.get_payload() {
                return true;
            }
        }
    }
    false
}

// ==================== SINGLE REGISTRATION ====================

#[test]
fn test_register_single_program() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );
    let name = String::from_str(&env, "Stellar Q1 Grant");

    client.register_program(&1, &program_admin, &name, &5_000);

    let program = client.get_program(&1);
    assert_eq!(program.admin, program_admin);
    assert_eq!(program.name, name);
    assert_eq!(program.total_funding, 5_000);
    assert_eq!(program.status, ProgramStatus::Active);
    assert_eq!(token_client.balance(&contract_id), 5_000);
    assert_eq!(token_client.balance(&program_admin), 5_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // ProgramExists
fn test_register_duplicate_single_program() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        20_000i128
    );
    let name = String::from_str(&env, "Grant Round");

    client.register_program(&1, &program_admin, &name, &5_000);
    client.register_program(&1, &program_admin, &name, &5_000);
}

// ==================== BATCH HAPPY PATH ====================

#[test]
fn test_batch_register_multiple_programs() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        50_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Hackathon Alpha"),
            total_funding: 10_000,
        },
        ProgramRegistrationItem {
            program_id: 2,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Hackathon Beta"),
            total_funding: 15_000,
        },
        ProgramRegistrationItem {
            program_id: 3,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Hackathon Gamma"),
            total_funding: 5_000,
        },
    ];

    let count = client.batch_register_programs(&items);
    assert_eq!(count, 3);

    for id in 1..=3 {
        let program = client.get_program(&id);
        assert_eq!(program.status, ProgramStatus::Active);
        assert_eq!(program.admin, program_admin);
    }

    assert_eq!(client.get_program(&1).total_funding, 10_000);
    assert_eq!(client.get_program(&2).total_funding, 15_000);
    assert_eq!(client.get_program(&3).total_funding, 5_000);

    // Total transferred: 30_000
    assert_eq!(token_client.balance(&contract_id), 30_000);
    assert_eq!(token_client.balance(&program_admin), 20_000);
}

#[test]
fn test_batch_register_single_item() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 42,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Solo Program"),
            total_funding: 3_000,
        },
    ];

    let count = client.batch_register_programs(&items);
    assert_eq!(count, 1);

    let program = client.get_program(&42);
    assert_eq!(program.total_funding, 3_000);
    assert_eq!(program.status, ProgramStatus::Active);
}

#[test]
fn test_batch_register_with_different_admins() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );
    let other_admin = Address::generate(&env);
    token_admin.mint(&other_admin, &10_000);

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Program A"),
            total_funding: 5_000,
        },
        ProgramRegistrationItem {
            program_id: 2,
            admin: other_admin.clone(),
            name: String::from_str(&env, "Program B"),
            total_funding: 7_000,
        },
    ];

    let count = client.batch_register_programs(&items);
    assert_eq!(count, 2);

    assert_eq!(client.get_program(&1).admin, program_admin);
    assert_eq!(client.get_program(&2).admin, other_admin);
}

// ==================== BATCH SIZE BOUNDARY ====================

#[test]
fn test_batch_register_at_max_batch_size() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        200_000i128
    );

    let mut items = Vec::new(&env);
    for i in 1..=20u64 {
        items.push_back(ProgramRegistrationItem {
            program_id: i,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Program"),
            total_funding: 100,
        });
    }

    let count = client.batch_register_programs(&items);
    assert_eq!(count, 20);

    for i in 1..=20u64 {
        let program = client.get_program(&i);
        assert_eq!(program.status, ProgramStatus::Active);
    }
    assert_eq!(token_client.balance(&contract_id), 2_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // InvalidBatchSize
fn test_batch_register_exceeds_max_batch_size() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        200_000i128
    );

    let mut items = Vec::new(&env);
    for i in 1..=21u64 {
        items.push_back(ProgramRegistrationItem {
            program_id: i,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Program"),
            total_funding: 100,
        });
    }

    client.batch_register_programs(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // InvalidBatchSize
fn test_batch_register_empty_batch() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );
    let items: Vec<ProgramRegistrationItem> = vec![&env];
    client.batch_register_programs(&items);
}

// ==================== DUPLICATE PROGRAM ID ====================

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // DuplicateProgramId
fn test_batch_register_duplicate_ids_in_batch() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        20_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "First"),
            total_funding: 1_000,
        },
        ProgramRegistrationItem {
            program_id: 1, // duplicate
            admin: program_admin.clone(),
            name: String::from_str(&env, "Second"),
            total_funding: 2_000,
        },
    ];

    client.batch_register_programs(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // DuplicateProgramId
fn test_batch_register_triple_duplicate_in_batch() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        30_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 5,
            admin: program_admin.clone(),
            name: String::from_str(&env, "A"),
            total_funding: 1_000,
        },
        ProgramRegistrationItem {
            program_id: 5, // duplicate
            admin: program_admin.clone(),
            name: String::from_str(&env, "B"),
            total_funding: 2_000,
        },
        ProgramRegistrationItem {
            program_id: 5, // triple
            admin: program_admin.clone(),
            name: String::from_str(&env, "C"),
            total_funding: 3_000,
        },
    ];

    client.batch_register_programs(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // DuplicateProgramId
fn test_batch_register_non_adjacent_duplicates() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        30_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "First"),
            total_funding: 1_000,
        },
        ProgramRegistrationItem {
            program_id: 2,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Middle"),
            total_funding: 2_000,
        },
        ProgramRegistrationItem {
            program_id: 1, // non-adjacent duplicate of first
            admin: program_admin.clone(),
            name: String::from_str(&env, "Sneaky Dup"),
            total_funding: 3_000,
        },
    ];

    client.batch_register_programs(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // ProgramExists
fn test_batch_register_conflicts_with_existing_program() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        50_000i128
    );

    // Register program 1 individually first
    client.register_program(
        &1,
        &program_admin,
        &String::from_str(&env, "Existing"),
        &5_000,
    );

    // Batch includes program_id 1 which already exists
    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1, // already registered
            admin: program_admin.clone(),
            name: String::from_str(&env, "Conflict"),
            total_funding: 2_000,
        },
        ProgramRegistrationItem {
            program_id: 2,
            admin: program_admin.clone(),
            name: String::from_str(&env, "New"),
            total_funding: 3_000,
        },
    ];

    client.batch_register_programs(&items);
}

// ==================== INVALID AMOUNT ====================

#[test]
#[should_panic(expected = "Error(Contract, #8)")] // InvalidAmount
fn test_batch_register_zero_amount() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Zero Fund"),
            total_funding: 0,
        },
    ];

    client.batch_register_programs(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")] // InvalidAmount
fn test_batch_register_negative_amount() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Negative"),
            total_funding: -500,
        },
    ];

    client.batch_register_programs(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")] // InvalidAmount
fn test_batch_register_mixed_valid_invalid_amounts() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        20_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Valid"),
            total_funding: 5_000,
        },
        ProgramRegistrationItem {
            program_id: 2,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Invalid"),
            total_funding: 0,
        },
    ];

    client.batch_register_programs(&items);
}

// ==================== INVALID NAME ====================

#[test]
#[should_panic(expected = "Error(Contract, #9)")] // InvalidName
fn test_batch_register_empty_name() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, ""),
            total_funding: 1_000,
        },
    ];

    client.batch_register_programs(&items);
}

// ==================== ATOMICITY (ALL-OR-NOTHING) ====================

#[test]
fn test_batch_register_atomicity_no_partial_writes() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        50_000i128
    );

    // Register program 3 first so the batch will fail on it
    client.register_program(
        &3,
        &program_admin,
        &String::from_str(&env, "Pre-existing"),
        &1_000,
    );

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 10,
            admin: program_admin.clone(),
            name: String::from_str(&env, "New A"),
            total_funding: 2_000,
        },
        ProgramRegistrationItem {
            program_id: 3, // already exists — triggers failure
            admin: program_admin.clone(),
            name: String::from_str(&env, "Conflict"),
            total_funding: 3_000,
        },
    ];

    let res = client.try_batch_register_programs(&items);
    assert!(res.is_err());

    // Program 10 must NOT have been created (all-or-nothing)
    let lookup = client.try_get_program(&10);
    assert!(lookup.is_err());
}

#[test]
fn test_batch_register_atomicity_balance_unchanged_on_failure() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        50_000i128
    );

    let balance_before = token_client.balance(&program_admin);

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Good"),
            total_funding: 5_000,
        },
        ProgramRegistrationItem {
            program_id: 2,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Bad"),
            total_funding: -1, // invalid amount
        },
    ];

    let res = client.try_batch_register_programs(&items);
    assert!(res.is_err());

    // Balance must be unchanged since the batch was rejected during validation
    assert_eq!(token_client.balance(&program_admin), balance_before);
}

// ==================== NOT INITIALIZED ====================

#[test]
fn test_batch_register_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ProgramEscrowContract, ());
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let some_admin = Address::generate(&env);

    let items = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: some_admin.clone(),
            name: String::from_str(&env, "Test"),
            total_funding: 1_000,
        },
    ];

    let res = client.try_batch_register_programs(&items);
    assert!(res.is_err());
}

// ==================== PROGRAM LOOKUP ====================

#[test]
fn test_get_program_not_found() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );
    let res = client.try_get_program(&999);
    assert!(res.is_err());
}

// ==================== INIT EDGE CASES ====================

#[test]
fn test_double_init_fails() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        10_000i128
    );
    let other = Address::generate(&env);
    let res = client.try_init(&other, &other);
    assert!(res.is_err());
}

// ==================== SEQUENTIAL BATCH CALLS ====================

#[test]
fn test_sequential_batch_registrations() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        100_000i128
    );

    // First batch: programs 1-2
    let batch_one = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Round 1A"),
            total_funding: 5_000,
        },
        ProgramRegistrationItem {
            program_id: 2,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Round 1B"),
            total_funding: 5_000,
        },
    ];

    let count1 = client.batch_register_programs(&batch_one);
    assert_eq!(count1, 2);

    // Second batch: programs 3-4 (no overlap)
    let batch_two = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 3,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Round 2A"),
            total_funding: 8_000,
        },
        ProgramRegistrationItem {
            program_id: 4,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Round 2B"),
            total_funding: 2_000,
        },
    ];

    let count2 = client.batch_register_programs(&batch_two);
    assert_eq!(count2, 2);

    // All four programs exist
    for id in 1..=4 {
        let p = client.get_program(&id);
        assert_eq!(p.status, ProgramStatus::Active);
    }
    assert_eq!(token_client.balance(&contract_id), 20_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // ProgramExists
fn test_sequential_batch_overlap_fails() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        100_000i128
    );

    let batch_one = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1,
            admin: program_admin.clone(),
            name: String::from_str(&env, "First"),
            total_funding: 5_000,
        },
    ];
    client.batch_register_programs(&batch_one);

    // Second batch reuses program_id 1
    let batch_two = vec![
        &env,
        ProgramRegistrationItem {
            program_id: 1, // already registered in batch one
            admin: program_admin.clone(),
            name: String::from_str(&env, "Overlap"),
            total_funding: 3_000,
        },
    ];
    client.batch_register_programs(&batch_two);
}

// ==================== JURISDICTION CONTROLS ====================

#[test]
fn test_register_program_juris_config() {
fn test_register_prog_w_juris_config() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        25_000i128
    );

    let cfg = ProgramJurisdictionConfig {
        tag: Some(String::from_str(&env, "EU-only")),
        requires_kyc: true,
        max_funding: Some(10_000),
        registration_paused: false,
    };

    client.register_program_juris(
    client.register_prog_w_juris(
        &91,
        &program_admin,
        &String::from_str(&env, "EU Hackathon Program"),
        &5_000,
        &cfg.tag.clone(),
        &cfg.requires_kyc,
        &cfg.max_funding.clone(),
        &cfg.registration_paused,
        &OptionalJurisdiction::Some(cfg.clone()),
        &Some(true),
    );

    let program = client.get_program(&91);
    assert_eq!(client.get_program_jurisdiction(&91), Some(cfg));
}

#[test]
fn test_register_program_juris_requires_kyc_attestation() {
    assert_eq!(program.jurisdiction, OptionalJurisdiction::Some(cfg.clone()));
    assert_eq!(client.get_program_jurisdiction(&91), OptionalJurisdiction::Some(cfg));
}

#[test]
fn test_register_prog_w_juris_requires_kyc_attestation() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        25_000i128
    );

    let cfg = ProgramJurisdictionConfig {
        tag: Some(String::from_str(&env, "US-only")),
        requires_kyc: true,
        max_funding: Some(10_000),
        registration_paused: false,
    };

    let res = client.try_register_program_juris(
    let res = client.try_register_prog_w_juris(
        &92,
        &program_admin,
        &String::from_str(&env, "US Program"),
        &5_000,
        &cfg.tag.clone(),
        &cfg.requires_kyc,
        &cfg.max_funding.clone(),
        &cfg.registration_paused,
        &OptionalJurisdiction::Some(cfg),
        &Some(false),
    );
    assert!(res.is_err());
}

#[test]
fn test_register_program_juris_max_funding_enforced() {
fn test_register_prog_w_juris_max_funding_enforced() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        25_000i128
    );

    let cfg = ProgramJurisdictionConfig {
        tag: Some(String::from_str(&env, "EU-only")),
        requires_kyc: false,
        max_funding: Some(2_000),
        registration_paused: false,
    };

    let res = client.try_register_program_juris(
    let res = client.try_register_prog_w_juris(
        &93,
        &program_admin,
        &String::from_str(&env, "Capped Program"),
        &5_000,
        &cfg.tag.clone(),
        &cfg.requires_kyc,
        &cfg.max_funding.clone(),
        &cfg.registration_paused,
        &OptionalJurisdiction::Some(cfg),
        &None,
    );
    assert!(res.is_err());
}

#[test]
fn test_register_program_juris_pause_enforced() {
fn test_register_prog_w_juris_pause_enforced() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        25_000i128
    );

    let cfg = ProgramJurisdictionConfig {
        tag: Some(String::from_str(&env, "paused-zone")),
        requires_kyc: false,
        max_funding: Some(8_000),
        registration_paused: true,
    };

    let res = client.try_register_program_juris(
    let res = client.try_register_prog_w_juris(
        &94,
        &program_admin,
        &String::from_str(&env, "Paused Program"),
        &5_000,
        &cfg.tag.clone(),
        &cfg.requires_kyc,
        &cfg.max_funding.clone(),
        &cfg.registration_paused,
        &OptionalJurisdiction::Some(cfg),
        &None,
    );
    assert!(res.is_err());
}

#[test]
fn test_batch_register_juris() {
fn test_batch_reg_progs_w_juris() {
    setup!(
        env,
        client,
        contract_id,
        admin,
        program_admin,
        token_client,
        token_admin,
        40_000i128
    );

    let eu_cfg = ProgramJurisdictionConfig {
        tag: Some(String::from_str(&env, "EU-only")),
        requires_kyc: true,
        max_funding: Some(10_000),
        registration_paused: false,
    };

    let items = vec![
        &env,
        ProgramRegistrationWithJurisdictionItem {
            program_id: 95,
            admin: program_admin.clone(),
            name: String::from_str(&env, "Generic Program"),
            total_funding: 5_000,
            juris_tag: None,
            juris_requires_kyc: false,
            juris_max_funding: None,
            juris_registration_paused: false,
            jurisdiction: OptionalJurisdiction::None,
            kyc_attested: None,
        },
        ProgramRegistrationWithJurisdictionItem {
            program_id: 96,
            admin: program_admin.clone(),
            name: String::from_str(&env, "EU Program"),
            total_funding: 7_000,
            juris_tag: eu_cfg.tag.clone(),
            juris_requires_kyc: eu_cfg.requires_kyc,
            juris_max_funding: eu_cfg.max_funding.clone(),
            juris_registration_paused: eu_cfg.registration_paused,
            jurisdiction: OptionalJurisdiction::Some(eu_cfg.clone()),
            kyc_attested: Some(true),
        },
    ];

    let count = client.batch_register_juris(&items);
    assert_eq!(count, 2);

    let generic = client.get_program(&95);
    assert_eq!(client.get_program_jurisdiction(&95), None);

    let eu = client.get_program(&96);
    assert_eq!(client.get_program_jurisdiction(&96), Some(eu_cfg));
    let count = client.batch_reg_progs_w_juris(&items);
    assert_eq!(count, 2);

    let generic = client.get_program(&95);
    assert_eq!(generic.jurisdiction, OptionalJurisdiction::None);

    let eu = client.get_program(&96);
    assert_eq!(eu.jurisdiction, OptionalJurisdiction::Some(eu_cfg));
}
