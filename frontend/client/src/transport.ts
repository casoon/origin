import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { toOriginError } from "./errors";
import type { PlatformEvent } from "./types";

/** Event channel the host bridge emits on. Must match `origin_tauri::bridge`. */
const PLATFORM_EVENT = "origin://platform-event";

/**
 * Call a Rust command.
 *
 * This is the single place in the whole frontend that calls `invoke`. Products build
 * their own typed wrappers on top of it (ADR-0010) rather than importing Tauri APIs
 * into views.
 */
export async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(name, args);
  } catch (thrown) {
    throw toOriginError(thrown);
  }
}

/**
 * Subscribe to platform events.
 *
 * The returned function unsubscribes; call it from a component teardown.
 */
export async function onPlatformEvent(
  handler: (event: PlatformEvent) => void,
): Promise<UnlistenFn> {
  return listen<PlatformEvent>(PLATFORM_EVENT, (event) => handler(event.payload));
}

/**
 * Subscribe to a product-specific window event emitted directly via `app.emit(...)`
 * on the Rust side (as opposed to a typed [`PlatformEvent`] published on the event bus
 * and forwarded through {@link onPlatformEvent}).
 *
 * Products with their own long-running, high-frequency progress reporting (a crawl, an
 * import) that does not fit the platform's job/event model still need a transport —
 * this keeps that need from forcing a direct `@tauri-apps/api` import into a view
 * (ADR-0010). The returned function unsubscribes; call it from a component teardown.
 *
 * `name` and its payload type are declared together, in one place, as a product's own
 * event map — not chosen independently at each call site the way a bare
 * `onEvent<T>(name: string, ...)` would allow:
 *
 * ```ts
 * type MyEvents = { "myapp://crawl-progress": { current: number; total: number } };
 * onEvent<MyEvents>("myapp://crawl-progress", (payload) => { ... }); // payload is typed
 * ```
 *
 * Without an explicit event map, `payload` infers as `unknown` rather than silently
 * accepting whatever type a caller names — a typed product wrapper is expected to
 * supply the map once and re-export a narrower function views call instead.
 */
export async function onEvent<Events extends Record<string, unknown> = Record<string, unknown>>(
  name: keyof Events & string,
  handler: (payload: Events[keyof Events & typeof name]) => void,
): Promise<UnlistenFn> {
  return listen<Events[keyof Events & typeof name]>(name, (event) => handler(event.payload));
}
