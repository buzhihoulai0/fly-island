//! Host monitor: polls a status-app instance on the LAN and mirrors the
//! snapshot into the island. Requests run on the Tauri async runtime so the
//! access token never reaches the webview.

mod config;
mod fetch;
mod view;

use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager};

use config::{
    config_path, delete_token, load_config, read_token, save_config, save_token,
    HostMonitorConfig, HostMonitorConfigInput, MIN_INTERVAL_MS,
};
use fetch::{build_client, fetch_state, Fetched};
use view::{parse_payload, HostMonitorTestResult, HostMonitorView};

const EVENT: &str = "host-monitor-changed";
const IDLE_INTERVAL_MS: u64 = 2_000;
const MAX_BACKOFF_STEPS: u32 = 3;
const MAX_DELAY_MS: u64 = 30_000;
const MAX_TOKEN_CHARACTERS: usize = 200;

static SERVICE: OnceLock<Service> = OnceLock::new();

struct Service {
    app: AppHandle,
    config_path: PathBuf,
    state: Mutex<State>,
}

struct State {
    config: HostMonitorConfig,
    has_token: bool,
    payload: Option<view::HostPayload>,
    fetched_at: u64,
    last_ok_at: u64,
    connected: bool,
    error: Option<String>,
    failures: u32,
}

#[tauri::command]
pub fn get_host_monitor_state() -> Result<HostMonitorView, String> {
    Ok(service()?.view())
}

#[tauri::command]
pub fn set_host_monitor_config(config: HostMonitorConfigInput) -> Result<HostMonitorView, String> {
    service()?.apply_config(config)
}

#[tauri::command]
pub async fn refresh_host_monitor() -> Result<HostMonitorView, String> {
    Ok(run_fetch().await)
}

#[tauri::command]
pub async fn test_host_monitor_connection(
    base_url: String,
    token: Option<String>,
) -> Result<HostMonitorTestResult, String> {
    let effective_token = match token {
        Some(token) if !token.trim().is_empty() => {
            let token = token.trim();
            if token.chars().count() > MAX_TOKEN_CHARACTERS {
                return Err("访问令牌过长。".to_string());
            }
            Some(token.to_string())
        }
        Some(_) => None,
        None => read_token()?,
    };
    let candidate = HostMonitorConfig {
        base_url,
        ..HostMonitorConfig::default()
    }
    .normalized();
    if candidate.base_url.is_empty() {
        return Err("请先填写小主机地址。".to_string());
    }
    let client = build_client()?;
    let started = Instant::now();
    match fetch_state(&client, &candidate.base_url, effective_token.as_deref()).await {
        Ok(fetched) => {
            let payload = parse_payload(&fetched.json);
            let hostname = payload
                .sys
                .as_ref()
                .and_then(|sys| sys.hostname.clone())
                .or_else(|| payload.host.as_ref().and_then(|host| host.name.clone()));
            Ok(HostMonitorTestResult {
                ok: true,
                message: "连接成功".to_string(),
                latency_ms: if fetched.latency_ms > 0 {
                    fetched.latency_ms
                } else {
                    started.elapsed().as_millis() as u64
                },
                overall: payload.overall.clone(),
                hostname,
                service_count: payload.services.len() as u32,
            })
        }
        Err(error) => Ok(HostMonitorTestResult {
            ok: false,
            message: error,
            ..HostMonitorTestResult::default()
        }),
    }
}

pub fn init(app: &AppHandle) -> Result<(), String> {
    if SERVICE.get().is_some() {
        return Ok(());
    }
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("无法定位配置目录：{error}"))?;
    std::fs::create_dir_all(&directory).map_err(|error| format!("无法创建配置目录：{error}"))?;
    let path = config_path(&directory);
    let config = load_config(&path);
    let has_token = read_token().unwrap_or(None).is_some();
    let service = Service {
        app: app.clone(),
        config_path: path,
        state: Mutex::new(State {
            config,
            has_token,
            payload: None,
            fetched_at: 0,
            last_ok_at: 0,
            connected: false,
            error: None,
            failures: 0,
        }),
    };
    let _ = SERVICE.set(service);
    tauri::async_runtime::spawn(poll_loop());
    Ok(())
}

fn service() -> Result<&'static Service, String> {
    SERVICE
        .get()
        .ok_or_else(|| "小主机监控服务尚未初始化。".to_string())
}

async fn poll_loop() {
    loop {
        match SERVICE.get() {
            Some(service) => {
                if service.pending_target().is_some() {
                    let _ = run_fetch().await;
                }
                sleep(service.next_delay()).await;
            }
            None => sleep(Duration::from_millis(IDLE_INTERVAL_MS)).await,
        }
    }
}

async fn run_fetch() -> HostMonitorView {
    let service = match SERVICE.get() {
        Some(service) => service,
        None => return HostMonitorView::default(),
    };
    let target = match service.pending_target() {
        Some(target) => target,
        None => return service.view(),
    };
    let result = match build_client() {
        Ok(client) => fetch_state(&client, &target.0, target.1.as_deref()).await,
        Err(error) => Err(error),
    };
    service.apply_result(result)
}

async fn sleep(duration: Duration) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(duration)).await;
}

impl Service {
    fn view(&self) -> HostMonitorView {
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let config = &state.config;
        let now = now_millis();
        let stale = state.connected
            && state.last_ok_at > 0
            && now.saturating_sub(state.last_ok_at) > config.interval_ms.saturating_mul(3);
        HostMonitorView {
            configured: !config.base_url.is_empty(),
            enabled: config.enabled,
            base_url: config.base_url.clone(),
            interval_ms: config.interval_ms,
            show_in_collapsed: config.show_in_collapsed,
            has_token: state.has_token,
            fetched_at: state.fetched_at,
            connected: state.connected,
            stale,
            error: state.error.clone(),
            payload: state.payload.clone(),
        }
    }

    fn emit_view(&self) -> HostMonitorView {
        let view = self.view();
        let _ = self.app.emit(EVENT, view.clone());
        view
    }

    fn pending_target(&self) -> Option<(String, Option<String>)> {
        let base_url = {
            let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            if !state.config.enabled || state.config.base_url.is_empty() {
                return None;
            }
            state.config.base_url.clone()
        };
        Some((base_url, read_token().unwrap_or(None)))
    }

    fn next_delay(&self) -> Duration {
        let (interval, failures) = {
            let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            if !state.config.enabled || state.config.base_url.is_empty() {
                return Duration::from_millis(IDLE_INTERVAL_MS);
            }
            (state.config.interval_ms, state.failures)
        };
        let factor = 1u64 << failures.min(MAX_BACKOFF_STEPS);
        let delay = interval.saturating_mul(factor).min(MAX_DELAY_MS);
        Duration::from_millis(delay.max(MIN_INTERVAL_MS))
    }

    fn apply_result(&self, result: Result<Fetched, String>) -> HostMonitorView {
        {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            match result {
                Ok(fetched) => {
                    state.payload = Some(parse_payload(&fetched.json));
                    state.fetched_at = now_millis();
                    state.last_ok_at = state.fetched_at;
                    state.connected = true;
                    state.error = None;
                    state.failures = 0;
                }
                Err(error) => {
                    state.connected = false;
                    state.error = Some(error);
                    state.failures = state.failures.saturating_add(1);
                }
            }
        }
        self.emit_view()
    }

    fn apply_config(&self, input: HostMonitorConfigInput) -> Result<HostMonitorView, String> {
        let previous_has_token = self
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .has_token;
        let mut has_token = previous_has_token;
        if let Some(token) = input.token.as_deref() {
            let token = token.trim();
            if token.is_empty() {
                delete_token()?;
                has_token = false;
            } else {
                if token.chars().count() > MAX_TOKEN_CHARACTERS {
                    return Err("访问令牌过长。".to_string());
                }
                save_token(token)?;
                has_token = true;
            }
        }

        let mut next = HostMonitorConfig {
            base_url: input.base_url,
            interval_ms: input.interval_ms,
            enabled: input.enabled,
            show_in_collapsed: input.show_in_collapsed,
        };
        if next.interval_ms == 0 {
            next.interval_ms = self
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .config
                .interval_ms;
        }
        let next = next.normalized();

        {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            if state.config.base_url != next.base_url {
                state.payload = None;
                state.connected = false;
                state.error = None;
                state.failures = 0;
                state.last_ok_at = 0;
            }
            state.config = next.clone();
            state.has_token = has_token;
        }

        save_config(&self.config_path, &next)?;
        Ok(self.emit_view())
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
