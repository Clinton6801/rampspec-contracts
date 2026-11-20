#![no_main]

use evidence_registry::{EvidenceRegistry, EvidenceRegistryClient};
use libfuzzer_sys::fuzz_target;
use rampspec_shared_types::{
    ContractError, EVIDENCE_ID_DOMAIN, EvidenceInput, EvidenceRecord, MAX_SCORE_BPS, NetworkKind,
    PersistentKeyV1, SCHEMA_VERSION, SUPPORTED_PROTOCOL_BITMAP,
};
use soroban_sdk::{
    Address, Bytes, BytesN, Env,
    testutils::{Address as _, Events as _, storage::Instance as _, storage::Persistent as _},
    xdr::{FromXdr, ToXdr},
};

fn byte(data: &[u8], index: usize) -> u8 {
    data.get(index).copied().unwrap_or_default()
}

fn u32_at(data: &[u8], index: usize) -> u32 {
    u32::from_le_bytes([
        byte(data, index),
        byte(data, index + 1),
        byte(data, index + 2),
        byte(data, index + 3),
    ])
}

fn u64_at(data: &[u8], index: usize) -> u64 {
    u64::from_le_bytes([
        byte(data, index),
        byte(data, index + 1),
        byte(data, index + 2),
        byte(data, index + 3),
        byte(data, index + 4),
        byte(data, index + 5),
        byte(data, index + 6),
        byte(data, index + 7),
    ])
}

fn expected_error(input: &EvidenceInput) -> Option<ContractError> {
    let hashes = [
        &input.report_hash,
        &input.target_hash,
        &input.suite_hash,
        &input.specs_hash,
        &input.artifact_root,
    ];
    if hashes.iter().any(|hash| hash.to_array() == [0; 32]) {
        return Some(ContractError::InvalidHash);
    }
    if input.score_bps > MAX_SCORE_BPS {
        return Some(ContractError::InvalidScore);
    }
    if input.protocol_bitmap == 0 || input.protocol_bitmap & !SUPPORTED_PROTOCOL_BITMAP != 0 {
        return Some(ContractError::InvalidCounts);
    }
    let total = input
        .passed
        .checked_add(input.failed)
        .and_then(|value| value.checked_add(input.warnings))
        .and_then(|value| value.checked_add(input.skipped));
    if total.is_none_or(|value| value == 0) {
        return Some(ContractError::InvalidCounts);
    }
    None
}

fn evidence_id(env: &Env, input: &EvidenceInput) -> BytesN<32> {
    let mut preimage = Bytes::new(env);
    preimage.extend_from_slice(EVIDENCE_ID_DOMAIN);
    preimage.append(&input.publisher.clone().to_xdr(env));
    preimage.extend_from_slice(&input.report_hash.to_array());
    preimage.extend_from_slice(&input.network.discriminant().to_be_bytes());
    BytesN::from_array(env, &env.crypto().sha256(&preimage).to_array())
}

fuzz_target!(|data: &[u8]| {
    let env = Env::default();
    let bytes = Bytes::from_slice(&env, data);
    let _ = EvidenceInput::from_xdr(&env, &bytes);
    let _ = EvidenceRecord::from_xdr(&env, &bytes);

    let publisher = Address::generate(&env);
    let input = EvidenceInput {
        publisher: publisher.clone(),
        report_hash: BytesN::from_array(&env, &[byte(data, 0); 32]),
        target_hash: BytesN::from_array(&env, &[byte(data, 1); 32]),
        suite_hash: BytesN::from_array(&env, &[byte(data, 2); 32]),
        specs_hash: BytesN::from_array(&env, &[byte(data, 3); 32]),
        artifact_root: BytesN::from_array(&env, &[byte(data, 4); 32]),
        network: match byte(data, 5) % 4 {
            0 => NetworkKind::Testnet,
            1 => NetworkKind::Futurenet,
            2 => NetworkKind::Pubnet,
            _ => NetworkKind::Standalone,
        },
        protocol_bitmap: u64_at(data, 6),
        score_bps: u32_at(data, 14),
        passed: u32_at(data, 18),
        failed: u32_at(data, 22),
        warnings: u32_at(data, 26),
        skipped: u32_at(data, 30),
    };
    env.mock_all_auths();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);
    client.register_attestor(&publisher, &BytesN::from_array(&env, &[9; 32]));
    let id = evidence_id(&env, &input);
    let ttl_before = env.as_contract(&contract_id, || {
        (
            env.storage().instance().get_ttl(),
            env.storage()
                .persistent()
                .get_ttl(&PersistentKeyV1::Attestor(publisher.clone())),
        )
    });

    let result = client.try_publish_evidence(&input);
    if let Some(error) = expected_error(&input) {
        assert_eq!(result, Err(Ok(error)));
        assert_eq!(env.events().all().events().len(), 0);
        assert_eq!(client.get_evidence(&id), None);
        env.as_contract(&contract_id, || {
            assert_eq!(
                env.storage()
                    .persistent()
                    .get::<_, BytesN<32>>(&PersistentKeyV1::ActiveReport(
                        input.publisher.clone(),
                        input.report_hash.clone(),
                        input.network.clone(),
                    ),),
                None
            );
            assert_eq!(
                (
                    env.storage().instance().get_ttl(),
                    env.storage()
                        .persistent()
                        .get_ttl(&PersistentKeyV1::Attestor(publisher.clone())),
                ),
                ttl_before
            );
        });
    } else {
        assert_eq!(result, Ok(Ok(id.clone())));
        assert_eq!(client.get_evidence(&id).map(|record| record.id), Some(id));
    }
});
