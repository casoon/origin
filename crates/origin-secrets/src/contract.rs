//! The contract every [`SecretStore`] implementation must satisfy (ADR-0004, §41).
//!
//! Adapters run this suite against their own backend, so a system keychain and the
//! in-memory double cannot drift apart:
//!
//! ```ignore
//! #[tokio::test]
//! async fn satisfies_the_secret_store_contract() {
//!     origin_secrets::contract::run_all(&MySecretStore::new(), "my-store-test").await;
//! }
//! ```

use crate::{Secret, SecretKey, SecretStore};

/// Run every contract check against `store`.
///
/// `namespace` must be unique per implementation so that parallel test runs against a
/// real system keychain do not collide.
pub async fn run_all<S: SecretStore>(store: &S, namespace: &str) {
    missing_key_returns_none(store, namespace).await;
    write_then_read(store, namespace).await;
    overwrite_replaces_the_value(store, namespace).await;
    delete_removes_the_value(store, namespace).await;
    delete_is_idempotent(store, namespace).await;
    keys_are_isolated(store, namespace).await;
    concurrent_writes_do_not_corrupt(store, namespace).await;
}

async fn missing_key_returns_none<S: SecretStore>(store: &S, namespace: &str) {
    let key = SecretKey::new(namespace, "does-not-exist");
    let _ = store.delete(&key).await;

    assert!(
        store.get(&key).await.expect("get must not fail").is_none(),
        "a missing key must yield Ok(None), not an error"
    );
}

async fn write_then_read<S: SecretStore>(store: &S, namespace: &str) {
    let key = SecretKey::new(namespace, "write-then-read");
    store.set(&key, Secret::new("value-1")).await.expect("set");

    let stored = store.get(&key).await.expect("get").expect("value present");
    assert_eq!(stored.expose(), "value-1");

    store.delete(&key).await.expect("cleanup");
}

async fn overwrite_replaces_the_value<S: SecretStore>(store: &S, namespace: &str) {
    let key = SecretKey::new(namespace, "overwrite");
    store.set(&key, Secret::new("first")).await.expect("set");
    store
        .set(&key, Secret::new("second"))
        .await
        .expect("overwrite");

    let stored = store.get(&key).await.expect("get").expect("value present");
    assert_eq!(stored.expose(), "second", "set must replace, not append");

    store.delete(&key).await.expect("cleanup");
}

async fn delete_removes_the_value<S: SecretStore>(store: &S, namespace: &str) {
    let key = SecretKey::new(namespace, "delete");
    store.set(&key, Secret::new("value")).await.expect("set");
    store.delete(&key).await.expect("delete");

    assert!(store.get(&key).await.expect("get").is_none());
}

async fn delete_is_idempotent<S: SecretStore>(store: &S, namespace: &str) {
    let key = SecretKey::new(namespace, "delete-twice");
    store.set(&key, Secret::new("value")).await.expect("set");

    store.delete(&key).await.expect("first delete");
    store
        .delete(&key)
        .await
        .expect("deleting a missing key must succeed");
}

async fn keys_are_isolated<S: SecretStore>(store: &S, namespace: &str) {
    let one = SecretKey::new(namespace, "isolated-a");
    let two = SecretKey::new(namespace, "isolated-b");

    store.set(&one, Secret::new("a")).await.expect("set a");
    store.set(&two, Secret::new("b")).await.expect("set b");

    assert_eq!(
        store.get(&one).await.expect("get a").expect("a").expose(),
        "a"
    );
    assert_eq!(
        store.get(&two).await.expect("get b").expect("b").expose(),
        "b"
    );

    store.delete(&one).await.expect("cleanup a");
    store.delete(&two).await.expect("cleanup b");
}

async fn concurrent_writes_do_not_corrupt<S: SecretStore>(store: &S, namespace: &str) {
    let key = SecretKey::new(namespace, "concurrent");

    let (first, second) = tokio::join!(
        store.set(&key, Secret::new("writer-a")),
        store.set(&key, Secret::new("writer-b"))
    );
    first.expect("concurrent set a");
    second.expect("concurrent set b");

    let stored = store.get(&key).await.expect("get").expect("value present");
    assert!(
        matches!(stored.expose(), "writer-a" | "writer-b"),
        "a concurrent write must leave one whole value, got {stored:?}"
    );

    store.delete(&key).await.expect("cleanup");
}
