/**
 * This product's slice of the client.
 *
 * One typed function per command, so no component ever calls `command()` with a raw
 * string — let alone `invoke()` (ADR-0010).
 */

import { command } from "@origin/client";

export function greeting(): Promise<string> {
  return command<string>("example_greeting");
}
