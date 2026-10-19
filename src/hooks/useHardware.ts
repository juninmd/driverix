import { useCallback, useState } from "react";
import { api, type HardwareView } from "../lib/api";

export type HardwareState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "done"; view: HardwareView }
  | { kind: "error"; message: string };

export function useHardware() {
  const [state, setState] = useState<HardwareState>({ kind: "idle" });

  const load = useCallback(async () => {
    setState({ kind: "loading" });
    try {
      setState({ kind: "done", view: await api.loadHardware() });
    } catch (e) {
      setState({ kind: "error", message: String(e) });
    }
  }, []);

  return { state, load };
}
