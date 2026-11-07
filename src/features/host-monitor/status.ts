import type { HostMonitorView, HostPulseState } from "./types";

/** 折叠条与面板共用的整体状态判断，避免各自再解析 overall/counts。 */
export function getHostPulseState(view: HostMonitorView | null): HostPulseState {
  if (!view || !view.configured || !view.enabled || !view.connected) {
    return "offline";
  }

  if (view.stale) {
    return "degraded";
  }

  const overall = view.payload?.overall;
  const services = view.payload?.services ?? [];
  const critical = services.filter((service) => !service.optional);

  // The host reports "ok" while optional services are down; a core service that
  // is down still has to raise the island badge.
  if (overall === "down" || critical.some((service) => service.status === "down")) {
    return "down";
  }

  if (overall === "degraded" || critical.some((service) => service.status === "degraded")) {
    return "degraded";
  }

  if (overall === "ok") {
    return "ok";
  }

  const counts = view.payload?.counts;

  if (counts && counts.down > 0) {
    return "down";
  }

  if (counts && counts.degraded > 0) {
    return "degraded";
  }

  return "ok";
}
