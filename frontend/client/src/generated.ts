// Generated from the Rust contracts by `cargo xtask generate`. Do not edit.
//
// Every type here crosses the IPC boundary. Changing one in Rust and forgetting this
// file is what `cargo xtask generate --check` exists to catch.

export type Account = { id: AccountId, connector: ConnectorId, 
/**
 * What the user sees. Never a token, never an internal handle.
 */
display_name: string, status: AccountStatus, connected_at: string, };

export type AccountExpired = { account: AccountId, connector: ConnectorId, };

export type AccountId = string;

export type AccountIdentity = { 
/**
 * The service's own identifier — a GitHub login, a GA4 property id.
 */
external_id: string, 
/**
 * What to show the user.
 */
display_name: string, 
/**
 * Scopes the service reports as actually granted, which can be fewer than
 * requested. Surfacing this is how a product explains a missing feature.
 */
granted_scopes: Array<string>, };

export type AccountStatus = "active" | "expired" | "disconnected";

export type Alert = { id: AlertId, 
/**
 * Stable identity of *the problem*, not of this occurrence. Two raises with the
 * same fingerprint are the same alert, so the user is not notified twice.
 */
fingerprint: string, severity: Severity, title: string, body: string | null, connector: ConnectorId | null, account: AccountId | null, state: AlertState, raised_at: string, resolved_at: string | null, };

export type AlertId = string;

export type AlertRaised = { alert: Alert, 
/**
 * `true` when an alert with the same fingerprint was already active, so
 * notification sinks can stay quiet.
 */
deduplicated: boolean, };

export type AlertResolved = { alert: AlertId, at: string, };

export type AlertState = "active" | "acknowledged" | "resolved" | "silenced";

export type AppInfo = { id: string, name: string, version: string, 
/**
 * Modules compiled into this build, in registration order.
 */
modules: Array<string>, };

export type AuthKind = "o_auth2" | "personal_access_token" | "none";

export type ConnectorDescriptor = { id: ConnectorId, display_name: string, auth: AuthKind, 
/**
 * The rights this connector needs at the external service.
 *
 * Declared, not inferred: a reviewer can see in one place whether an integration
 * asks for write access, and a product can refuse to ship one that does.
 */
required_permissions: Array<ProductPermission>, 
/**
 * Whether the user may connect several accounts (ADR-0016).
 */
supports_multiple_accounts: boolean, };

export type ConnectorId = string;

export type ErrorContract = { kind: ErrorKind, message: string, retryable: boolean, needs_user_action: boolean, retry_after_seconds: number | null, };

export type ErrorKind = "authentication" | "permission" | "network" | "offline" | "rate_limited" | "storage" | "external_service" | "validation" | "configuration" | "internal";

export type Health = "healthy" | "warning" | "critical" | "unknown";

export type Job = { id: JobId, 
/**
 * Product-defined job kind, e.g. `scan-repository`.
 */
kind: string, status: JobStatus, progress: Progress, cancelable: boolean, started_at: string, finished_at: string | null, error: string | null, };

export type JobFinished = { job: JobId, kind: string, status: JobStatus, error: string | null, };

export type JobId = string;

export type JobProgress = { job: JobId, current: number, total: number | null, };

export type JobStarted = { job: JobId, kind: string, };

export type JobStatus = "queued" | "running" | "succeeded" | "failed" | "cancelled";

export type Metric = { key: MetricKey, value: number, unit: Unit, at: string, };

export type MetricKey = string;

export type PlatformEvent = { "type": "sync_completed" } & SyncCompleted | { "type": "sync_failed" } & SyncFailed | { "type": "alert_raised" } & AlertRaised | { "type": "alert_resolved" } & AlertResolved | { "type": "account_expired" } & AccountExpired | { "type": "job_started" } & JobStarted | { "type": "job_progress" } & JobProgress | { "type": "job_finished" } & JobFinished;

export type PlatformPermission = "filesystem" | "shell" | "process" | "notifications" | "credential_store" | "global_shortcut" | "autostart";

export type ProductPermission = { "read": { scope: string, } } | { "write": { scope: string, } };

export type Progress = { current: number, 
/**
 * `None` while the total is not yet known — the UI shows an indeterminate bar.
 */
total: number | null, };

export type Severity = "info" | "warning" | "critical";

export type SyncCompleted = { sync: SyncId, connector: ConnectorId, account: AccountId, 
/**
 * How many records changed. `0` means the service reported no change.
 */
changed: number, at: string, };

export type SyncFailed = { sync: SyncId, connector: ConnectorId, account: AccountId, kind: ErrorKind, message: string, 
/**
 * When the platform intends to try again, if it does.
 */
retry_at: string | null, };

export type SyncId = string;

export type SyncOutcome = { "outcome": "updated" } | { "outcome": "not_modified" } | { "outcome": "failed", kind: ErrorKind, message: string, };

export type SyncState = { last_attempt: string | null, last_success: string | null, last_outcome: SyncOutcome | null, 
/**
 * Validators handed back to the service on the next request.
 */
etag: string | null, last_modified: string | null, 
/**
 * Consecutive failures, used for exponential backoff.
 */
failure_streak: number, };

export type SyncStatus = { target: SyncTarget, state: SyncState, health: Health, 
/**
 * When the engine intends to run it next, RFC 3339.
 */
due_at: string | null, };

export type SyncTarget = { connector: ConnectorId, account: AccountId, name: string, };

export type Trend = { current: number, previous: number, };

export type Unit = "count" | "percent" | "bytes" | "milliseconds" | "per_minute" | { "custom": string };
