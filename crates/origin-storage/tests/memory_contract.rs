use origin_storage::MemoryStorage;

#[tokio::test]
async fn memory_storage_satisfies_the_storage_contract() {
    origin_storage::contract::run_all(&MemoryStorage::new(), "origin-memory-test").await;
}
