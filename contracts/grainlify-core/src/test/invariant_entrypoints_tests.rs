#![cfg(test)]

use crate::{DataKey, GrainlifyContract, GrainlifyContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

fn setup_contract(env: &Env) -> (GrainlifyContractClient<'_>, Address) {
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.init_admin(&admin);
    (client, admin)
}

#[test]
fn test_check_invariants_healthy_after_init() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let report = client.check_invariants();
    assert!(report.healthy);
    assert!(report.config_sane);
    assert!(report.metrics_sane);
    assert!(report.admin_set);
    assert!(report.version_set);
    assert_eq!(report.violation_count, 0);
    assert!(client.verify_invariants());
}

#[test]
fn test_check_invariants_detects_metric_drift() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    env.as_contract(&client.address, || {
        let op_key = Symbol::new(&env, "op_count");
        let err_key = Symbol::new(&env, "err_count");
        env.storage().persistent().set(&op_key, &2_u64);
        env.storage().persistent().set(&err_key, &5_u64);
    });

    let report = client.check_invariants();
    assert!(report.config_sane);
    assert!(!report.metrics_sane);
    assert!(!report.healthy);
    assert!(report.violation_count > 0);
    assert!(!client.verify_invariants());
}

#[test]
fn test_check_invariants_detects_config_drift() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    env.as_contract(&client.address, || {
        env.storage().instance().remove(&DataKey::Version);
    });

    let report = client.check_invariants();
    assert!(!report.config_sane);
    assert!(!report.healthy);
    assert!(report.violation_count > 0);
    assert!(!client.verify_invariants());
}
