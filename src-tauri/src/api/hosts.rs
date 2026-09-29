use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
const BEGIN: &str = "# LARK HOSTS BEGIN";
const END: &str = "# LARK HOSTS END";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostEntry {
    pub ip: String,
    pub hostname: String,
    pub comment: Option<String>,
    pub enabled: bool,
}
fn hosts_path() -> &'static Path {
    Path::new(r"C:\Windows\System32\drivers\etc\hosts")
}
fn validate_entry(e: &HostEntry) -> Result<(), String> {
    if e.ip.trim().is_empty() || e.hostname.trim().is_empty() {
        return Err("IP 地址和域名不能为空".into());
    }
    e.ip.parse::<std::net::IpAddr>()
        .map_err(|_| "IP 地址格式无效".to_string())?;
    let h = e.hostname.trim();
    if h.len() > 253
        || h.starts_with('.')
        || h.ends_with('.')
        || h.contains("..")
        || !h
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    {
        return Err(format!("域名格式无效：{h}"));
    }
    Ok(())
}
fn parse_lines(content: &str) -> Vec<HostEntry> {
    content
        .lines()
        .filter_map(|raw| {
            let raw = raw.trim();
            if raw.is_empty() {
                return None;
            }
            let enabled = !raw.starts_with('#');
            let line = raw.trim_start_matches('#').trim();
            let (body, comment) = line.split_once('#').map_or((line, None), |(b, c)| {
                (b.trim(), Some(c.trim().to_string()))
            });
            let mut parts = body.split_whitespace();
            let ip = parts.next();
            let hostname = parts.next();
            if let (Some(ip), Some(hostname)) = (ip, hostname) {
                if ip.parse::<std::net::IpAddr>().is_ok()
                    && !hostname.is_empty()
                    && hostname.len() <= 253
                    && !hostname.starts_with('.')
                    && !hostname.ends_with('.')
                    && !hostname.contains("..")
                    && hostname
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
                {
                    return Some(HostEntry {
                        ip: ip.into(),
                        hostname: hostname.into(),
                        comment,
                        enabled,
                    });
                }
            }
            if !enabled {
                return Some(HostEntry {
                    ip: String::new(),
                    hostname: String::new(),
                    comment: Some(line.to_string()),
                    enabled: false,
                });
            }
            None
        })
        .collect()
}
fn parse_managed(content: &str) -> Vec<HostEntry> {
    let Some(start) = content.lines().position(|l| l.trim() == BEGIN) else {
        return vec![];
    };
    let lines: Vec<&str> = content.lines().collect();
    let end = lines
        .iter()
        .skip(start + 1)
        .position(|l| l.trim() == END)
        .map(|i| i + start + 1);
    let Some(end) = end else { return vec![] };
    parse_lines(&lines[start + 1..end].join("\n"))
}
#[tauri::command]
pub fn read_hosts() -> Result<Vec<HostEntry>, String> {
    #[cfg(not(target_os = "windows"))]
    return Err("Hosts 编辑器目前仅支持 Windows".into());
    #[cfg(target_os = "windows")]
    fs::read_to_string(hosts_path())
        .map(|c| parse_managed(&c))
        .map_err(|e| format!("读取 hosts 失败：{e}"))
}
#[tauri::command]
pub fn read_hosts_all() -> Result<Vec<HostEntry>, String> {
    #[cfg(not(target_os = "windows"))]
    return Err("Hosts 编辑器目前仅支持 Windows".into());
    #[cfg(target_os = "windows")]
    fs::read_to_string(hosts_path())
        .map(|c| parse_lines(&c))
        .map_err(|e| format!("读取 hosts 失败：{e}"))
}
fn write_file(content: String) -> Result<(), String> {
    let path = hosts_path();
    let temp = path.with_extension("lark.tmp");
    use std::io::Write;
    let mut file = fs::File::create(&temp).map_err(|e| format!("写入临时 hosts 文件失败：{e}"))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("写入临时 hosts 文件失败：{e}"))?;
    file.sync_all()
        .map_err(|e| format!("刷新 hosts 文件失败：{e}"))?;
    drop(file);
    fs::rename(&temp, path).map_err(|e| {
        let _ = fs::remove_file(&temp);
        format!("保存 hosts 失败，请以管理员身份运行：{e}")
    })
}
#[tauri::command(rename_all = "camelCase")]
pub fn write_hosts_all(entries: Vec<HostEntry>) -> Result<(), String> {
    #[cfg(not(target_os = "windows"))]
    return Err("Hosts 编辑器目前仅支持 Windows".into());
    #[cfg(target_os = "windows")]
    {
        for e in &entries {
            if e.enabled {
                validate_entry(e)?;
            }
        }
        let content = entries
            .iter()
            .map(|e| {
                if !e.enabled && e.ip.trim().is_empty() && e.hostname.trim().is_empty() {
                    format!("# {}", e.comment.as_deref().unwrap_or_default())
                } else {
                    format!(
                        "{}{} {}{}",
                        if e.enabled { "" } else { "# " },
                        e.ip.trim(),
                        e.hostname.trim(),
                        e.comment
                            .as_deref()
                            .filter(|c| !c.is_empty())
                            .map(|c| format!(" # {c}"))
                            .unwrap_or_default()
                    )
                }
            })
            .collect::<Vec<_>>()
            .join("\r\n");
        write_file(content + "\r\n")
    }
}
#[tauri::command(rename_all = "camelCase")]
pub fn write_hosts(entries: Vec<HostEntry>) -> Result<(), String> {
    #[cfg(not(target_os = "windows"))]
    return Err("Hosts 编辑器目前仅支持 Windows".into());
    #[cfg(target_os = "windows")]
    {
        let mut seen = std::collections::HashSet::new();
        for e in &entries {
            validate_entry(e)?;
            if !seen.insert(e.hostname.to_ascii_lowercase()) {
                return Err(format!("域名重复：{}", e.hostname));
            }
        }
        let path = hosts_path();
        let original = fs::read_to_string(path).map_err(|e| format!("读取 hosts 失败：{e}"))?;
        let lines: Vec<&str> = original.lines().collect();
        let start = lines.iter().position(|l| l.trim() == BEGIN);
        let end = start.and_then(|s| {
            lines
                .iter()
                .skip(s + 1)
                .position(|l| l.trim() == END)
                .map(|i| i + s + 1)
        });
        let block = std::iter::once(BEGIN.to_string())
            .chain(entries.iter().map(|e| {
                format!(
                    "{}{} {}{}",
                    if e.enabled { "" } else { "# " },
                    e.ip.trim(),
                    e.hostname.trim(),
                    e.comment
                        .as_deref()
                        .filter(|c| !c.is_empty())
                        .map(|c| format!(" # {c}"))
                        .unwrap_or_default()
                )
            }))
            .chain(std::iter::once(END.to_string()))
            .collect::<Vec<_>>()
            .join("\r\n");
        let next = match (start, end) {
            (Some(s), Some(e)) => lines[..s].join("\r\n") + &block + &lines[e + 1..].join("\r\n"),
            _ => original.trim_end_matches(['\r', '\n']).to_string() + "\r\n\r\n" + &block + "\r\n",
        };
        write_file(next)
    }
}
#[tauri::command]
pub fn read_hosts_raw() -> Result<String, String> {
    #[cfg(not(target_os = "windows"))]
    return Err("Hosts 编辑器目前仅支持 Windows".into());
    #[cfg(target_os = "windows")]
    fs::read_to_string(hosts_path()).map_err(|e| format!("读取 hosts 失败：{e}"))
}
#[tauri::command]
pub fn write_hosts_raw(content: String) -> Result<(), String> {
    #[cfg(not(target_os = "windows"))]
    return Err("Hosts 编辑器目前仅支持 Windows".into());
    #[cfg(target_os = "windows")]
    write_file(content)
}
#[tauri::command]
pub fn flush_dns() -> Result<(), String> {
    #[cfg(not(target_os = "windows"))]
    return Err("DNS 刷新目前仅支持 Windows".into());
    #[cfg(target_os = "windows")]
    std::process::Command::new("ipconfig")
        .arg("/flushdns")
        .output()
        .map_err(|e| e.to_string())
        .and_then(|o| {
            if o.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&o.stderr).to_string())
            }
        })
}
