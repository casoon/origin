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
 * `onEvent<T>(name: string, ...)` would allow.
 *
 * **Prefer {@link eventChannel} over calling this directly.** Naming `Events` here
 * turns off inference for the event name, so `payload` widens to the union of every
 * payload in the map and each handler has to narrow it again:
 *
 * ```ts
 * type MyEvents = { "a": { x: number }; "b": { y: string } };
 * onEvent<MyEvents>("a", (payload) => payload.x); // payload is { x: number } | { y: string }
 * ```
 *
 * That is a TypeScript limitation, not a choice: a function cannot take the map
 * explicitly and still infer the key, because supplying any type argument falls the
 * rest back to their defaults. Binding the map first — which is what `eventChannel`
 * does — leaves the key free to infer.
 *
 * Without an explicit event map, `payload` infers as `unknown` rather than silently
 * accepting whatever type a caller names.
 */
export async function onEvent<Events extends Record<string, unknown> = Record<string, unknown>>(
  name: keyof Events & string,
  handler: (payload: Events[keyof Events & typeof name]) => void,
): Promise<UnlistenFn> {
  return listen<Events[keyof Events & typeof name]>(name, (event) => handler(event.payload));
}

/**
 * Bind a product's event map once and get back a subscribe function that infers each
 * payload from the event name.
 *
 * This is the "typed product wrapper" the platform expects each product to declare, in
 * the one form that actually narrows:
 *
 * ```ts
 * type MyEvents = {
 *   "myapp://crawl-progress": { current: number; total: number };
 *   "myapp://crawl-done": { total: number };
 * };
 *
 * export const onAppEvent = eventChannel<MyEvents>();
 *
 * onAppEvent("myapp://crawl-progress", (payload) => payload.current); // narrow, no cast
 * onAppEvent("myapp://typo", () => {});                               // compile error
 * ```
 *
 * The returned function resolves to an unsubscribe callback; call it from a component
 * teardown, exactly as with {@link onEvent}.
 */
export function eventChannel<Events extends Record<string, unknown>>() {
  return <Name extends keyof Events & string>(
    name: Name,
    handler: (payload: Events[Name]) => void,
  ): Promise<UnlistenFn> =>
    listen<Events[Name]>(name, (event) => handler(event.payload));
}
