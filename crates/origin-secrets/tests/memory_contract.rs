use origin_secrets::MemorySecretStore;

#[tokio::test]
async fn memory_store_satisfies_the_secret_store_contract() {
    origin_secrets::contract::run_all(&MemorySecretStore::new(), "origin-memory-test").await;
}
