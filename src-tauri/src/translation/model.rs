use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationRequest {
    pub provider: String,
    pub text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub service: serde_json::Value,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationResponse {
    pub text: String,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
}
