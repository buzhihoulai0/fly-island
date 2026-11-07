import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  HostMonitorConfig,
  HostMonitorTestResult,
  HostMonitorView,
} from "./types";

export const HOST_MONITOR_EVENT = "host-monitor-changed";

export function getHostMonitorState() {
  return invoke<HostMonitorView>("get_host_monitor_state");
}

export function setHostMonitorConfig(config: HostMonitorConfig) {
  return invoke<HostMonitorView>("set_host_monitor_config", { config });
}

/** 立即拉取一次快照；结果同时通过返回值与 host-monitor-changed 事件反映。 */
export function refreshHostMonitor() {
  return invoke<HostMonitorView>("refresh_host_monitor");
}

export function testHostMonitorConnection(baseUrl: string, token: string | null) {
  return invoke<HostMonitorTestResult>("test_host_monitor_connection", {
    baseUrl,
    token,
  });
}

export function listenHostMonitorChanged(
  handler: (view: HostMonitorView) => void,
): Promise<UnlistenFn> {
  return listen<HostMonitorView>(HOST_MONITOR_EVENT, (event) => {
    handler(event.payload);
  });
}
