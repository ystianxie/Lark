use base64::{engine::general_purpose, Engine as _};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::Duration;

const DEFAULT_TIMEOUT_MS: u64 = 8_000;
const MAX_TIMEOUT_MS: u64 = 30_000;
const MIN_TIMEOUT_MS: u64 = 3_000;
// Baidu's form endpoint documents a 4 MiB limit for the Base64 image field.
const MAX_BAIDU_IMAGE_BASE64_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrRequest {
    pub provider: String,
    pub image_path: String,
    pub service: Value,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrResponse {
    pub text: String,
    pub provider: String,
}

fn config_value<'a>(config: &'a Value, key: &str) -> Option<&'a Value> {
    config
        .get(key)
        .or_else(|| config.get("config").and_then(|nested| nested.get(key)))
}

fn config_string(config: &Value, key: &str) -> String {
    let value = config_value(config, key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if let Some(ciphertext) = value.strip_prefix("dpapi:") {
        crate::utils::dpapi::unprotect(ciphertext).unwrap_or_default()
    } else {
        value
    }
}

fn config_bool(config: &Value, key: &str) -> bool {
    config_value(config, key)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn form_encode(values: &BTreeMap<String, String>) -> String {
    fn encode(value: &str) -> String {
        value.bytes().fold(String::new(), |mut output, byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                output.push(byte as char);
            } else if byte == b' ' {
                output.push('+');
            } else {
                output.push_str(&format!("%{byte:02X}"));
            }
            output
        })
    }
    values
        .iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn response_json(
    response: reqwest::blocking::Response,
    provider: &str,
    action: &str,
) -> Result<Value, String> {
    let status = response.status();
    let body = response
        .text()
        .map_err(|error| format!("读取{provider} {action}响应失败：{error}"))?;
    let payload: Value = serde_json::from_str(&body)
        .map_err(|error| format!("解析{provider} {action}响应失败：{error}"))?;
    if !status.is_success() {
        let detail = payload
            .get("error_description")
            .and_then(Value::as_str)
            .or_else(|| payload.get("error_msg").and_then(Value::as_str))
            .or_else(|| payload.get("msg").and_then(Value::as_str))
            .unwrap_or("服务返回错误");
        return Err(format!(
            "{provider} {action}失败（HTTP {status}）：{detail}"
        ));
    }
    let has_provider_error = payload
        .get("error_code")
        .map(|code| {
            code.as_i64() != Some(0) && code.as_str().map(|value| value != "0").unwrap_or(true)
        })
        .unwrap_or(false);
    if has_provider_error {
        let error_code = payload.get("error_code").unwrap_or(&Value::Null);
        let detail = payload
            .get("error_msg")
            .and_then(Value::as_str)
            .or_else(|| payload.get("msg").and_then(Value::as_str))
            .unwrap_or("服务返回错误");
        return Err(format!("{provider} {action}失败（{error_code}）：{detail}"));
    }
    Ok(payload)
}

fn recognize_baidu_blocking(request: OcrRequest) -> Result<OcrResponse, String> {
    let api_key = config_string(&request.service, "apiKey").trim().to_string();
    let secret_key = config_string(&request.service, "secretKey")
        .trim()
        .to_string();
    if api_key.is_empty() || secret_key.is_empty() {
        return Err("百度 OCR 需要填写 API Key 和 Secret Key".to_string());
    }

    let image = crate::api::screenshot::read_screenshot_bytes(&request.image_path)?;
    let image_base64 = general_purpose::STANDARD.encode(image);
    if image_base64.len() > MAX_BAIDU_IMAGE_BASE64_BYTES {
        return Err("截图过大，百度 OCR 要求上传图片不超过 4 MB".to_string());
    }

    let timeout = Duration::from_millis(
        request
            .timeout_ms
            .unwrap_or(DEFAULT_TIMEOUT_MS)
            .clamp(MIN_TIMEOUT_MS, MAX_TIMEOUT_MS),
    );
    let client = Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| format!("创建百度 OCR 请求失败：{error}"))?;

    let mut token_params = BTreeMap::new();
    token_params.insert("grant_type".to_string(), "client_credentials".to_string());
    token_params.insert("client_id".to_string(), api_key);
    token_params.insert("client_secret".to_string(), secret_key);
    let token_endpoint = format!(
        "https://aip.baidubce.com/oauth/2.0/token?{}",
        form_encode(&token_params)
    );
    let token_payload = response_json(
        client
            .get(token_endpoint)
            .send()
            .map_err(|error| format!("百度 OCR 鉴权请求失败：{error}"))?,
        "百度 OCR",
        "鉴权",
    )?;
    let access_token = token_payload
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "百度 OCR 鉴权响应缺少 access_token".to_string())?;

    let endpoint = if config_bool(&request.service, "useAccurate") {
        "https://aip.baidubce.com/rest/2.0/ocr/v1/accurate_basic"
    } else {
        "https://aip.baidubce.com/rest/2.0/ocr/v1/general_basic"
    };
    let mut image_params = BTreeMap::new();
    image_params.insert("image".to_string(), image_base64);
    let endpoint = format!("{endpoint}?access_token={access_token}");
    let ocr_payload = response_json(
        client
            .post(endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_encode(&image_params))
            .send()
            .map_err(|error| format!("百度 OCR 识别请求失败：{error}"))?,
        "百度 OCR",
        "识别",
    )?;

    let text = ocr_payload
        .get("words_result")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("words").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    if text.trim().is_empty() {
        return Err("百度 OCR 未识别到文字".to_string());
    }
    Ok(OcrResponse {
        text,
        provider: request.provider,
    })
}

fn parse_ddddocr_response(payload: &Value) -> Result<String, String> {
    let code = payload
        .get("code")
        .and_then(|value| value.as_i64().or_else(|| value.as_str()?.parse().ok()))
        .ok_or_else(|| "ddddocr-rust 响应缺少 code".to_string())?;
    if code != 200 {
        let message = payload
            .get("msg")
            .and_then(Value::as_str)
            .unwrap_or("服务返回错误");
        return Err(format!("ddddocr-rust 识别失败（{code}）：{message}"));
    }
    let text = payload
        .get("data")
        .and_then(|data| data.get("text"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if text.is_empty() {
        return Err("ddddocr-rust 未识别到文字".to_string());
    }
    Ok(text)
}

fn recognize_ddddocr_blocking(request: OcrRequest) -> Result<OcrResponse, String> {
    let endpoint = {
        let configured = config_string(&request.service, "endpoint");
        if configured.trim().is_empty() {
            "http://127.0.0.1:8000/ocr".to_string()
        } else {
            configured.trim().to_string()
        }
    };
    let image = crate::api::screenshot::read_screenshot_bytes(&request.image_path)?;
    let image_base64 = general_purpose::STANDARD.encode(image);
    let timeout = Duration::from_millis(
        request
            .timeout_ms
            .unwrap_or(DEFAULT_TIMEOUT_MS)
            .clamp(MIN_TIMEOUT_MS, MAX_TIMEOUT_MS),
    );
    let client = Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| format!("创建 ddddocr-rust 请求失败：{error}"))?;
    let body = serde_json::json!({"image": image_base64});
    let body = serde_json::to_vec(&body)
        .map_err(|error| format!("生成 ddddocr-rust 请求失败：{error}"))?;
    let payload = response_json(
        client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .map_err(|error| format!("ddddocr-rust 识别请求失败：{error}"))?,
        "ddddocr-rust",
        "识别",
    )?;
    let text = parse_ddddocr_response(&payload)?;
    Ok(OcrResponse {
        text,
        provider: request.provider,
    })
}

fn recognize_blocking(request: OcrRequest) -> Result<OcrResponse, String> {
    match request.provider.as_str() {
        "baidu-ocr" => recognize_baidu_blocking(request),
        "ddddocr-rust" => recognize_ddddocr_blocking(request),
        "paddleocr-local" => Err(format!("暂不支持的 OCR 服务：{}", request.provider)),
        provider => Err(format!("暂不支持的 OCR 服务：{provider}")),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn ocr_image(request: OcrRequest) -> Result<OcrResponse, String> {
    tauri::async_runtime::spawn_blocking(move || recognize_blocking(request))
        .await
        .map_err(|error| format!("OCR 任务执行失败：{error}"))?
}

#[cfg(test)]
mod tests {
    use super::{form_encode, parse_ddddocr_response};
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn form_encoding_is_url_safe_for_base64() {
        let values = BTreeMap::from([(String::from("image"), String::from("a+b/c=="))]);
        assert_eq!(form_encode(&values), "image=a%2Bb%2Fc%3D%3D");
    }

    #[test]
    fn parses_ddddocr_success_payload() {
        let payload = json!({
            "code": 200,
            "msg": "success",
            "data": {"text": "九乘六等于？", "probability": null}
        });
        assert_eq!(parse_ddddocr_response(&payload).unwrap(), "九乘六等于？");
    }

    #[test]
    fn reports_ddddocr_error_payload() {
        let payload = json!({"code": 400, "msg": "invalid image", "data": null});
        assert_eq!(
            parse_ddddocr_response(&payload).unwrap_err(),
            "ddddocr-rust 识别失败（400）：invalid image"
        );
    }
}
