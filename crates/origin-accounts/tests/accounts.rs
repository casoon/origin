use origin_accounts::{AccountService, AccountStore};
use origin_auth::{TokenSet, TokenStore};
use origin_core::testing::FakeClock;
use origin_core::{AccountStatus, Clock, ConnectorId};
use origin_events::{EventBus, PlatformEvent};
use origin_secrets::{MemorySecretStore, Secret};
use origin_storage::{MemoryStorage, Record, Storage, StorageKey, namespace};
use std::sync::Arc;
use time::macros::datetime;

struct Harness {
    service: AccountService,
    tokens: TokenStore,
    events: EventBus,
    storage: Arc<MemoryStorage>,
    clock: Arc<dyn Clock>,
}

fn harness() -> Harness {
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(datetime!(2026-08-23 10:00 UTC)));
    let storage = Arc::new(MemoryStorage::new());
    let accounts = AccountStore::new(storage.clone(), clock.clone());
    let tokens = TokenStore::new(Arc::new(MemorySecretStore::new()));
    let events = EventBus::new();

    Harness {
        service: AccountService::new(
            accounts,
            tokens.clone(),
            events.clone(),
            storage.clone(),
            clock.clone(),
        ),
        tokens,
        events,
        storage,
        clock,
    }
}

fn token_set(access: &str) -> TokenSet {
    TokenSet {
        access_token: Secret::new(access),
        refresh_token: None,
        token_type: "Bearer".to_owned(),
        expires_at: None,
        scopes: vec!["repo".to_owned()],
    }
}

#[tokio::test]
async fn one_connector_can_hold_several_accounts() {
    let harness = harness();
    let github = ConnectorId::new("github");

    let work = harness
        .service
        .connect(&github, "work", &token_set("at-work"))
        .await
        .unwrap();
    let personal = harness
        .service
        .connect(&github, "personal", &token_set("at-personal"))
        .await
        .unwrap();

    assert_ne!(work.id, personal.id);

    let accounts = harness.service.list_for(&github).await.unwrap();
    assert_eq!(accounts.len(), 2);
    assert_eq!(
        accounts
            .iter()
            .map(|a| a.display_name.as_str())
            .collect::<Vec<_>>(),
        vec!["personal", "work"],
        "accounts are listed in a stable order"
    );
}

#[tokio::test]
async fn accounts_of_different_connectors_do_not_mix() {
    let harness = harness();
    let github = ConnectorId::new("github");
    let cloudflare = ConnectorId::new("cloudflare");

    harness
        .service
        .connect(&github, "gh", &token_set("a"))
        .await
        .unwrap();
    harness
        .service
        .connect(&cloudflare, "cf", &token_set("b"))
        .await
        .unwrap();

    assert_eq!(harness.service.list_for(&github).await.unwrap().len(), 1);
    assert_eq!(
        harness.service.list_for(&cloudflare).await.unwrap().len(),
        1
    );
    assert_eq!(harness.service.list().await.unwrap().len(), 2);
}

#[tokio::test]
async fn connecting_stores_credentials_addressed_by_account() {
    let harness = harness();
    let github = ConnectorId::new("github");

    let work = harness
        .service
        .connect(&github, "work", &token_set("at-work"))
        .await
        .unwrap();
    let personal = harness
        .service
        .connect(&github, "personal", &token_set("at-personal"))
        .await
        .unwrap();

    assert_eq!(
        harness
            .tokens
            .load(&github, &work.id)
            .await
            .unwrap()
            .unwrap()
            .access_token
            .expose(),
        "at-work"
    );
    assert_eq!(
        harness
            .tokens
            .load(&github, &personal.id)
            .await
            .unwrap()
            .unwrap()
            .access_token
            .expose(),
        "at-personal"
    );
}

#[tokio::test]
async fn disconnecting_removes_the_record_and_the_credentials() {
    let harness = harness();
    let github = ConnectorId::new("github");

    let work = harness
        .service
        .connect(&github, "work", &token_set("a"))
        .await
        .unwrap();
    let personal = harness
        .service
        .connect(&github, "personal", &token_set("b"))
        .await
        .unwrap();

    harness.service.disconnect(&work.id).await.unwrap();

    assert!(
        harness
            .tokens
            .load(&github, &work.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(harness.service.list().await.unwrap().len(), 1);
    assert!(
        harness
            .tokens
            .load(&github, &personal.id)
            .await
            .unwrap()
            .is_some(),
        "revoking one account must not touch another"
    );
}

#[tokio::test]
async fn disconnecting_an_unknown_account_is_a_validation_error() {
    let harness = harness();

    let error = harness
        .service
        .disconnect(&origin_core::AccountId::new("nope"))
        .await
        .unwrap_err();

    assert_eq!(error.kind(), origin_core::ErrorKind::Validation);
}

#[tokio::test]
async fn expiring_an_account_publishes_an_event_once() {
    let harness = harness();
    let github = ConnectorId::new("github");
    let mut events = harness.events.subscribe::<PlatformEvent>().unwrap();

    let account = harness
        .service
        .connect(&github, "work", &token_set("a"))
        .await
        .unwrap();
    harness.service.mark_expired(&account.id).await.unwrap();

    let event = events.recv().await.unwrap();
    assert!(matches!(event, PlatformEvent::AccountExpired(_)));

    assert!(!harness.service.get(&account.id).await.unwrap().is_usable());

    // A second report of the same problem must stay quiet.
    harness.service.mark_expired(&account.id).await.unwrap();
    assert!(
        events.try_recv().is_err(),
        "an already expired account must not publish again"
    );
}

#[tokio::test]
async fn expired_credentials_are_kept_so_a_provider_outage_costs_nothing() {
    let harness = harness();
    let github = ConnectorId::new("github");

    let account = harness
        .service
        .connect(&github, "work", &token_set("a"))
        .await
        .unwrap();
    harness.service.mark_expired(&account.id).await.unwrap();

    assert!(
        harness
            .tokens
            .load(&github, &account.id)
            .await
            .unwrap()
            .is_some(),
        "marking expired must not delete credentials"
    );

    harness.service.mark_active(&account.id).await.unwrap();
    assert_eq!(
        harness.service.get(&account.id).await.unwrap().status,
        AccountStatus::Active
    );
}

#[tokio::test]
async fn disconnecting_removes_everything_stored_under_the_account() {
    let harness = harness();
    let github = ConnectorId::new("github");

    let work = harness
        .service
        .connect(&github, "work", &token_set("a"))
        .await
        .unwrap();
    let personal = harness
        .service
        .connect(&github, "personal", &token_set("b"))
        .await
        .unwrap();

    // Two modules cache data for the account, without telling anyone.
    for area in ["notifications", "sync"] {
        harness
            .storage
            .put(
                &StorageKey::new(namespace::account(&github, &work.id, area), "k"),
                Record::new("1", harness.clock.now()),
            )
            .await
            .unwrap();
    }
    let other = StorageKey::new(namespace::account(&github, &personal.id, "sync"), "k");
    harness
        .storage
        .put(&other, Record::new("2", harness.clock.now()))
        .await
        .unwrap();

    harness.service.disconnect(&work.id).await.unwrap();

    for area in ["notifications", "sync"] {
        assert!(
            harness
                .storage
                .get(&StorageKey::new(
                    namespace::account(&github, &work.id, area),
                    "k"
                ))
                .await
                .unwrap()
                .is_none(),
            "{area} data must be gone without the module registering anything"
        );
    }
    assert!(
        harness.storage.get(&other).await.unwrap().is_some(),
        "the other account's data must survive"
    );
}
