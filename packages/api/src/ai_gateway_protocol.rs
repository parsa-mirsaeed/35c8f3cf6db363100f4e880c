//! Internal application-to-AI-gateway wire types.
//!
//! Provider credentials and destination URLs are intentionally absent. The
//! application may select only the configured model/profile; the gateway owns
//! every external destination and secret.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayEmbeddingRequest {
    pub model: String,
    pub input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayEmbeddingResponse {
    pub object: String,
    pub data: Vec<GatewayEmbeddingData>,
    pub model: String,
    #[serde(default)]
    pub usage: Option<GatewayUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayEmbeddingData {
    pub object: String,
    pub embedding: Vec<f32>,
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GatewayUsage {
    #[serde(default)]
    pub prompt_tokens: Option<u32>,
    #[serde(default)]
    pub completion_tokens: Option<u32>,
    #[serde(default)]
    pub total_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayChatRequest {
    pub model: String,
    pub messages: Vec<GatewayChatMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<GatewayResponseFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayChatResponse {
    pub model: String,
    pub choices: Vec<GatewayChatChoice>,
    #[serde(default)]
    pub usage: Option<GatewayUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayChatChoice {
    pub index: usize,
    pub message: GatewayChatMessage,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayErrorEnvelope {
    pub error: GatewayErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatewayErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
}
