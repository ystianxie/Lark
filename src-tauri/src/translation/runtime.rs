use super::model::{TranslationRequest, TranslationResponse};
use chrono::{TimeZone, Utc};
use crypto::digest::Digest;
use crypto::hmac::Hmac;
use crypto::mac::Mac;
use crypto::sha2::Sha256;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn translation_config_string(config: &serde_json::Value, key: &str) -> String {
    let value = config
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    if let Some(ciphertext) = value.strip_prefix("dpapi:") {
        crate::utils::dpapi::unprotect(ciphertext).unwrap_or_default()
    } else {
        value
    }
}

fn translation_config_bool(config: &serde_json::Value, key: &str) -> bool {
    config
        .get(key)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn translation_form_encode(params: &HashMap<String, String>) -> String {
    fn encode(value: &str) -> String {
        value.bytes().fold(String::new(), |mut output, byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                output.push(byte as char);
            } else {
                output.push_str(&format!("%{byte:02X}"));
            }
            output
        })
    }
    params
        .iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn translation_sha256_hex(value: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.input(value);
    digest.result_str()
}

fn translation_hmac_sha256(key: &[u8], value: &str) -> Vec<u8> {
    let mut hmac = Hmac::new(Sha256::new(), key);
    hmac.input(value.as_bytes());
    hmac.result().code().to_vec()
}

fn translation_hmac_sha256_hex(key: &[u8], value: &str) -> String {
    translation_hmac_sha256(key, value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

const MAX_TRANSLATION_CHARS: usize = 20_000;
const TRANSLATION_CHUNK_CHARS: usize = 1_500;

fn split_translation_text(text: &str, max_chars: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let hard_end = (start + max_chars).min(chars.len());
        if hard_end == chars.len() {
            chunks.push(chars[start..hard_end].iter().collect());
            break;
        }
        let mut end = hard_end;
        // Prefer a natural boundary in the latter half of the chunk. Include
        // the boundary character so punctuation and whitespace are preserved.
        for index in (start + max_chars / 2..hard_end).rev() {
            if matches!(
                chars[index - 1],
                '\n' | '\r' | '。' | '！' | '？' | '；' | '.' | '!' | '?' | ';' | ' ' | '\t'
            ) {
                end = index;
                break;
            }
        }
        if end <= start {
            end = hard_end;
        }
        chunks.push(chars[start..end].iter().collect());
        start = end;
    }
    chunks
}

fn translate_text_chunk_blocking(
    request: TranslationRequest,
) -> Result<TranslationResponse, String> {
    if request.text.trim().is_empty() {
        return Err("翻译文本为空".to_string());
    }
    let timeout = Duration::from_millis(request.timeout_ms.unwrap_or(8_000).clamp(3_000, 30_000));
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| format!("创建翻译请求失败：{error}"))?;
    let source_lang = if request.source_lang.is_empty() {
        "auto"
    } else if request.source_lang == "zh-CN" {
        "zh"
    } else {
        &request.source_lang
    };
    let target_lang = if request.target_lang == "zh-CN" {
        "zh"
    } else {
        &request.target_lang
    };
    let config = &request.service;
    let provider = request.provider.as_str();

    let (response, source, target) = match provider {
        "niutrans" => {
            let mut params = HashMap::from([
                ("from".to_string(), source_lang.to_string()),
                ("to".to_string(), target_lang.to_string()),
                (
                    "apikey".to_string(),
                    translation_config_string(config, "apiKey"),
                ),
                ("src_text".to_string(), request.text.clone()),
            ]);
            for key in ["dictNo", "memoryNo", "dict"] {
                let value = translation_config_string(config, key);
                if !value.is_empty() {
                    params.insert(key.to_string(), value);
                }
            }
            if translation_config_bool(config, "dictflag") {
                params.insert("dictflag".to_string(), "1".to_string());
            }
            let endpoint = "https://api.niutrans.com/NiuTransServer/translation";
            let encoded = translation_form_encode(&params);
            let builder = if request.text.chars().count() > 1500 {
                client
                    .post(endpoint)
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(encoded)
            } else {
                client.get(format!("{endpoint}?{encoded}"))
            };
            (
                builder
                    .send()
                    .map_err(|error| format!("小牛翻译请求失败：{error}"))?,
                source_lang.to_string(),
                target_lang.to_string(),
            )
        }
        "baidu" => {
            let app_id = translation_config_string(config, "appId");
            let app_key = translation_config_string(config, "appKey");
            if app_id.is_empty() || app_key.is_empty() {
                return Err("百度翻译需要填写 APP ID 和密钥".to_string());
            }
            let salt = format!(
                "{}{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis(),
                std::process::id()
            );
            let sign = crate::utils::string_factory::md5(&format!(
                "{app_id}{}{salt}{app_key}",
                request.text
            ));
            let mut params = HashMap::from([
                ("q".to_string(), request.text.clone()),
                ("from".to_string(), source_lang.to_string()),
                ("to".to_string(), target_lang.to_string()),
                ("appid".to_string(), app_id),
                ("salt".to_string(), salt),
                ("sign".to_string(), sign),
            ]);
            if translation_config_bool(config, "needIntervene") {
                params.insert("needIntervene".to_string(), "1".to_string());
            }
            let encoded = translation_form_encode(&params);
            (
                client
                    .post("https://fanyi-api.baidu.com/api/trans/vip/translate")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(encoded)
                    .send()
                    .map_err(|error| format!("百度翻译请求失败：{error}"))?,
                source_lang.to_string(),
                target_lang.to_string(),
            )
        }
        "tengxun" => {
            let secret_id = translation_config_string(config, "secretId")
                .trim()
                .to_string();
            let secret_key = translation_config_string(config, "secretKey")
                .trim()
                .to_string();
            if secret_id.is_empty() || secret_key.is_empty() {
                return Err("腾讯翻译君需要填写 SecretId 和 SecretKey".to_string());
            }
            let host = "tmt.tencentcloudapi.com";
            let service_name = "tmt";
            let version = "2018-03-21";
            let action = "TextTranslate";
            let region = {
                let value = translation_config_string(config, "region");
                if value.is_empty() {
                    "ap-guangzhou".to_string()
                } else {
                    value.trim().to_string()
                }
            };
            let project_id = translation_config_string(config, "projectId")
                .parse::<i64>()
                .unwrap_or(0);
            let source = if source_lang == "auto" {
                "auto"
            } else {
                source_lang
            };
            let body = serde_json::json!({
                "SourceText": request.text,
                "Source": source,
                "Target": target_lang,
                "ProjectId": project_id
            });
            let body = serde_json::to_string(&body)
                .map_err(|error| format!("生成腾讯翻译君请求失败：{error}"))?;
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let date = Utc
                .timestamp_opt(timestamp, 0)
                .single()
                .ok_or_else(|| "生成腾讯翻译君签名日期失败".to_string())?
                .format("%Y-%m-%d")
                .to_string();
            let canonical_headers =
                format!("content-type:application/json; charset=utf-8\nhost:{host}\n");
            let signed_headers = "content-type;host";
            let canonical_request = format!(
                "POST\n/\n\n{canonical_headers}\n{signed_headers}\n{}",
                translation_sha256_hex(body.as_bytes())
            );
            let credential_scope = format!("{date}/{service_name}/tc3_request");
            let string_to_sign = format!(
                "TC3-HMAC-SHA256\n{timestamp}\n{credential_scope}\n{}",
                translation_sha256_hex(canonical_request.as_bytes())
            );
            let secret_date = translation_hmac_sha256(format!("TC3{secret_key}").as_bytes(), &date);
            let secret_service = translation_hmac_sha256(&secret_date, service_name);
            let secret_signing = translation_hmac_sha256(&secret_service, "tc3_request");
            let signature = translation_hmac_sha256_hex(&secret_signing, &string_to_sign);
            let authorization = format!(
                "TC3-HMAC-SHA256 Credential={secret_id}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
            );
            (
                client
                    .post(format!("https://{host}"))
                    .header("Content-Type", "application/json; charset=utf-8")
                    .header("Host", host)
                    .header("X-TC-Action", action)
                    .header("X-TC-Version", version)
                    .header("X-TC-Region", region)
                    .header("X-TC-Timestamp", timestamp.to_string())
                    .header("Authorization", authorization)
                    .body(body)
                    .send()
                    .map_err(|error| format!("腾讯翻译君请求失败：{error}"))?,
                source_lang.to_string(),
                target_lang.to_string(),
            )
        }
        "deepseek" | "zhipu" => {
            let api_key = translation_config_string(config, "apiKey");
            if api_key.is_empty() {
                return Err(if provider == "zhipu" {
                    "智谱未配置 API Key".to_string()
                } else {
                    "DeepSeek 未配置 API Key".to_string()
                });
            }
            let base_url = {
                let value = translation_config_string(config, "baseUrl");
                if value.is_empty() {
                    if provider == "zhipu" {
                        "https://open.bigmodel.cn/api/paas/v4".to_string()
                    } else {
                        "https://api.deepseek.com".to_string()
                    }
                } else {
                    value
                }
            };
            let endpoint = if base_url.ends_with("/chat/completions") {
                base_url
            } else {
                format!("{}/chat/completions", base_url.trim_end_matches('/'))
            };
            let model = {
                let value = translation_config_string(config, "model");
                if value.is_empty() {
                    if provider == "zhipu" {
                        "glm-5.3".to_string()
                    } else {
                        "deepseek-flash".to_string()
                    }
                } else {
                    value
                }
            };
            let provider_name = if provider == "zhipu" {
                "智谱"
            } else {
                "DeepSeek"
            };
            let target_name = match target_lang {
                "zh" | "zh-CN" => "Simplified Chinese",
                "en" => "English",
                "ja" => "Japanese",
                "ko" => "Korean",
                "fr" => "French",
                "es" => "Spanish",
                "ru" => "Russian",
                "ar" => "Arabic",
                "pt" => "Portuguese",
                "th" => "Thai",
                "mn" => "Mongolian (Cyrillic)",
                "de" => "German",
                "lo" => "Lao",
                "am" => "Amharic",
                "bn" => "Bengali",
                "cs" => "Czech",
                "da" => "Danish",
                "et" => "Estonian",
                "fi" => "Finnish",
                "fil" => "Filipino",
                "he" => "Hebrew",
                "hr" => "Croatian",
                "hu" => "Hungarian",
                "jv" => "Indonesian Java",
                "mg" => "Malagasy",
                "mi" => "Maori",
                "mk" => "Macedonian",
                "ms" => "Malay",
                "nl" => "Dutch",
                "no" => "Norwegian",
                "pl" => "Polish",
                "ro" => "Romanian",
                "sk" => "Slovak",
                "sl" => "Slovenian",
                "sm" => "Samoan",
                "sq" => "Albanian",
                "sr" => "Serbian",
                "su" => "Indonesian Sunda",
                "sv" => "Swedish",
                "sw" => "Swahili",
                "uk" => "Ukrainian",
                "vi" => "Vietnamese",
                "yo" => "Yoruba",
                "yue" => "Cantonese",
                "kk" => "Kazakh (Cyrillic)",
                "km" => "Khmer",
                "my" => "Burmese",
                "id" => "Indonesian",
                "ps" => "Pashto",
                "hi" => "Hindi",
                "fa" => "Persian",
                "ta" => "Tamil",
                "si" => "Sinhala",
                "it" => "Italian",
                "tr" => "Turkish",
                "bg" => "Bulgarian",
                "ckb" => "Kurdish (Sorani)",
                _ => target_lang,
            };
            let body = serde_json::json!({
                "model": model,
                "messages": [
                    {"role": "system", "content": "You are a translation engine. Return only the translated text, without explanations, notes, or quotation marks."},
                    {"role": "user", "content": format!("Translate the following text into {target_name}. Preserve the original meaning and formatting:\n\n{}", request.text)}
                ],
                "stream": false
            });
            let body = serde_json::to_string(&body)
                .map_err(|error| format!("生成 {provider_name} 请求失败：{error}"))?;
            (
                client
                    .post(endpoint)
                    .header("Content-Type", "application/json")
                    .header("Authorization", format!("Bearer {api_key}"))
                    .body(body)
                    .send()
                    .map_err(|error| format!("{provider_name} 翻译请求失败：{error}"))?,
                source_lang.to_string(),
                target_lang.to_string(),
            )
        }
        _ => return Err(format!("暂不支持的翻译服务：{provider}")),
    };

    let status = response.status();
    let response_body = response
        .text()
        .map_err(|error| format!("读取翻译响应失败：{error}"))?;
    let payload: serde_json::Value = serde_json::from_str(&response_body)
        .map_err(|error| format!("翻译响应解析失败：{error}"))?;
    let provider_adapter = super::providers::provider_for(provider)
        .ok_or_else(|| format!("暂不支持的翻译服务：{provider}"))?;
    let text = provider_adapter.parse_response(status, &payload)?;
    if text.is_empty() {
        return Err("翻译服务返回了空结果".to_string());
    }
    Ok(TranslationResponse {
        text,
        source_lang: Some(source),
        target_lang: Some(target),
    })
}

fn translate_text_blocking(request: TranslationRequest) -> Result<TranslationResponse, String> {
    let char_count = request.text.chars().count();
    if char_count > MAX_TRANSLATION_CHARS {
        return Err(format!(
            "翻译文本过长，最多支持 {MAX_TRANSLATION_CHARS} 个字符"
        ));
    }
    let chunks = split_translation_text(&request.text, TRANSLATION_CHUNK_CHARS);
    if chunks.len() <= 1 {
        return translate_text_chunk_blocking(request);
    }
    let mut translated = String::new();
    let mut source_lang = None;
    let mut target_lang = None;
    for chunk in chunks {
        let chunk_request = TranslationRequest {
            provider: request.provider.clone(),
            text: chunk,
            source_lang: request.source_lang.clone(),
            target_lang: request.target_lang.clone(),
            service: request.service.clone(),
            timeout_ms: request.timeout_ms,
        };
        let response = translate_text_chunk_blocking(chunk_request)?;
        translated.push_str(&response.text);
        source_lang = source_lang.or(response.source_lang);
        target_lang = target_lang.or(response.target_lang);
    }
    Ok(TranslationResponse {
        text: translated,
        source_lang,
        target_lang,
    })
}

#[tauri::command(rename_all = "camelCase")]
pub async fn translate_text(request: TranslationRequest) -> Result<TranslationResponse, String> {
    tauri::async_runtime::spawn_blocking(move || translate_text_blocking(request))
        .await
        .map_err(|error| format!("翻译任务执行失败：{error}"))?
}

#[cfg(test)]
mod tests {
    use super::{
        split_translation_text, translate_text_blocking, MAX_TRANSLATION_CHARS,
        TRANSLATION_CHUNK_CHARS,
    };
    use crate::translation::model::TranslationRequest;
    use serde_json::json;

    #[test]
    fn splits_by_natural_boundary_without_losing_characters() {
        let text = format!("{}。{}", "甲".repeat(1_200), "乙".repeat(500));
        let chunks = split_translation_text(&text, TRANSLATION_CHUNK_CHARS);
        assert!(chunks.len() >= 2);
        assert_eq!(chunks.concat(), text);
        assert!(chunks
            .iter()
            .all(|chunk| chunk.chars().count() <= TRANSLATION_CHUNK_CHARS));
        assert!(chunks[0].ends_with('。'));
    }

    #[test]
    fn keeps_short_text_in_one_chunk() {
        let chunks = split_translation_text("你好。", TRANSLATION_CHUNK_CHARS);
        assert_eq!(chunks, vec!["你好。".to_string()]);
    }

    #[test]
    fn rejects_text_over_total_limit_before_requesting_provider() {
        let request = TranslationRequest {
            provider: "unsupported".to_string(),
            text: "字".repeat(MAX_TRANSLATION_CHARS + 1),
            source_lang: "auto".to_string(),
            target_lang: "en".to_string(),
            service: json!({}),
            timeout_ms: None,
        };
        assert_eq!(
            translate_text_blocking(request).unwrap_err(),
            format!("翻译文本过长，最多支持 {MAX_TRANSLATION_CHARS} 个字符")
        );
    }
}
