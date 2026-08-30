/**
 * The demo's own slice of the client.
 *
 * Products extend `@origin/client` like this — one typed function per command — so no
 * component ever calls `command()` with a raw string, let alone `invoke()`.
 */

import { command } from "@origin/client";
import type { PulseSnapshot } from "./pulse.generated";

export type { PulseSnapshot } from "./pulse.generated";

export const pulse = {
  /** Current state, from cache. Cheap. */
  snapshot(): Promise<PulseSnapshot> {
    return command<PulseSnapshot>("demo_snapshot");
  },

  /** Fetch a new reading. This is what a sync would do in a real product. */
  refresh(): Promise<PulseSnapshot> {
    return command<PulseSnapshot>("demo_refresh");
  },
};
