//! The contract every [`Storage`] implementation must satisfy (§41).

use crate::{Record, Storage, StorageKey};
use time::OffsetDateTime;
use time::macros::datetime;

const STORED_AT: OffsetDateTime = datetime!(2026-08-23 10:00 UTC);

/// Run every contract check against `storage`.
///
/// `namespace` must be unique per implementation so parallel runs cannot collide.
pub async fn run_all<S: Storage>(storage: &S, namespace: &str) {
    storage.clear(namespace).await.expect("clear before run");

    missing_key_returns_none(storage, namespace).await;
    put_then_get(storage, namespace).await;
    put_overwrites(storage, namespace).await;
    delete_removes_and_is_idempotent(storage, namespace).await;
    keys_are_scoped_to_their_namespace(storage, namespace).await;
    clear_only_affects_its_namespace(storage, namespace).await;
    clear_prefix_removes_a_whole_subtree(storage, namespace).await;
    expiry_metadata_round_trips(storage, namespace).await;

    storage.clear(namespace).await.expect("clear after run");
}

async fn missing_key_returns_none<S: Storage>(storage: &S, namespace: &str) {
    let key = StorageKey::new(namespace, "absent");
    assert!(
        storage.get(&key).await.expect("get").is_none(),
        "a missing key must yield Ok(None)"
    );
}

async fn put_then_get<S: Storage>(storage: &S, namespace: &str) {
    let key = StorageKey::new(namespace, "roundtrip");
    storage
        .put(&key, Record::new("\"value\"", STORED_AT))
        .await
        .expect("put");

    let record = storage.get(&key).await.expect("get").expect("present");
    assert_eq!(record.value, "\"value\"");
    assert_eq!(record.stored_at, STORED_AT);

    storage.delete(&key).await.expect("cleanup");
}

async fn put_overwrites<S: Storage>(storage: &S, namespace: &str) {
    let key = StorageKey::new(namespace, "overwrite");
    storage
        .put(&key, Record::new("\"first\"", STORED_AT))
        .await
        .expect("put");
    storage
        .put(&key, Record::new("\"second\"", STORED_AT))
        .await
        .expect("overwrite");

    let record = storage.get(&key).await.expect("get").expect("present");
    assert_eq!(record.value, "\"second\"");

    storage.delete(&key).await.expect("cleanup");
}

async fn delete_removes_and_is_idempotent<S: Storage>(storage: &S, namespace: &str) {
    let key = StorageKey::new(namespace, "delete");
    storage
        .put(&key, Record::new("\"value\"", STORED_AT))
        .await
        .expect("put");

    storage.delete(&key).await.expect("delete");
    assert!(storage.get(&key).await.expect("get").is_none());
    storage
        .delete(&key)
        .await
        .expect("deleting a missing key must succeed");
}

async fn keys_are_scoped_to_their_namespace<S: Storage>(storage: &S, namespace: &str) {
    let other = format!("{namespace}-other");
    let mine = StorageKey::new(namespace, "mine");
    let theirs = StorageKey::new(&other, "theirs");

    storage
        .put(&mine, Record::new("1", STORED_AT))
        .await
        .expect("put mine");
    storage
        .put(&theirs, Record::new("2", STORED_AT))
        .await
        .expect("put theirs");

    let keys = storage.keys(namespace).await.expect("keys");
    assert!(keys.contains(&mine));
    assert!(
        !keys.contains(&theirs),
        "keys() must not leak across namespaces"
    );

    storage.clear(&other).await.expect("cleanup other");
    storage.delete(&mine).await.expect("cleanup mine");
}

async fn clear_only_affects_its_namespace<S: Storage>(storage: &S, namespace: &str) {
    let other = format!("{namespace}-keep");
    let dropped = StorageKey::new(namespace, "dropped");
    let kept = StorageKey::new(&other, "kept");

    storage
        .put(&dropped, Record::new("1", STORED_AT))
        .await
        .expect("put");
    storage
        .put(&kept, Record::new("2", STORED_AT))
        .await
        .expect("put");

    storage.clear(namespace).await.expect("clear");

    assert!(storage.get(&dropped).await.expect("get").is_none());
    assert!(
        storage.get(&kept).await.expect("get").is_some(),
        "clear() must not touch other namespaces"
    );

    storage.clear(&other).await.expect("cleanup");
}

async fn clear_prefix_removes_a_whole_subtree<S: Storage>(storage: &S, namespace: &str) {
    let prefix = format!("{namespace}.acct.demo.a1.");
    let inside_one = StorageKey::new(format!("{prefix}notifications"), "k");
    let inside_two = StorageKey::new(format!("{prefix}sync"), "k");
    // Same prefix up to the separator — must survive, or one account's disconnect
    // would wipe another's data.
    let sibling = StorageKey::new(format!("{namespace}.acct.demo.a1b2.sync"), "k");

    for key in [&inside_one, &inside_two, &sibling] {
        storage
            .put(key, Record::new("1", STORED_AT))
            .await
            .expect("put");
    }

    let removed = storage.clear_prefix(&prefix).await.expect("clear_prefix");

    assert_eq!(
        removed, 2,
        "clear_prefix must report how many records it removed"
    );
    assert!(storage.get(&inside_one).await.expect("get").is_none());
    assert!(storage.get(&inside_two).await.expect("get").is_none());
    assert!(
        storage.get(&sibling).await.expect("get").is_some(),
        "a namespace that merely starts with the same characters must survive"
    );

    storage.delete(&sibling).await.expect("cleanup");
}

async fn expiry_metadata_round_trips<S: Storage>(storage: &S, namespace: &str) {
    let key = StorageKey::new(namespace, "expiring");
    let expires_at = datetime!(2026-08-23 10:05 UTC);
    storage
        .put(
            &key,
            Record::new("\"value\"", STORED_AT).expiring_at(expires_at),
        )
        .await
        .expect("put");

    let record = storage.get(&key).await.expect("get").expect("present");
    assert_eq!(
        record.expires_at,
        Some(expires_at),
        "expiry metadata must survive a round trip"
    );
    assert!(record.is_expired_at(datetime!(2026-08-23 10:06 UTC)));
    assert!(!record.is_expired_at(datetime!(2026-08-23 10:04 UTC)));

    storage.delete(&key).await.expect("cleanup");
}
