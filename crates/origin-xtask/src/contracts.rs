//! TypeScript bindings generated from the Rust contracts (§30).
//!
//! Hand-maintained mirrors of IPC types drift, silently, and the mismatch shows up as
//! `undefined` in production rather than as a build error. Every type that crosses the
//! IPC boundary is exported from its Rust definition instead.
//!
//! `cargo xtask generate --check` fails when the checked-in file no longer matches the
//! Rust types, so the drift becomes a red CI run.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use ts_rs::{Config, TS};

/// Where the generated bindings live. Part of `@origin/client`, because that is the
/// only package allowed to know the transport (ADR-0010).
const OUTPUT: &str = "frontend/client/src/generated.ts";

const HEADER: &str = "\
// Generated from the Rust contracts by `cargo xtask generate`. Do not edit.
//
// Every type here crosses the IPC boundary. Changing one in Rust and forgetting this
// file is what `cargo xtask generate --check` exists to catch.
";

/// Path of the generated bindings, relative to the workspace root.
pub(crate) fn output_path(root: &Path) -> PathBuf {
    root.join(OUTPUT)
}

/// Render the bindings for every contract type.
pub(crate) fn render() -> Result<String, String> {
    // `number`, not ts-rs's default `bigint`: serde_json writes u64 as a JSON number
    // and the IPC layer hands JavaScript a `number`. Declaring `bigint` would describe
    // a value that never arrives. Anything above 2^53 would lose precision on this
    // path regardless — a contract that needs such values has to carry them as strings.
    let config = Config::default().with_large_int("number");
    let mut declarations: Vec<(String, String)> = Vec::new();

    macro_rules! export {
        ($($type:ty),* $(,)?) => {
            $(
                declarations.push((
                    <$type as TS>::ident(&config),
                    <$type as TS>::decl(&config),
                ));
            )*
        };
    }

    export![
        // origin-domain: the domain primitives
        origin_domain::AccountId,
        origin_domain::AlertId,
        origin_domain::ConnectorId,
        origin_domain::JobId,
        origin_domain::SyncId,
        origin_domain::ErrorKind,
        origin_domain::ErrorContract,
        origin_domain::Health,
        origin_domain::Severity,
        origin_domain::AlertState,
        origin_domain::Alert,
        origin_domain::MetricKey,
        origin_domain::Unit,
        origin_domain::Metric,
        origin_domain::Trend,
        origin_domain::AccountStatus,
        origin_domain::Account,
        origin_domain::JobStatus,
        origin_domain::Progress,
        origin_domain::Job,
        origin_domain::SyncOutcome,
        origin_domain::SyncState,
        origin_domain::ProductPermission,
        origin_domain::PlatformPermission,
        // connectors
        origin_connector::AuthKind,
        origin_connector::ConnectorDescriptor,
        origin_connector::AccountIdentity,
        // sync
        origin_sync::SyncTarget,
        origin_sync::SyncStatus,
        // the application shell
        origin_app::AppInfo,
        // events reaching the webview
        origin_events::SyncCompleted,
        origin_events::SyncFailed,
        origin_events::AlertRaised,
        origin_events::AlertResolved,
        origin_events::AccountExpired,
        origin_events::JobStarted,
        origin_events::JobProgress,
        origin_events::JobFinished,
        origin_events::PlatformEvent,
    ];

    // Stable order so the file does not churn between runs.
    declarations.sort_by(|left, right| left.0.cmp(&right.0));
    declarations.dedup_by(|left, right| left.0 == right.0);

    let mut rendered = String::from(HEADER);
    for (_, declaration) in &declarations {
        let _ = write!(rendered, "\nexport {declaration}\n");
    }

    Ok(rendered)
}
