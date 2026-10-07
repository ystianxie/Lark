//! Provider response adapters kept independent from the Tauri command boundary.
use serde_json::Value;
mod baidu;
mod niutrans;
mod openai;
mod tengxun;
pub trait TranslationProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn parse_response(
        &self,
        status: reqwest::StatusCode,
        payload: &Value,
    ) -> Result<String, String>;
}
pub fn provider_for(id: &str) -> Option<Box<dyn TranslationProvider>> {
    match id {
        "niutrans" => Some(Box::new(niutrans::NiuTransProvider)),
        "baidu" => Some(Box::new(baidu::BaiduProvider)),
        "tengxun" => Some(Box::new(tengxun::TencentProvider)),
        "deepseek" => Some(Box::new(openai::OpenAiProvider { id: "deepseek" })),
        "zhipu" => Some(Box::new(openai::OpenAiProvider { id: "zhipu" })),
        _ => None,
    }
}
fn status_error(status: reqwest::StatusCode, payload: &Value) -> String {
    let message = payload
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(Value::as_str)
        .or_else(|| payload.get("error_msg").and_then(Value::as_str))
        .unwrap_or("服务返回错误");
    format!("翻译请求失败（HTTP {status}）：{message}")
}
