//! The recorded request of a model probe: the messages and tool calls a probe sent a provider.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// The wire shapes
// ---------------------------------------------------------------------------

/// One chat message as sent to the provider — the OpenAI chat/completions shape, which is why an
/// assistant message may carry `tool_calls` and a `tool` message answers one by `tool_call_id`,
/// and why those two keys stay snake_case on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProbeMessage {
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "contract", ts(optional = nullable))]
    pub tool_calls: Vec<ProbeToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub tool_call_id: Option<String>,
}

/// One tool call on an assistant message, in the chat/completions wire shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProbeToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub function: ProbeToolFunction,
}

/// The function half of a tool call: the tool's name and its JSON-encoded arguments string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProbeToolFunction {
    pub name: String,
    /// The call's arguments as the JSON-encoded string the wire carries.
    pub arguments: String,
}

/// One case's request as sent: which (language, scenario, prompt) it probes, and its message
/// array. What `request_json` stores and the detail read returns, one entry per case.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProbeRequestOut {
    pub language: String,
    pub scenario: String,
    pub prompt: String,
    pub messages: Vec<ProbeMessage>,
}
