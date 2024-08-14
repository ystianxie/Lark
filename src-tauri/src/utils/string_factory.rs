use base64::engine::general_purpose;
use base64::Engine;
use crypto::digest::Digest;
use crypto::md5::Md5;
use pinyin::ToPinyin;

pub fn base64_encode(bytes: &[u8]) -> String {
    general_purpose::STANDARD.encode(bytes)
}

pub fn base64_decode(base64: &str) -> Vec<u8> {
    general_purpose::STANDARD.decode(base64).unwrap()
}

pub fn md5(s: &str) -> String {
    let mut hasher = Md5::new();
    hasher.input_str(s);
    hasher.result_str()
}

#[allow(unused)]
pub fn md5_by_bytes(bytes: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.input(bytes);
    hasher.result_str()
}

#[allow(unused)]
pub fn text_to_pinyin(text: &str) -> (String, String) {
    let mut pinyin = String::new();
    let mut abb = String::new();
    for (i, p) in text.to_pinyin().enumerate() {
        match p {
            Some(char) => {
                pinyin.push_str(&char.plain());
                abb.push_str(&char.first_letter());
            }
            None => {
                let ori_char = text.chars().nth(i).unwrap();
                pinyin.push_str(ori_char.to_string().as_str());
                abb.push_str(ori_char.to_string().as_str());
            }
        }
    }
    (pinyin, abb)
}