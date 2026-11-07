//! Snapshot types for the remote host monitor plus the parser for the
//! status-app `/api/state` payload. Every field is tolerant so a status-app
//! upgrade cannot break the island.

use serde::Serialize;
use serde_json::Value;

/// History points forwarded to the island; the panel only has room for a short
/// sparkline, so the event payload stays small.
const HISTORY_POINTS: usize = 40;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostMonitorView {
    pub configured: bool,
    pub enabled: bool,
    pub base_url: String,
    pub interval_ms: u64,
    pub show_in_collapsed: bool,
    pub has_token: bool,
    pub fetched_at: u64,
    pub connected: bool,
    pub stale: bool,
    pub error: Option<String>,
    pub payload: Option<HostPayload>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostPayload {
    pub overall: Option<String>,
    pub counts: Option<Counts>,
    pub host: Option<HostInfo>,
    pub sys: Option<SysInfo>,
    pub services: Vec<ServiceInfo>,
    pub procs: Option<Procs>,
    pub history: Option<History>,
    pub app: Option<AppInfo>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub ok: u32,
    pub degraded: u32,
    pub down: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInfo {
    pub name: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub cores: Option<u32>,
    pub uptime: Option<f64>,
    pub load: Option<f64>,
    pub temp: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SysInfo {
    pub cpu: Option<f64>,
    pub mem_pct: Option<f64>,
    pub mem_used: Option<f64>,
    pub mem_total: Option<f64>,
    pub swap_used: Option<f64>,
    pub swap_total: Option<f64>,
    pub disk_pct: Option<f64>,
    pub disk_used: Option<f64>,
    pub disk_total: Option<f64>,
    pub load: Option<f64>,
    pub temp: Option<f64>,
    pub uptime: Option<f64>,
    pub rx_rate: Option<f64>,
    pub tx_rate: Option<f64>,
    pub cores: Option<u32>,
    pub hostname: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub id: String,
    pub name: String,
    pub desc: Option<String>,
    pub group: Option<String>,
    pub optional: bool,
    pub status: String,
    pub pid: Option<u64>,
    pub cpu: Option<f64>,
    pub rss: Option<f64>,
    pub age: Option<f64>,
    pub count: Option<u32>,
    pub ports: Vec<PortInfo>,
    pub http: Option<HttpProbe>,
    pub note: Option<String>,
    pub has_log: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortInfo {
    pub port: u32,
    pub open: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpProbe {
    pub status: Option<u32>,
    pub ms: Option<f64>,
    pub ok: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Procs {
    pub total: Option<u32>,
    pub rss: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub cpu: Vec<f64>,
    pub mem: Vec<f64>,
    pub net_rx: Vec<f64>,
    pub net_tx: Vec<f64>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub uptime: Option<f64>,
    pub version: Option<String>,
}

/// Result of a one-off connection check from the settings window.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostMonitorTestResult {
    pub ok: bool,
    pub message: String,
    pub latency_ms: u64,
    pub overall: Option<String>,
    pub hostname: Option<String>,
    pub service_count: u32,
}

pub fn parse_payload(value: &Value) -> HostPayload {
    let services: Vec<ServiceInfo> = value
        .get("services")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(parse_service).collect())
        .unwrap_or_default();
    HostPayload {
        overall: text(value, "overall"),
        counts: reconcile_counts(value.get("counts").map(parse_counts), &services),
        host: value.get("host").map(parse_host),
        sys: value.get("sys").map(parse_sys),
        services,
        procs: value.get("procs").map(parse_procs),
        history: value.get("history").map(parse_history),
        app: value.get("app").map(parse_app),
    }
}

/// The status-app counts sometimes skip optional services that are down, so the
/// per-service statuses win whenever the totals disagree.
fn reconcile_counts(parsed: Option<Counts>, services: &[ServiceInfo]) -> Option<Counts> {
    if services.is_empty() {
        return parsed;
    }
    let mut tally = Counts {
        ok: 0,
        degraded: 0,
        down: 0,
        total: services.len() as u32,
    };
    for service in services {
        match service.status.as_str() {
            "ok" => tally.ok += 1,
            "degraded" => tally.degraded += 1,
            "down" => tally.down += 1,
            _ => {}
        }
    }
    match parsed {
        Some(counts)
            if counts.total == tally.total
                && counts.ok + counts.degraded + counts.down == tally.total =>
        {
            Some(counts)
        }
        _ => Some(tally),
    }
}

fn parse_counts(value: &Value) -> Counts {
    let ok = count(value, "ok");
    let degraded = count(value, "degraded");
    let down = count(value, "down");
    let total = count(value, "total");
    Counts {
        ok,
        degraded,
        down,
        total: if total > 0 { total } else { ok + degraded + down },
    }
}

fn parse_host(value: &Value) -> HostInfo {
    HostInfo {
        name: text(value, "name"),
        title: text(value, "title"),
        subtitle: text(value, "subtitle"),
        cores: optional_count(value, "cores"),
        uptime: number(value, "uptime"),
        load: load(value),
        temp: number(value, "temp"),
    }
}

fn parse_sys(value: &Value) -> SysInfo {
    SysInfo {
        cpu: number(value, "cpu"),
        mem_pct: number(value, "memPct"),
        mem_used: number(value, "memUsed"),
        mem_total: number(value, "memTotal"),
        swap_used: number(value, "swapUsed"),
        swap_total: number(value, "swapTotal"),
        disk_pct: number(value, "diskPct"),
        disk_used: number(value, "diskUsed"),
        disk_total: number(value, "diskTotal"),
        load: load(value),
        temp: number(value, "temp"),
        uptime: number(value, "uptime"),
        rx_rate: number(value, "rxRate"),
        tx_rate: number(value, "txRate"),
        cores: optional_count(value, "cores"),
        hostname: text(value, "hostname"),
    }
}

fn parse_service(value: &Value) -> ServiceInfo {
    ServiceInfo {
        id: text(value, "id").unwrap_or_default(),
        name: text(value, "name").unwrap_or_default(),
        desc: text(value, "desc"),
        group: text(value, "group"),
        optional: value
            .get("optional")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        status: text(value, "status").unwrap_or_else(|| "unknown".to_string()),
        pid: value.get("pid").and_then(Value::as_u64),
        cpu: number(value, "cpu"),
        rss: number(value, "rss"),
        age: number(value, "age"),
        count: optional_count(value, "count"),
        ports: value
            .get("ports")
            .and_then(Value::as_array)
            .map(|ports| ports.iter().map(parse_port).collect())
            .unwrap_or_default(),
        http: value.get("http").filter(|probe| !probe.is_null()).map(parse_http),
        note: text(value, "note"),
        has_log: value
            .get("hasLog")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

fn parse_port(value: &Value) -> PortInfo {
    PortInfo {
        port: count(value, "port"),
        open: value.get("open").and_then(Value::as_bool).unwrap_or(false),
    }
}

fn parse_http(value: &Value) -> HttpProbe {
    HttpProbe {
        status: optional_count(value, "status"),
        ms: number(value, "ms"),
        ok: value.get("ok").and_then(Value::as_bool),
    }
}

fn parse_procs(value: &Value) -> Procs {
    Procs {
        total: optional_count(value, "total"),
        rss: number(value, "rss"),
    }
}

fn parse_history(value: &Value) -> History {
    History {
        cpu: series(value, "cpu"),
        mem: series(value, "mem"),
        net_rx: series(value, "netRx"),
        net_tx: series(value, "netTx"),
    }
}

fn parse_app(value: &Value) -> AppInfo {
    AppInfo {
        uptime: number(value, "uptime"),
        version: text(value, "version"),
    }
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn count(value: &Value, key: &str) -> u32 {
    value
        .get(key)
        .and_then(Value::as_u64)
        .map(|raw| raw.min(u32::MAX as u64) as u32)
        .unwrap_or_default()
}

fn optional_count(value: &Value, key: &str) -> Option<u32> {
    value.get(key).and_then(Value::as_u64).map(|raw| raw.min(u32::MAX as u64) as u32)
}

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|text| !text.trim().is_empty())
}

/// `load` is a scalar on some status-app builds and a 1/5/15 array on others.
fn load(value: &Value) -> Option<f64> {
    match value.get("load") {
        Some(Value::Array(items)) => items.first().and_then(Value::as_f64),
        other => other.and_then(Value::as_f64),
    }
}

fn series(value: &Value, key: &str) -> Vec<f64> {
    let items = match value.get(key).and_then(Value::as_array) {
        Some(items) => items,
        None => return Vec::new(),
    };
    let start = items.len().saturating_sub(HISTORY_POINTS);
    items[start..]
        .iter()
        .filter_map(Value::as_f64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Value {
        json!({
            "overall": "degraded",
            "counts": { "ok": 8, "degraded": 1, "down": 2, "total": 11 },
            "host": { "name": "dsh", "title": "DSH", "cores": 4, "uptime": 12.5, "load": [0.4, 0.5, 0.6], "temp": 33 },
            "sys": {
                "cpu": 13.7, "memPct": 39.6, "memUsed": 3230310400.0, "memTotal": 8151273472.0,
                "diskPct": 10.2, "load": 0.61, "rxRate": 1484, "txRate": 378, "hostname": "dsh"
            },
            "services": [
                {
                    "id": "dsh-web", "name": "DSH Web", "status": "ok", "cpu": 54,
                    "rss": 1432797184, "ports": [{ "port": 3080, "open": true }],
                    "http": { "status": 401, "ms": 2, "ok": true }, "hasLog": true
                },
                { "id": "snowluma", "name": "SnowLuma", "status": "down", "http": null }
            ],
            "procs": { "total": 192, "rss": 1886404608 },
            "history": { "cpu": [1, 2, 3], "mem": [4], "netRx": [], "netTx": [5, 6] },
            "app": { "uptime": 99, "version": "1.2.3" }
        })
    }

    #[test]
    fn parses_status_app_snapshot() {
        let payload = parse_payload(&sample());
        assert_eq!(payload.overall.as_deref(), Some("degraded"));
        // The fixture counts claim 11 services while only two are listed, so the
        // per-service statuses are used instead.
        let counts = payload.counts.expect("counts");
        assert_eq!(counts.total, 2);
        assert_eq!(counts.ok, 1);
        assert_eq!(counts.down, 1);
        assert_eq!(counts.degraded, 0);
        assert_eq!(payload.services.len(), 2);
        assert_eq!(payload.services[0].id, "dsh-web");
        assert_eq!(
            payload.services[0].http.as_ref().and_then(|probe| probe.status),
            Some(401)
        );
        assert!(payload.services[1].http.is_none());
        assert_eq!(
            payload
                .sys
                .as_ref()
                .and_then(|sys| sys.hostname.clone())
                .as_deref(),
            Some("dsh")
        );
        assert_eq!(
            payload.history.as_ref().map(|history| history.cpu.len()),
            Some(3)
        );
    }

    #[test]
    fn keeps_consistent_counts_from_the_host() {
        let value = json!({
            "counts": { "ok": 1, "degraded": 1, "down": 0, "total": 2 },
            "services": [
                { "id": "a", "name": "A", "status": "ok" },
                { "id": "b", "name": "B", "status": "degraded" }
            ]
        });
        let counts = parse_payload(&value).counts.expect("counts");
        assert_eq!(counts.total, 2);
        assert_eq!(counts.ok, 1);
        assert_eq!(counts.degraded, 1);
    }

    #[test]
    fn tolerates_missing_and_odd_shapes() {
        for value in [
            json!({}),
            json!({ "sys": "nope" }),
            json!({ "services": 3, "history": { "cpu": "nope" } })
        ] {
            let payload = parse_payload(&value);
            assert!(payload.services.is_empty());
            assert_eq!(payload.overall, None);
        }
    }

    #[test]
    fn caps_history_points() {
        let cpu = Value::Array((0..90).map(|index| json!(index as f64)).collect());
        let payload = parse_payload(&json!({ "history": { "cpu": cpu } }));
        let series = payload.history.expect("history").cpu;
        assert_eq!(series.len(), HISTORY_POINTS);
        assert_eq!(series.first().copied(), Some((90 - HISTORY_POINTS) as f64));
    }
}

