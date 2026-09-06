import { useCallback, useEffect, useRef, useState } from "react";
import { getPerformanceSnapshot } from "../lib/bridge";
import type { PerformanceSnapshot } from "../types";

export interface PerformanceMonitorState {
  snapshot: PerformanceSnapshot | null;
  loading: boolean;
  showLoading: boolean;
  refreshing: boolean;
  error: string | null;
  stale: boolean;
  refresh: () => Promise<void>;
}

export function usePerformanceMonitor(
  intervalMs = 2_000,
): PerformanceMonitorState {
  const [snapshot, setSnapshot] = useState<PerformanceSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [showLoading, setShowLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(false);
  const inFlightRef = useRef<Promise<void> | null>(null);

  const fetchSnapshot = useCallback((manual = false): Promise<void> => {
    if (inFlightRef.current) return inFlightRef.current;
    if (manual) setRefreshing(true);

    const request = getPerformanceSnapshot()
      .then((next) => {
        if (!mountedRef.current) return;
        setSnapshot(next);
        setError(null);
      })
      .catch((reason: unknown) => {
        if (!mountedRef.current) return;
        setError(normalizePerformanceError(reason));
      })
      .finally(() => {
        if (mountedRef.current) {
          setLoading(false);
          if (manual) setRefreshing(false);
        }
        inFlightRef.current = null;
      });
    inFlightRef.current = request;
    return request;
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    let disposed = false;
    let timer: number | null = null;

    function schedule() {
      if (disposed) return;
      timer = window.setTimeout(tick, intervalMs);
    }

    function tick() {
      if (disposed) return;
      if (document.visibilityState !== "visible") {
        schedule();
        return;
      }
      void fetchSnapshot().finally(schedule);
    }

    function handleVisibilityChange() {
      if (document.visibilityState !== "visible") return;
      if (timer !== null) window.clearTimeout(timer);
      timer = null;
      tick();
    }

    tick();
    document.addEventListener("visibilitychange", handleVisibilityChange);
    return () => {
      disposed = true;
      mountedRef.current = false;
      if (timer !== null) window.clearTimeout(timer);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, [fetchSnapshot, intervalMs]);

  useEffect(() => {
    if (!loading || snapshot) {
      setShowLoading(false);
      return;
    }
    const timer = window.setTimeout(() => setShowLoading(true), 300);
    return () => window.clearTimeout(timer);
  }, [loading, snapshot]);

  const stale = Boolean(
    snapshot
      && (error
        || Date.now() - snapshot.capturedAtUnixMs > snapshot.refreshAfterMs * 3),
  );

  return {
    snapshot,
    loading,
    showLoading,
    refreshing,
    error,
    stale,
    refresh: () => fetchSnapshot(true),
  };
}

function normalizePerformanceError(reason: unknown): string {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  return "unknown";
}
