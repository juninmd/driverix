import { useCallback, useRef, useState } from "react";
import { api, type HardwareView } from "../lib/api";

export type HardwareState =
  | { kind: "idle" }
  /** `previous` keeps the last view on screen while a refresh runs. */
  | { kind: "loading"; previous?: HardwareView }
  | { kind: "done"; view: HardwareView }
  /** `previous` is the last good view, so a failed refresh does not blank the panel. */
  | { kind: "error"; message: string; previous?: HardwareView };

export function useHardware() {
  const [state, setState] = useState<HardwareState>({ kind: "idle" });

  // One hardware read at a time: a second click must not start another PowerShell run.
  const inFlight = useRef(false);

  const load = useCallback(async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    setState((s) => ({
      kind: "loading",
      previous: s.kind === "done" ? s.view : s.kind === "loading" ? s.previous : undefined,
    }));
    try {
      setState({ kind: "done", view: await api.loadHardware() });
    } catch (e) {
      setState((s) => ({
        kind: "error",
        message: String(e),
        previous: s.kind === "loading" ? s.previous : undefined,
      }));
    } finally {
      inFlight.current = false;
    }
  }, []);

  return { state, load };
}
