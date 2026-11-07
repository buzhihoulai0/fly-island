// 与 src-tauri/src/host_monitor/view.rs 的 HostMonitorView 及 status-app 的 /api/state 对应。
// Rust 侧 Option 字段会序列化为 null，所以可空字段全部显式标注。

export type HostMonitorCounts = {
  ok: number;
  degraded: number;
  down: number;
  total: number;
};

export type HostMonitorHostInfo = {
  name: string | null;
  title: string | null;
  subtitle: string | null;
  cores: number | null;
  uptime: number | null;
  load: number | null;
  temp: number | null;
};

export type HostMonitorSysInfo = {
  cpu: number | null;
  memPct: number | null;
  memUsed: number | null;
  memTotal: number | null;
  swapUsed: number | null;
  swapTotal: number | null;
  diskPct: number | null;
  diskUsed: number | null;
  diskTotal: number | null;
  load: number | null;
  temp: number | null;
  uptime: number | null;
  rxRate: number | null;
  txRate: number | null;
  cores: number | null;
  hostname: string | null;
};

export type HostMonitorPort = {
  port: number;
  open: boolean;
};

export type HostMonitorHttpProbe = {
  status: number | null;
  ms: number | null;
  ok: boolean | null;
};

export type HostMonitorService = {
  id: string;
  name: string;
  desc: string | null;
  group: string | null;
  optional: boolean;
  status: string;
  pid: number | null;
  cpu: number | null;
  rss: number | null;
  age: number | null;
  count: number | null;
  ports: HostMonitorPort[];
  http: HostMonitorHttpProbe | null;
  note: string | null;
  hasLog: boolean;
};

export type HostMonitorHistory = {
  cpu: number[];
  mem: number[];
  netRx: number[];
  netTx: number[];
};

export type HostMonitorProcs = {
  total: number | null;
  rss: number | null;
};

export type HostMonitorAppInfo = {
  uptime: number | null;
  version: string | null;
};

export type HostMonitorPayload = {
  overall: string | null;
  counts: HostMonitorCounts | null;
  host: HostMonitorHostInfo | null;
  sys: HostMonitorSysInfo | null;
  services: HostMonitorService[];
  procs: HostMonitorProcs | null;
  history: HostMonitorHistory | null;
  app: HostMonitorAppInfo | null;
};

export type HostMonitorView = {
  configured: boolean;
  enabled: boolean;
  baseUrl: string;
  intervalMs: number;
  showInCollapsed: boolean;
  hasToken: boolean;
  fetchedAt: number;
  connected: boolean;
  stale: boolean;
  error: string | null;
  payload: HostMonitorPayload | null;
};

export type HostMonitorTestResult = {
  ok: boolean;
  message: string;
  latencyMs: number;
  overall: string | null;
  hostname: string | null;
  serviceCount: number;
};

/** 设置窗口提交的配置；token 省略表示保持原值，空串表示删除。 */
export type HostMonitorConfig = {
  baseUrl: string;
  intervalMs: number;
  enabled: boolean;
  showInCollapsed: boolean;
  token?: string;
};

export type HostPulseState = "ok" | "degraded" | "down" | "offline";
