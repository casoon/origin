//! The product's IPC surface.
//!
//! Commands resolve state, delegate and translate errors. No logic lives here.

use crate::example::ExampleService;
use origin_tauri::{CommandError, OriginState};
use tauri::State;

#[tauri::command]
pub async fn example_greeting(state: State<'_, OriginState>) -> Result<String, CommandError> {
    Ok(state
        .application()
        .require::<ExampleService>()?
        .greeting()
        .await?)
}
