/**
 * `@origin/client` — the frontend's view of the Rust core.
 *
 * Views import from here. They never import `@tauri-apps/api` directly, so the
 * transport can change without touching a single component (ADR-0010).
 */

import { command } from "./transport";
import type {
  Account,
  AppInfo,
  ConnectorDescriptor,
  Health,
  Job,
  SyncStatus,
  SyncTarget,
} from "./types";

export { command, onPlatformEvent } from "./transport";
export { OriginError, toOriginError } from "./errors";
export type * from "./types";

/** Product identity, version and the modules that were compiled in. */
export function appInfo(): Promise<AppInfo> {
  return command<AppInfo>("origin_app_info");
}

/** User settings. Values are whatever the Rust side stores under that key. */
export const settings = {
  get<T>(key: string): Promise<T | null> {
    return command<T | null>("origin_setting_get", { key });
  },

  set<T>(key: string, value: T): Promise<void> {
    return command<void>("origin_setting_set", { key, value });
  },

  /** Keys that differ from their default. */
  customised(): Promise<string[]> {
    return command<string[]>("origin_settings_customised");
  },
};

/** Connected accounts, across all connectors. */
export const accounts = {
  list(): Promise<Account[]> {
    return command<Account[]>("origin_accounts");
  },

  /**
   * Remove an account and its credentials.
   *
   * Data a module cached for the account is not removed by this call.
   */
  disconnect(account: string): Promise<void> {
    return command<void>("origin_account_disconnect", { account });
  },
};

/** What this build can connect to. */
export function connectors(): Promise<ConnectorDescriptor[]> {
  return command<ConnectorDescriptor[]>("origin_connectors");
}

/** Background jobs: progress, cancellation, uniform lifecycle. */
export const jobs = {
  list(): Promise<Job[]> {
    return command<Job[]>("origin_jobs");
  },

  /**
   * Ask a job to stop.
   *
   * Resolves once the request is recorded, not once the job ended — keep watching its
   * status.
   */
  cancel(job: string): Promise<void> {
    return command<void>("origin_job_cancel", { job });
  },
};

/** The sync engine: what is registered, how it is doing, and when it runs next. */
export const sync = {
  status(): Promise<SyncStatus[]> {
    return command<SyncStatus[]>("origin_sync_status");
  },

  /** Refresh one target now. Bypasses the throttle — this is explicit user intent. */
  now(target: SyncTarget): Promise<void> {
    return command<void>("origin_sync_now", { target });
  },
};

/** Overall health across every registered sync target. */
export function health(): Promise<Health> {
  return command<Health>("origin_health");
}

/**
 * Open an http(s) URL in the user's browser.
 *
 * Rejects with `kind: "permission"` when the product did not grant itself the
 * capability, and with `kind: "validation"` for anything that is not http(s).
 */
export function openUrl(url: string): Promise<void> {
  return command<void>("origin_open_url", { url });
}
