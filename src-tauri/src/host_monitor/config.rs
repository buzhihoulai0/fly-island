//! Persisted host monitor configuration and the access token kept in the
//! Windows credential store.

use keyring::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const DEFAULT_BASE_URL: &str = "http://192.168.31.236:8090";
pub const DEFAULT_INTERVAL_MS: u64 = 3_000;
pub const MIN_INTERVAL_MS: u64 = 1_000;
pub const MAX_INTERVAL_MS: u64 = 30_000;

const CONFIG_FILE_NAME: &str = "host-monitor.json";
const MAX_BASE_URL_LENGTH: usize = 240;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HostMonitorConfig {
    pub base_url: String,
    pub interval_ms: u64,
    pub enabled: bool,
    pub show_in_collapsed: bool,
}

impl Default for HostMonitorConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            interval_ms: DEFAULT_INTERVAL_MS,
            enabled: true,
            show_in_collapsed: true,
        }
    }
}

impl HostMonitorConfig {
    pub fn normalized(mut self) -> Self {
        self.base_url = self.base_url.trim().trim_end_matches('/').to_string();
        if self.base_url.chars().count() > MAX_BASE_URL_LENGTH {
            self.base_url.truncate(MAX_BASE_URL_LENGTH);
        }
        if !self.base_url.is_empty()
            && !self.base_url.starts_with("http://")
            && !self.base_url.starts_with("https://")
        {
            self.base_url = format!("http://{}", self.base_url);
        }
        self.interval_ms = self.interval_ms.clamp(MIN_INTERVAL_MS, MAX_INTERVAL_MS);
        self
    }
}

/// Configuration sent from the settings window. `token` is optional: leave it
/// out to keep the stored token, send an empty string to delete it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HostMonitorConfigInput {
    pub base_url: String,
    pub interval_ms: u64,
    pub enabled: bool,
    pub show_in_collapsed: bool,
    pub token: Option<String>,
}

pub fn config_path(app_config_dir: &Path) -> PathBuf {
    app_config_dir.join(CONFIG_FILE_NAME)
}

pub fn load_config(path: &Path) -> HostMonitorConfig {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<HostMonitorConfig>(&raw).ok())
        .unwrap_or_default()
        .normalized()
}

pub fn save_config(path: &Path, config: &HostMonitorConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("无法创建配置目录：{error}"))?;
    }
    let body = serde_json::to_string_pretty(config)
        .map_err(|error| format!("无法序列化小主机监控配置：{error}"))?;
    fs::write(path, body).map_err(|error| format!("无法写入小主机监控配置：{error}"))
}

fn credential() -> Result<Entry, String> {
    Entry::new("com.focusd.island.host-monitor", "token").map_err(|error| error.to_string())
}

pub fn read_token() -> Result<Option<String>, String> {
    match credential()?.get_password() {
        Ok(token) if !token.trim().is_empty() => Ok(Some(token)),
        Ok(_) => Ok(None),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(error) => Err(format!("无法读取 Windows 凭据：{error}")),
    }
}

pub fn save_token(token: &str) -> Result<(), String> {
    let token = token.trim();
    if token.chars().count() > 200 {
        return Err("访问令牌过长。".to_string());
    }
    credential()?
        .set_password(token)
        .map_err(|error| format!("无法保存到 Windows 凭据：{error}"))
}

pub fn delete_token() -> Result<(), String> {
    match credential()?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(format!("无法删除 Windows 凭据：{error}")),
    }
}
