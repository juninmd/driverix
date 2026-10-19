import { useCallback, useEffect, useMemo, useState } from "react";
import { Backups } from "./components/Backups";
import { Devices } from "./components/Devices";
import { Hardware } from "./components/Hardware";
import { InstallFlow } from "./components/InstallFlow";
import { Updates } from "./components/Updates";
import { useHardware } from "./hooks/useHardware";
import { useScan } from "./hooks/useScan";
import { api, type Offer, type Status } from "./lib/api";

type Tab = "updates" | "devices" | "hardware" | "backups";

const PHASE_TEXT = {
  inventory: "Lendo o hardware…",
  "windows-update": "Consultando o Windows Update (pode levar alguns minutos)…",
} as const;

export default function App() {
  const { state, start, cancel } = useScan();
  const hardware = useHardware();
  const [status, setStatus] = useState<Status>({ elevated: false, backupsDir: "" });
  const [tab, setTab] = useState<Tab>("updates");
  const [installing, setInstalling] = useState<Offer[] | null>(null);
  const [backupsKey, setBackupsKey] = useState(0);
  const [elevateError, setElevateError] = useState("");

  useEffect(() => {
    api.status().then(setStatus);
  }, []);

  const result = state.kind === "done" ? state.result : null;
  const running = state.kind === "running";
  const devices = result?.devices ?? [];
  const byId = useMemo(() => new Map(devices.map((d) => [d.instanceId, d])), [devices]);
  const installedVersion = useCallback(
    (o: Offer) => o.deviceIds.map((id) => byId.get(id)?.version).find(Boolean) ?? "",
    [byId],
  );
  const newer = result?.offers.filter((o) => o.decision === "newer").length ?? 0;
  const broken = devices.filter((d) => d.problemCode !== 0).length;

  const elevate = async () => {
    setElevateError("");
    try {
      await api.relaunchElevated();
    } catch (e) {
      setElevateError(String(e));
    }
  };

  return (
    <div className="app">
      <header>
        <div>
          <h1>Driverix</h1>
          <p className="sub">Drivers do Windows Update, com backup e rollback</p>
        </div>
        {running ? (
          <button type="button" onClick={cancel}>
            Cancelar
          </button>
        ) : (
          <button type="button" className="primary" onClick={start}>
            {result ? "Buscar de novo" : "Buscar atualizações"}
          </button>
        )}
      </header>

      {!status.elevated && (
        <div className="banner" role="status">
          <span>Você pode buscar sem privilégios, mas instalar e restaurar exigem administrador.</span>
          <button type="button" onClick={elevate}>
            Reabrir como administrador
          </button>
          {elevateError && <span className="error">{elevateError}</span>}
        </div>
      )}

      {running && (
        <p className="progress" role="status">
          {PHASE_TEXT[state.phase]}
        </p>
      )}
      {state.kind === "error" && (
        <p className="error" role="alert">
          {state.message}
        </p>
      )}
      {result && (
        <p className="summary">
          {devices.length} dispositivos · {newer} atualização(ões) disponível(is) · {broken} com problema
        </p>
      )}

      <div role="tablist" aria-label="Seções" className="tabs">
        {(
          [
            ["updates", "Atualizações"],
            ["devices", "Dispositivos"],
            ["hardware", "Hardware"],
            ["backups", "Backups"],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={tab === id}
            className={tab === id ? "active" : ""}
            onClick={() => setTab(id)}
          >
            {label}
          </button>
        ))}
      </div>

      <main role="tabpanel">
        {tab === "updates" &&
          (result ? (
            <Updates
              offers={result.offers}
              installedVersion={installedVersion}
              busy={running || installing !== null}
              elevated={status.elevated}
              onInstall={setInstalling}
            />
          ) : (
            <p className="empty">Clique em “Buscar atualizações” para começar.</p>
          ))}
        {tab === "devices" &&
          (result ? <Devices devices={devices} /> : <p className="empty">Faça uma busca para listar o hardware.</p>)}
        {tab === "hardware" && <Hardware state={hardware.state} onLoad={hardware.load} />}
        {tab === "backups" && <Backups elevated={status.elevated} refreshKey={backupsKey} />}
      </main>

      {installing && (
        <InstallFlow
          offers={installing}
          onClose={() => setInstalling(null)}
          onDone={() => {
            setBackupsKey((k) => k + 1);
            void start();
          }}
        />
      )}
    </div>
  );
}
