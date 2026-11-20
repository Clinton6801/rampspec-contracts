use policy_account_fixture::{PolicyConfigurationChanged, SignatureMode};
use soroban_sdk::{BytesN, Env, FromVal, Map, Symbol, TryFromVal, Val, events::Event};

#[test]
fn policy_configuration_event_schema_is_stable_and_privacy_safe() {
    let env = Env::default();
    let event = PolicyConfigurationChanged {
        change: Symbol::new(&env, "set_signers"),
        version: 2,
        policy_commitment: BytesN::from_array(&env, &[7; 32]),
        signature_mode: SignatureMode::Ed25519,
        signer_count: 3,
        threshold: 2,
    };
    let topics = event.topics(&env);
    assert_eq!(topics.len(), 2);
    assert_eq!(
        Symbol::from_val(&env, &topics.get(0).unwrap()),
        Symbol::new(&env, "policy_configuration_changed")
    );
    assert_eq!(
        Symbol::from_val(&env, &topics.get(1).unwrap()),
        Symbol::new(&env, "set_signers")
    );
    let data = Map::<Symbol, Val>::try_from_val(&env, &event.data(&env)).unwrap();
    assert_eq!(data.len(), 5);
    for field in [
        "version",
        "policy_commitment",
        "signature_mode",
        "signer_count",
        "threshold",
    ] {
        assert!(data.contains_key(Symbol::new(&env, field)));
    }
    assert!(!data.contains_key(Symbol::new(&env, "signers")));
    assert!(!data.contains_key(Symbol::new(&env, "signature")));
}
