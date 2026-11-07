import { useCallback, useEffect, useRef, useState } from "react";
import {
  getHostMonitorState,
  listenHostMonitorChanged,
  refreshHostMonitor,
} from "./api";
import type { HostMonitorView } from "./types";

/** 事件优先（Rust 侧轮询后推送），5 秒兜底轮询只读取内存快照，避免重复请求小主机。 */
const FALLBACK_POLL_MS = 5_000;

export function useHostMonitor() {
  const [view, setView] = useState<HostMonitorView | null>(null);
  const isReadingRef = useRef(false);
  const isMountedRef = useRef(true);

  const applyView = useCallback((next: HostMonitorView) => {
    setView((current) => (current && isSameView(current, next) ? current : next));
  }, []);

  const reload = useCallback(async () => {
    if (isReadingRef.current) {
      return;
    }

    isReadingRef.current = true;
    try {
      const next = await getHostMonitorState();
      if (isMountedRef.current) {
        applyView(next);
      }
    } catch (error) {
      console.error("Failed to read host monitor state", error);
    } finally {
      isReadingRef.current = false;
    }
  }, [applyView]);

  const refresh = useCallback(async () => {
    try {
      const next = await refreshHostMonitor();
      if (isMountedRef.current) {
        applyView(next);
      }
    } catch (error) {
      console.error("Failed to refresh host monitor", error);
    }
  }, [applyView]);

  useEffect(() => {
    isMountedRef.current = true;
    void reload();

    let isDisposed = false;
    let unlisten: (() => void) | null = null;

    void listenHostMonitorChanged((next) => {
      if (!isDisposed) {
        applyView(next);
      }
    })
      .then((stop) => {
        if (isDisposed) {
          stop();
          return;
        }
        unlisten = stop;
      })
      .catch((error) => {
        console.error("Failed to subscribe to host monitor", error);
      });

    const interval = window.setInterval(() => {
      void reload();
    }, FALLBACK_POLL_MS);

    return () => {
      isDisposed = true;
      isMountedRef.current = false;
      window.clearInterval(interval);
      unlisten?.();
    };
  }, [applyView, reload]);

  return { view, reload, refresh };
}

function isSameView(current: HostMonitorView, next: HostMonitorView) {
  return (
    current.configured === next.configured &&
    current.enabled === next.enabled &&
    current.baseUrl === next.baseUrl &&
    current.intervalMs === next.intervalMs &&
    current.showInCollapsed === next.showInCollapsed &&
    current.hasToken === next.hasToken &&
    current.fetchedAt === next.fetchedAt &&
    current.connected === next.connected &&
    current.stale === next.stale &&
    current.error === next.error
  );
}
