//! The platform IPC surface.
//!
//! Commands are the only way the frontend causes anything to happen. They are thin:
//! they resolve state, call the domain, and translate errors — no logic lives here.

use crate::state::OriginState;
use origin_app::AppInfo;
use origin_connector::ConnectorDescriptor;
use origin_domain::{Account, AccountId, AppError, ErrorContract, Health, Job, JobId};
use origin_sync::{SyncStatus, SyncTarget};
use serde::Serialize;
use tauri::{AppHandle, State};
use time::format_description::well_known::Rfc3339;

/// Error payload for IPC.
///
/// Wraps [`ErrorContract`] so the frontend always receives the same shape and never a
/// raw `rusqlite`, `reqwest` or `tauri` error (ADR-0002).
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct CommandError(ErrorContract);

impl From<AppError> for CommandError {
    fn from(error: AppError) -> Self {
        tracing::warn!(kind = ?error.kind(), %error, "command failed");
        Self(error.to_contract())
    }
}

type CommandResult<T> = Result<T, CommandError>;

#[tauri::command]
pub async fn origin_app_info(
    app: AppHandle,
    state: State<'_, OriginState>,
) -> CommandResult<AppInfo> {
    let package = app.package_info();
    Ok(AppInfo {
        id: state.config().app_id.clone(),
        name: package.name.clone(),
        version: package.version.to_string(),
        modules: state
            .application()
            .modules()
            .iter()
            .map(|module| (*module).to_owned())
            .collect(),
    })
}

#[tauri::command]
pub async fn origin_setting_get(
    state: State<'_, OriginState>,
    key: String,
) -> CommandResult<Option<serde_json::Value>> {
    Ok(state
        .application()
        .platform()
        .settings
        .get_json(&key)
        .await?)
}

#[tauri::command]
pub async fn origin_setting_set(
    state: State<'_, OriginState>,
    key: String,
    value: serde_json::Value,
) -> CommandResult<()> {
    state
        .application()
        .platform()
        .settings
        .set_json(&key, &value)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn origin_settings_customised(
    state: State<'_, OriginState>,
) -> CommandResult<Vec<String>> {
    Ok(state
        .application()
        .platform()
        .settings
        .customised_keys()
        .await?)
}

/// Open an external URL.
///
/// Fails with a permission error when the product did not wire an [`Opener`] into its
/// composition root — capabilities are granted at build time, not requested at runtime.
///
/// [`Opener`]: origin_platform::Opener
#[tauri::command]
pub async fn origin_open_url(state: State<'_, OriginState>, url: String) -> CommandResult<()> {
    let application = state.application();
    let opener = application.platform().opener.as_ref().ok_or_else(|| {
        AppError::Permission("this application cannot open external urls".to_owned())
    })?;

    opener.open_url(&url).await?;
    Ok(())
}

/// Every connected account, across all connectors.
#[tauri::command]
pub async fn origin_accounts(state: State<'_, OriginState>) -> CommandResult<Vec<Account>> {
    Ok(state.application().platform().accounts.list().await?)
}

/// Remove an account and its credentials.
///
/// Data a module cached for the account is not removed here — only the module knows
/// its namespaces (see `AccountService::disconnect`).
#[tauri::command]
pub async fn origin_account_disconnect(
    state: State<'_, OriginState>,
    account: String,
) -> CommandResult<()> {
    state
        .application()
        .platform()
        .accounts
        .disconnect(&AccountId::new(account))
        .await?;
    Ok(())
}

/// What this build can connect to.
///
/// Compiled in, so the list is fixed: a running application cannot gain a connector
/// (ADR-0006).
#[tauri::command]
pub async fn origin_connectors(
    state: State<'_, OriginState>,
) -> CommandResult<Vec<ConnectorDescriptor>> {
    Ok(state
        .application()
        .platform()
        .connectors
        .iter()
        .map(|connector| connector.descriptor())
        .collect())
}

/// Jobs the application knows about, newest first.
#[tauri::command]
pub async fn origin_jobs(state: State<'_, OriginState>) -> CommandResult<Vec<Job>> {
    Ok(state.application().platform().jobs.list().await)
}

/// Ask a job to stop.
///
/// Returns as soon as the request is recorded — a job decides itself when it can stop
/// safely, so the UI must keep watching its status rather than assuming it ended.
#[tauri::command]
pub async fn origin_job_cancel(state: State<'_, OriginState>, job: String) -> CommandResult<()> {
    state
        .application()
        .platform()
        .jobs
        .cancel(&JobId::new(job))
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn origin_sync_status(state: State<'_, OriginState>) -> CommandResult<Vec<SyncStatus>> {
    let application = state.application();
    let engine = &application.platform().sync;
    let now = application.platform().clock.now();

    let mut statuses = Vec::new();
    for target in engine.targets() {
        let sync_state = engine.state(&target).await?;
        let due_at = engine.due_at(&target).await.ok();

        statuses.push(SyncStatus {
            health: origin_sync::health_of(
                &sync_state,
                &engine.policy(&target).unwrap_or_default(),
                now,
            ),
            state: sync_state,
            due_at: due_at.and_then(|at| at.format(&Rfc3339).ok()),
            target,
        });
    }

    Ok(statuses)
}

/// Refresh one target now.
///
/// Explicit user intent, so this bypasses the throttle that protects against
/// automatic triggers.
#[tauri::command]
pub async fn origin_sync_now(
    state: State<'_, OriginState>,
    target: SyncTarget,
) -> CommandResult<()> {
    state
        .application()
        .platform()
        .sync
        .sync_now(&target)
        .await?;
    Ok(())
}

/// Overall health across every registered sync target.
#[tauri::command]
pub async fn origin_health(state: State<'_, OriginState>) -> CommandResult<Health> {
    Ok(state.application().platform().sync.health().await)
}

/// Exercises commands through a real, Tauri-managed `State` rather than by calling
/// their bodies with a hand-built value — the part `cargo check` alone cannot prove,
/// namely that `OriginState` is actually reachable as Tauri state the way the real host
/// wires it in `lib.rs`. No window is started; [`tauri::test::mock_app`] provides the
/// managed-state machinery without one.
///
/// Commands taking an `AppHandle` are not covered here: that parameter is the default,
/// `Wry`-backed [`tauri::AppHandle`], which a [`tauri::test::MockRuntime`]-backed app
/// cannot produce.
///
/// Windows only: `tauri::test::mock_app()` crashes the whole test binary there with
/// `STATUS_ENTRYPOINT_NOT_FOUND`, a known unresolved upstream issue
/// (tauri-apps/tauri#11028, #13419, #13948, #13954) — this module is not compiled on
/// that target, and the `tauri` dev-dependency's `test` feature is not enabled there
/// either (see Cargo.toml).
#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;
    use crate::HostConfig;
    use crate::state::OriginState;
    use origin_app::ApplicationBuilder;
    use tauri::Manager;
    use tokio_util::sync::CancellationToken;

    fn mock_app_with_state() -> tauri::App<tauri::test::MockRuntime> {
        let application = ApplicationBuilder::in_memory()
            .build()
            .expect("an in-memory application always builds");

        let app = tauri::test::mock_app();
        app.manage(OriginState::new(
            application,
            HostConfig::new("dev.origin.tests"),
            CancellationToken::new(),
        ));
        app
    }

    #[tokio::test]
    async fn health_reads_through_the_managed_state_to_the_sync_engine() {
        let app = mock_app_with_state();

        let health = origin_health(app.state()).await.unwrap();

        assert_eq!(health, Health::Unknown, "no sync targets are registered");
    }

    #[tokio::test]
    async fn a_setting_written_through_one_command_is_read_back_through_another() {
        let app = mock_app_with_state();

        origin_setting_set(app.state(), "theme".to_owned(), serde_json::json!("dark"))
            .await
            .unwrap();

        let value = origin_setting_get(app.state(), "theme".to_owned())
            .await
            .unwrap();

        assert_eq!(value, Some(serde_json::json!("dark")));
    }

    #[tokio::test]
    async fn a_freshly_built_application_has_no_accounts_or_jobs() {
        let app = mock_app_with_state();

        assert!(origin_accounts(app.state()).await.unwrap().is_empty());
        assert!(origin_jobs(app.state()).await.unwrap().is_empty());
    }
}
