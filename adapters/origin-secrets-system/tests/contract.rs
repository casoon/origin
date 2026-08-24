use origin_secrets_system::SystemSecretStore;

/// Runs the shared `SecretStore` contract against the real credential store.
///
/// Ignored by default: it writes to the developer's login keychain and can raise a
/// system prompt, so it must never run unattended in CI. Run it deliberately with
/// `cargo test -p origin-secrets-system -- --ignored`.
#[tokio::test]
#[ignore = "touches the real system credential store"]
async fn system_secret_store_satisfies_the_secret_store_contract() {
    let store = SystemSecretStore::new("dev.origin.contract-test");
    origin_secrets::contract::run_all(&store, "contract").await;
}
