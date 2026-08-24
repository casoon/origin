/**
 * The Rust contracts, as TypeScript.
 *
 * Everything here comes from `generated.ts`, which `cargo xtask generate` derives from
 * the Rust type definitions (§30). Changing a type in Rust and forgetting the frontend
 * is a red CI run rather than an `undefined` in production.
 *
 * Do not hand-write a mirror of a Rust type in this file. If something is missing, add
 * it to the export list in `origin-xtask` instead.
 */

export type {
  Account,
  AccountExpired,
  AccountId,
  AccountIdentity,
  AccountStatus,
  Alert,
  AlertId,
  AlertRaised,
  AlertResolved,
  AlertState,
  AppInfo,
  AuthKind,
  ConnectorDescriptor,
  ConnectorId,
  ErrorContract,
  ErrorKind,
  Health,
  Job,
  JobFinished,
  JobId,
  JobProgress,
  JobStarted,
  JobStatus,
  Metric,
  MetricKey,
  PlatformEvent,
  PlatformPermission,
  ProductPermission,
  Progress,
  Severity,
  SyncCompleted,
  SyncFailed,
  SyncId,
  SyncOutcome,
  SyncState,
  SyncStatus,
  SyncTarget,
  Trend,
  Unit,
} from "./generated";
