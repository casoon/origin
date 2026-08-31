//! What an external AI may do with this application (ADR-0027).
//!
//! Each tool wraps a *service method*. None of them reaches into Tauri, which is why
//! this works with no window open — and why a tool is never a copy of a command's
//! logic.

use crate::pulse::PulseService;
use async_trait::async_trait;
use origin_ai::AiService;
use origin_app::Application;
use origin_domain::{AppError, Result};
use origin_mcp::{AiPermission, AiPermissions, McpServer, Tool, ToolDescriptor, ToolOutput};
use origin_settings::Setting;
use serde_json::{Value, json};
use std::sync::Arc;

/// Kept in sync with the module's own setting.
const CRITICAL_ABOVE: Setting<f64> = Setting::new("demo.critical_above", || 85.0);

/// Assemble the server this product exposes.
///
/// The grant is read-and-propose: an external AI can look at things and prepare a
/// change, but nothing it does takes effect without a human. The commit tool below is
/// registered and still invisible, because the grant does not include it.
pub fn server(application: &Application) -> Result<McpServer> {
    let pulse = application.require::<PulseService>()?;
    let platform = application.platform().clone();

    Ok(McpServer::new("origin-demo", env!("CARGO_PKG_VERSION"))
        .with_permissions(AiPermissions::read_and_propose())
        .with_tool(Arc::new(StatusTool {
            pulse: pulse.clone(),
        }))
        .with_tool(Arc::new(ProposeThresholdTool {
            platform: platform.clone(),
        }))
        .with_tool(Arc::new(SetThresholdTool { platform })))
}

/// Read the current state.
#[derive(Debug)]
struct StatusTool {
    pulse: Arc<PulseService>,
}

#[async_trait]
impl Tool for StatusTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor::new(
            "demo.status",
            "Read status",
            "Returns the current load reading, the derived health state and any active \
             alerts. Use this before proposing a threshold change, so the proposal can \
             refer to the actual value.",
            AiPermission::Read,
        )
    }

    async fn call(&self, _arguments: Value) -> Result<ToolOutput> {
        let snapshot = self.pulse.snapshot().await?;
        let value = snapshot.metric.as_ref().map(|metric| metric.value);

        let text = match value {
            Some(value) => format!(
                "Load is {value:.1} %, health is {:?}, {} active alert(s).",
                snapshot.health,
                snapshot.alerts.len()
            ),
            None => "No reading yet.".to_owned(),
        };

        Ok(ToolOutput::text(text).with_structured(
            serde_json::to_value(&snapshot)
                .map_err(|error| AppError::internal(format!("cannot encode snapshot: {error}")))?,
        ))
    }
}

fn threshold_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "value": {
                "type": "number",
                "minimum": 0,
                "maximum": 100,
                "description": "The load percentage above which an alert is raised."
            }
        },
        "required": ["value"],
        "additionalProperties": false
    })
}

/// Read the requested threshold out of the arguments, rejecting anything unusable.
///
/// The arguments come from a model, so they are input, not a contract.
fn requested_threshold(arguments: &Value) -> Result<f64> {
    let value = arguments
        .get("value")
        .and_then(Value::as_f64)
        .ok_or_else(|| AppError::validation("`value` must be a number"))?;

    if !(0.0..=100.0).contains(&value) {
        return Err(AppError::validation(format!(
            "`value` must be between 0 and 100, got {value}"
        )));
    }

    Ok(value)
}

/// Prepare a change. Nothing is written.
#[derive(Debug)]
struct ProposeThresholdTool {
    platform: origin_app::Platform,
}

#[async_trait]
impl Tool for ProposeThresholdTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor::new(
            "demo.threshold.propose",
            "Propose a threshold",
            "Prepares a change to the critical load threshold and returns what it would \
             mean. Nothing is changed: a human confirms the proposal in the application.",
            AiPermission::Propose,
        )
        .with_schema(threshold_schema())
    }

    async fn call(&self, arguments: Value) -> Result<ToolOutput> {
        let requested = requested_threshold(&arguments)?;
        let current = self.platform.settings.get(&CRITICAL_ABOVE).await?;

        if (requested - current).abs() < f64::EPSILON {
            return Ok(ToolOutput::text(format!(
                "The critical threshold is already {current:.0} %. Nothing to change."
            )));
        }

        Ok(ToolOutput::text(format!(
            "Proposal: change the critical threshold from {current:.0} % to {requested:.0} %. \
             Confirm it in the application to apply it."
        ))
        .with_structured(json!({
            "setting": "demo.critical_above",
            "from": current,
            "to": requested,
            "applied": false
        })))
    }
}

/// Apply a change directly.
///
/// Registered on purpose while the grant excludes `Commit`: the tool is never listed
/// and a call is refused at the boundary. Switching it on is a deliberate act, visible
/// in one place.
#[derive(Debug)]
struct SetThresholdTool {
    platform: origin_app::Platform,
}

#[async_trait]
impl Tool for SetThresholdTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor::new(
            "demo.threshold.set",
            "Set a threshold",
            "Changes the critical load threshold immediately, without confirmation.",
            AiPermission::Commit,
        )
        .with_schema(threshold_schema())
    }

    async fn call(&self, arguments: Value) -> Result<ToolOutput> {
        let requested = requested_threshold(&arguments)?;
        self.platform
            .settings
            .set(&CRITICAL_ABOVE, &requested)
            .await?;

        Ok(ToolOutput::text(format!(
            "The critical threshold is now {requested:.0} %."
        )))
    }
}

/// The demo performs no inference of its own; this documents where it would go.
///
/// It is a separate port on purpose: MCP lets someone else's AI drive the application,
/// while this would let the application call a model. Conflating them is the mistake
/// ADR-0027 exists to prevent.
#[allow(dead_code)]
fn summariser(_ai: Arc<dyn AiService>) {}
