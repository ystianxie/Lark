use super::{status_error, TranslationProvider};
use serde_json::Value;
pub struct TencentProvider;
impl TranslationProvider for TencentProvider {
    fn id(&self) -> &'static str {
        "tengxun"
    }
    fn parse_response(&self, status: reqwest::StatusCode, p: &Value) -> Result<String, String> {
        if !status.is_success() {
            return Err(status_error(status, p));
        }
        if let Some(e) = p.get("Response").and_then(|r| r.get("Error")) {
            let c = e
                .get("Code")
                .and_then(Value::as_str)
                .unwrap_or("TencentCloudError");
            let m = e.get("Message").and_then(Value::as_str).unwrap_or(c);
            return Err(format!("腾讯翻译君错误（{c}）：{m}"));
        }
        Ok(p.get("Response")
            .and_then(|r| r.get("TargetText"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into())
    }
}
