use origin_storage::{Record, Storage, StorageKey};
use origin_storage_sqlite::SqliteStorage;
use time::macros::datetime;

#[tokio::test]
async fn sqlite_storage_satisfies_the_storage_contract() {
    let storage = SqliteStorage::in_memory().expect("open in-memory database");
    origin_storage::contract::run_all(&storage, "origin-sqlite-test").await;
}

#[tokio::test]
async fn data_survives_reopening_the_database_file() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("origin.sqlite3");
    let key = StorageKey::new("github", "notifications");

    {
        let storage = SqliteStorage::open(&path).expect("open");
        storage
            .put(
                &key,
                Record::new("\"value\"", datetime!(2026-08-23 10:00 UTC)),
            )
            .await
            .expect("put");
    }

    let reopened = SqliteStorage::open(&path).expect("reopen");
    let record = reopened.get(&key).await.expect("get").expect("present");
    assert_eq!(record.value, "\"value\"");
    assert_eq!(record.stored_at, datetime!(2026-08-23 10:00 UTC));
}

#[tokio::test]
async fn pruning_removes_only_expired_records() {
    let storage = SqliteStorage::in_memory().expect("open");
    let stored_at = datetime!(2026-08-23 10:00 UTC);
    let expiring = StorageKey::new("cache", "expiring");
    let permanent = StorageKey::new("cache", "permanent");

    storage
        .put(
            &expiring,
            Record::new("1", stored_at).expiring_at(datetime!(2026-08-23 10:05 UTC)),
        )
        .await
        .expect("put expiring");
    storage
        .put(&permanent, Record::new("2", stored_at))
        .await
        .expect("put permanent");

    let removed = storage
        .prune_expired(datetime!(2026-08-23 11:00 UTC))
        .await
        .expect("prune");

    assert_eq!(removed, 1);
    assert!(storage.get(&expiring).await.expect("get").is_none());
    assert!(storage.get(&permanent).await.expect("get").is_some());
}
