//! HTTP access to the status-app API on the monitored host. Every request is
//! made from Rust so the access token never reaches the webview.

use reqwest::{
    header::{ACCEPT, COOKIE},
    Client, StatusCode,
};
use serde_json::Value;
use std::time::{Duration, Instant};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

pub struct Fetched {
    pub json: Value,
    pub latency_ms: u64,
}

pub fn build_client() -> Result<Client, String> {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        // 小主机是局域网设备：必须绕过系统代理（例如 Clash 的 127.0.0.1:7897），
        // 否则请求会被代理接管并一直挂到超时。
        .no_proxy()
        .build()
        .map_err(|error| format!("无法创建监控请求：{error}"))
}

pub async fn fetch_state(
    client: &Client,
    base_url: &str,
    token: Option<&str>,
) -> Result<Fetched, String> {
    let base_url = base_url.trim().trim_end_matches('/');
    if base_url.is_empty() {
        return Err("未配置小主机地址。".to_string());
    }
    let url = format!("{base_url}/api/state");
    let started = Instant::now();
    let mut request = client.get(&url).header(ACCEPT, "application/json");
    if let Some(token) = token.map(str::trim).filter(|token| !token.is_empty()) {
        request = request.header(COOKIE, format!("dsh_sid={token}"));
    }
    let response = request.send().await.map_err(|error| {
        if error.is_timeout() {
            format!("连接小主机超时（{url}），请确认地址和局域网连通性。")
        } else if error.is_connect() {
            format!("无法连接小主机（{url}）。")
        } else {
            format!("请求小主机失败：{error}")
        }
    })?;
    let status = response.status();
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err("认证失败：请在设置里填写正确的访问令牌。".to_string());
    }
    if !status.is_success() {
        return Err(format!("小主机返回 HTTP {}。", status.as_u16()));
    }
    let json = response
        .json::<Value>()
        .await
        .map_err(|error| format!("无法解析小主机数据：{error}"))?;
    if json.get("sys").is_none() && json.get("services").is_none() {
        return Err("小主机返回的数据不是监控快照，请检查端口与路径。".to_string());
    }
    Ok(Fetched {
        json,
        latency_ms: started.elapsed().as_millis() as u64,
    })
}
