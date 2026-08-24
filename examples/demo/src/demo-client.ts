/**
 * The demo's own slice of the client.
 *
 * Products extend `@origin/client` like this — one typed function per command — so no
 * component ever calls `command()` with a raw string, let alone `invoke()`.
 */

import { command, type Alert, type Health, type Metric } from "@origin/client";

export interface PulseSnapshot {
  health: Health;
  metric: Metric | null;
  alerts: Alert[];
}

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
