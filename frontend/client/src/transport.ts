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
