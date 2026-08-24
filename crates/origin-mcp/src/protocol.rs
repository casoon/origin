//! The JSON-RPC 2.0 envelope MCP speaks.
//!
//! Only what this server needs: `initialize`, `tools/list`, `tools/call`. Those three
//! have been the stable core of MCP since the beginning.
//!
//! **Verify before connecting a real client.** The revision this was written against
//! is beyond the author's knowledge cutoff, and details — capability negotiation, the
//! exact result shapes, whether the transport is stateless — are the parts most likely
//! to have moved. The *boundary* in this crate is what matters architecturally; the
//! envelope is replaceable.

use serde::{Deserialize, Serialize};

/// Protocol revision this server announces.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Request {
    #[allow(dead_code)]
    #[serde(default = "jsonrpc_version")]
    pub jsonrpc: String,
    /// Absent for notifications, which take no response.
    #[serde(default)]
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

fn jsonrpc_version() -> String {
    "2.0".to_owned()
}

#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub jsonrpc: &'static str,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
}

/// JSON-RPC error codes, plus the one MCP-specific case this server produces.
pub(crate) mod codes {
    pub(crate) const INVALID_REQUEST: i32 = -32600;
    pub(crate) const METHOD_NOT_FOUND: i32 = -32601;
    pub(crate) const INVALID_PARAMS: i32 = -32602;
    pub(crate) const INTERNAL_ERROR: i32 = -32603;
}

impl Response {
    pub(crate) fn result(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub(crate) fn error(id: serde_json::Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(ResponseError {
                code,
                message: message.into(),
            }),
        }
    }
}
