//! The demo's own IPC surface.
//!
//! Product commands sit next to the platform commands in one invoke handler (see the
//! `origin_handler!` macro) and follow the same rule: resolve, delegate, translate.

use crate::pulse::{PulseService, PulseSnapshot};
use origin_tauri::{CommandError, OriginState};
use tauri::State;

#[tauri::command]
pub async fn demo_snapshot(state: State<'_, OriginState>) -> Result<PulseSnapshot, CommandError> {
    Ok(state
        .application()
        .require::<PulseService>()?
        .snapshot()
        .await?)
}

#[tauri::command]
pub async fn demo_refresh(state: State<'_, OriginState>) -> Result<PulseSnapshot, CommandError> {
    Ok(state
        .application()
        .require::<PulseService>()?
        .refresh()
        .await?)
}
