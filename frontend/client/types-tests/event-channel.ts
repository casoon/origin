/**
 * Type-level tests for `eventChannel`, checked by `pnpm check` (tsc --noEmit).
 *
 * They exist because the property under test is a *typing* property — that the
 * payload narrows per event name — which no runtime test can observe. Kept
 * outside `src/` so it is not part of the published package (`files: ["src"]`).
 */
import { eventChannel, onEvent } from "../src/transport";

type Events = {
  "demo://progress": { current: number; total: number };
  "demo://done": { ok: boolean };
};

const on = eventChannel<Events>();

// The payload narrows to the entry for that exact event name.
on("demo://progress", (payload) => {
  const current: number = payload.current;
  void current;
});
on("demo://done", (payload) => {
  const ok: boolean = payload.ok;
  void ok;
});

// An event name outside the map is rejected.
// @ts-expect-error "demo://typo" is not a key of Events
on("demo://typo", () => {});

// A field from a *different* event is rejected.
on("demo://done", (payload) => {
  // @ts-expect-error `current` belongs to "demo://progress", not "demo://done"
  void payload.current;
});

// Regression guard for why `eventChannel` exists at all: naming `Events` on
// `onEvent` turns off inference for the name, so the payload widens to the
// union of every payload in the map and this field access cannot compile.
onEvent<Events>("demo://progress", (payload) => {
  // @ts-expect-error payload is the union of all payloads, not the narrow one
  void payload.current;
});
