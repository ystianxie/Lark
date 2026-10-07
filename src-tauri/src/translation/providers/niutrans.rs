use super::{status_error, TranslationProvider};
use serde_json::Value;
pub struct NiuTransProvider;
impl TranslationProvider for NiuTransProvider {
    fn id(&self) -> &'static str {
        "niutrans"
    }
    fn parse_response(&self, status: reqwest::StatusCode, p: &Value) -> Result<String, String> {
        if !status.is_success() {
            return Err(status_error(status, p));
        }
        if let Some(c) = p.get("error_code") {
            let c = c
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| c.to_string());
            if c != "0" && c != "\"0\"" {
                return Err(p
                    .get("error_msg")
                    .and_then(Value::as_str)
                    .unwrap_or(&c)
                    .into());
            }
        }
        Ok(p.get("tgt_text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into())
    }
}
