use super::{status_error, TranslationProvider};
use serde_json::Value;
pub struct BaiduProvider;
impl TranslationProvider for BaiduProvider {
    fn id(&self) -> &'static str {
        "baidu"
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
        Ok(p.get("trans_result")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|i| i.get("dst").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default())
    }
}
