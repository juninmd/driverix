import { useMemo, useState } from "react";
import type { Device } from "../lib/api";
import { formatDate, problemText } from "../lib/format";

export function Devices({ devices }: { devices: Device[] }) {
  const [query, setQuery] = useState("");
  const [onlyProblems, setOnlyProblems] = useState(false);
  const [cls, setCls] = useState("");

  const classes = useMemo(() => [...new Set(devices.map((d) => d.class).filter(Boolean))].sort(), [devices]);
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return devices.filter(
      (d) =>
        (!onlyProblems || d.problemCode !== 0) &&
        (!cls || d.class === cls) &&
        (!q || `${d.name} ${d.manufacturer} ${d.provider} ${d.instanceId}`.toLowerCase().includes(q)),
    );
  }, [devices, query, onlyProblems, cls]);

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
                <div className="sub mono">{d.hardwareIds[0] ?? d.instanceId}</div>
              </td>
              <td>{d.class || "—"}</td>
              <td>{d.provider || d.manufacturer || "—"}</td>
              <td className="mono">{d.version || "—"}</td>
              <td>{formatDate(d.date)}</td>
              <td className={d.problemCode === 0 ? "ok" : "bad"}>{problemText(d.problemCode)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
}
