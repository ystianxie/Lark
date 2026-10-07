use super::{status_error, TranslationProvider};
use serde_json::Value;
pub struct OpenAiProvider {
    pub(crate) id: &'static str,
}
impl TranslationProvider for OpenAiProvider {
    fn id(&self) -> &'static str {
        self.id
    }
    fn parse_response(&self, status: reqwest::StatusCode, p: &Value) -> Result<String, String> {
        if !status.is_success() {
            return Err(status_error(status, p));
        }
        Ok(p.get("choices")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(|i| i.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .into())
    }
}
