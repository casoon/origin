use origin_app::Application;
use origin_events::{PlatformEvent, RecvError};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

/// Event name the frontend listens on. Only `@origin/client` uses it (ADR-0010).
pub(crate) const PLATFORM_EVENT: &str = "origin://platform-event";

/// Forward platform events from the in-process bus to the webview.
///
/// This is a one-way bridge: the frontend observes, it does not publish. Anything the
/// UI wants to *cause* goes through a command, so the domain stays in control.
pub(crate) fn forward_platform_events(app: &AppHandle, application: Arc<Application>) {
    let Ok(mut stream) = application.platform().events.subscribe::<PlatformEvent>() else {
        tracing::error!("cannot subscribe to platform events, ui will not receive updates");
        return;
    };

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            match stream.recv().await {
                Ok(event) => {
                    if let Err(error) = app.emit(PLATFORM_EVENT, &event) {
                        tracing::warn!(%error, "cannot forward platform event to the webview");
                    }
                }
                // The UI is a view of current state, so dropping backlog is acceptable —
                // but it must be visible in the log when it happens.
                Err(RecvError::Lagged(missed)) => {
                    tracing::warn!(missed, "ui event bridge fell behind");
                }
                Err(RecvError::Closed) => break,
            }
        }
        tracing::debug!("platform event bridge stopped");
    });
}
