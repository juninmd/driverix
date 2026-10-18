import { useCallback, useEffect, useState } from "react";
import { api, type ScanPhase, type ScanResult } from "../lib/api";

export type ScanState =
  | { kind: "idle" }
  | { kind: "running"; phase: ScanPhase }
  | { kind: "done"; result: ScanResult; at: number }
  | { kind: "error"; message: string };

export function useScan() {
  const [state, setState] = useState<ScanState>({ kind: "idle" });

  useEffect(() => {
    const un = api.onScanPhase((phase) => setState((s) => (s.kind === "running" ? { kind: "running", phase } : s)));
    return () => {
      un.then((f) => f());
    };
  }, []);

  const start = useCallback(async () => {
    setState({ kind: "running", phase: "inventory" });
    try {
      const result = await api.scanUpdates();
      setState({ kind: "done", result, at: Date.now() });
    } catch (e) {
      setState({ kind: "error", message: String(e) });
    }
  }, []);

  const cancel = useCallback(() => {
    void api.cancel();
  }, []);

  return { state, start, cancel };
}
