import { useMemo, useState } from "react";
import type { Device } from "../lib/api";
import { formatDate, isOldDriver, problemText } from "../lib/format";

export function Devices({ devices }: { devices: Device[] }) {
  const [query, setQuery] = useState("");
  const [onlyProblems, setOnlyProblems] = useState(false);
  const [onlyOld, setOnlyOld] = useState(false);
  const [cls, setCls] = useState("");

  const classes = useMemo(() => [...new Set(devices.map((d) => d.class).filter(Boolean))].sort(), [devices]);
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return devices.filter(
      (d) =>
        (!onlyProblems || d.problemCode !== 0) &&
        (!onlyOld || isOldDriver(d.date, d.provider)) &&
        (!cls || d.class === cls) &&
        (!q ||
          `${d.name} ${d.manufacturer} ${d.provider} ${d.vendor} ${d.hint} ${d.instanceId}`.toLowerCase().includes(q)),
    );
  }, [devices, query, onlyProblems, onlyOld, cls]);

  return (
    <>
      <div className="filters">
        <input
          type="search"
          placeholder="Buscar dispositivo, fabricante ou ID"
          aria-label="Buscar dispositivo"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <select aria-label="Classe" value={cls} onChange={(e) => setCls(e.target.value)}>
          <option value="">Todas as classes</option>
          {classes.map((c) => (
            <option key={c}>{c}</option>
          ))}
        </select>
        <label>
          <input type="checkbox" checked={onlyProblems} onChange={(e) => setOnlyProblems(e.target.checked)} /> Só com
          problema
        </label>
        <label>
          <input type="checkbox" checked={onlyOld} onChange={(e) => setOnlyOld(e.target.checked)} /> Só drivers com 3+
          anos
        </label>
        <span className="count">
          {shown.length} de {devices.length}
        </span>
      </div>
      <table className="data-table">
        <thead>
          <tr>
            <th>Dispositivo</th>
            <th>Classe</th>
            <th>Fornecedor</th>
            <th>Versão</th>
            <th>Data</th>
            <th>Estado</th>
          </tr>
        </thead>
        <tbody>
          {shown.map((d) => (
            <tr key={d.instanceId}>
              <td>
                <div className="title">{d.name || d.instanceId}</div>
                <div className="sub mono">{d.hint || (d.hardwareIds[0] ?? d.instanceId)}</div>
              </td>
              <td>{d.class || "—"}</td>
              <td>{d.provider || d.manufacturer || d.vendor || "—"}</td>
              <td className="mono">{d.version || "—"}</td>
              <td>
                {formatDate(d.date)}
                {isOldDriver(d.date, d.provider) && (
                  <span className="chip warn" title="Driver de terceiros com 3 anos ou mais">
                    antigo
                  </span>
                )}
              </td>
              <td className={d.problemCode === 0 ? "ok" : "bad"}>{problemText(d.problemCode)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
}
