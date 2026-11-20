#![no_main]

use evidence_registry::{EvidenceRegistry, EvidenceRegistryClient};
use libfuzzer_sys::fuzz_target;
use rampspec_shared_types::{ContractError, EvidenceInput, NetworkKind, SCHEMA_VERSION};
use soroban_sdk::{Address, BytesN, Env, testutils::Address as _, testutils::Events as _};
use std::vec::Vec;

fn input(env: &Env, publisher: &Address, report_byte: u8) -> EvidenceInput {
    EvidenceInput {
        publisher: publisher.clone(),
        report_hash: BytesN::from_array(env, &[report_byte.max(1); 32]),
        target_hash: BytesN::from_array(env, &[2; 32]),
        suite_hash: BytesN::from_array(env, &[3; 32]),
        specs_hash: BytesN::from_array(env, &[4; 32]),
        artifact_root: BytesN::from_array(env, &[5; 32]),
        network: NetworkKind::Testnet,
        protocol_bitmap: 1,
        score_bps: 5_000,
        passed: 1,
        failed: 0,
        warnings: 0,
        skipped: 0,
    }
}

fuzz_target!(|data: &[u8]| {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    let publisher = Address::generate(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);
    client.register_attestor(&publisher, &BytesN::from_array(&env, &[9; 32]));
    let mut known_ids = Vec::new();

    for action in data.chunks(3).take(32) {
        let opcode = action[0];
        let selector = action.get(1).copied().unwrap_or_default();
        let report_byte = action.get(2).copied().unwrap_or_default().max(1);
        match opcode % 4 {
            0 => {
                let evidence = input(&env, &publisher, report_byte);
                if let Ok(Ok(id)) = client.try_publish_evidence(&evidence) {
                    known_ids.push(id);
                }
            }
            1 => {
                let _ = client.try_set_paused(&(opcode & 0x80 != 0));
            }
            2 => {
                if !known_ids.is_empty() {
                    let id = known_ids[usize::from(selector) % known_ids.len()].clone();
                    let reason = BytesN::from_array(&env, &[7; 32]);
                    let _ = client.try_revoke_evidence(&id, &reason, &publisher);
                }
            }
            _ => {
                let old_id = if known_ids.is_empty() || selector & 1 == 0 {
                    BytesN::from_array(&env, &[selector.max(1); 32])
                } else {
                    known_ids[usize::from(selector) % known_ids.len()].clone()
                };
                let replacement = input(&env, &publisher, report_byte);
                let before: Vec<_> = known_ids
                    .iter()
                    .map(|id| (id.clone(), client.get_evidence(id)))
                    .collect();
                let result = client.try_supersede_evidence(&old_id, &replacement, &publisher);
                match result {
                    Ok(Ok(next_id)) => known_ids.push(next_id),
                    rejected => {
                        for (id, record) in before {
                            assert_eq!(client.get_evidence(&id), record);
                        }
                        assert_eq!(env.events().all().events().len(), 0);
                        assert!(matches!(
                            rejected,
                            Err(Ok(ContractError::EvidenceNotFound
                                | ContractError::EvidenceNotActive
                                | ContractError::EvidenceAlreadyExists
                                | ContractError::InvalidSupersession
                                | ContractError::Paused))
                                | Err(Err(_))
                        ));
                    }
                }
            }
        }
    }
});
