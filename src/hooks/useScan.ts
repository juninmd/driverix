import { useCallback, useEffect, useRef, useState } from "react";
import { api, type ScanPhase, type ScanResult } from "../lib/api";

export type ScanState =
  | { kind: "idle" }
  | { kind: "running"; phase: ScanPhase }
  | { kind: "done"; result: ScanResult; at: number }
  | { kind: "cancelled" }
  | { kind: "error"; message: string };

export interface LastScan {
  result: ScanResult;
  at: number;
}

export function useScan() {
  const [state, setState] = useState<ScanState>({ kind: "idle" });
  // Kept apart from `state` so a re-scan does not blank the list the user is looking at.
  const [last, setLast] = useState<LastScan | null>(null);
  // The backend reports a cancelled scan as an error; the user asked for it, so it is not one.
  const cancelRequested = useRef(false);

  useEffect(() => {
    const un = api.onScanPhase((phase) => setState((s) => (s.kind === "running" ? { kind: "running", phase } : s)));
    return () => {
      un.then((f) => f());
    };
  }, []);

  const start = useCallback(async () => {
    cancelRequested.current = false;
    setState({ kind: "running", phase: "inventory" });
    try {
      const result = await api.scanUpdates();
      const at = Date.now();
      setLast({ result, at });
      setState({ kind: "done", result, at });
    } catch (e) {
      setState(cancelRequested.current ? { kind: "cancelled" } : { kind: "error", message: String(e) });
    }
  }, []);

  const cancel = useCallback(() => {
    cancelRequested.current = true;
    void api.cancel();
  }, []);

  return { state, last, start, cancel };
}
