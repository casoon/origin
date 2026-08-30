// Generated from PulseSnapshot in src-tauri/src/pulse.rs. Do not edit.

import type { Alert, Health, Metric } from "@origin/client";

export type PulseSnapshot = { health: Health, metric: Metric | null, alerts: Array<Alert>, };
