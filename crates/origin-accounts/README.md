# origin-accounts

Account management for Origin: several accounts per connector, credentials kept out of the database.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_accounts::{AccountService, AccountStore};
use origin_auth::{TokenSet, TokenStore};
use origin_domain::ConnectorId;

let accounts = AccountService::new(
    AccountStore::new(storage.clone(), clock.clone()),
    TokenStore::new(secrets.clone()),
    events.clone(),
    storage.clone(),
    clock.clone(),
);

// `tokens` normally comes from AuthorizationFlow::exchange().
let account = accounts
    .connect(&ConnectorId::new("github"), "octocat", &tokens)
    .await?;

let all = accounts.list().await?;
accounts.disconnect(&account.id).await?;
```

## Stability

Pre-1.0 (`0.1.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
