import { useMemo, useState } from "react";
import type { Device } from "../lib/api";
import { DEVICE_SORTS, type DeviceSort, sortDevices } from "../lib/devices";
import { formatDate, isOldDriver, problemText } from "../lib/format";

export function Devices({ devices }: { devices: Device[] }) {
  const [query, setQuery] = useState("");
  const [onlyProblems, setOnlyProblems] = useState(false);
  const [onlyOld, setOnlyOld] = useState(false);
  const [cls, setCls] = useState("");
  const [sort, setSort] = useState<DeviceSort>("inventory");

  const classes = useMemo(() => [...new Set(devices.map((d) => d.class).filter(Boolean))].sort(), [devices]);
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    const filtered = devices.filter(
      (d) =>
        (!onlyProblems || d.problemCode !== 0) &&
        (!onlyOld || isOldDriver(d.date, d.provider)) &&
        (!cls || d.class === cls) &&
        (!q ||
          `${d.name} ${d.manufacturer} ${d.provider} ${d.vendor} ${d.hint} ${d.instanceId}`.toLowerCase().includes(q)),
    );
    return sortDevices(filtered, sort);
  }, [devices, query, onlyProblems, onlyOld, cls, sort]);

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
        <select aria-label="Ordenar" value={sort} onChange={(e) => setSort(e.target.value as DeviceSort)}>
          {Object.entries(DEVICE_SORTS).map(([value, label]) => (
            <option key={value} value={value}>
              {label}
            </option>
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
            <th scope="col">Dispositivo</th>
            <th scope="col">Classe</th>
            <th scope="col">Fornecedor</th>
            <th scope="col">Versão</th>
            <th scope="col">Data</th>
            <th scope="col">Estado</th>
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
              <td className={d.problemCode === 0 ? "ok" : "bad"}>
                {problemText(d.problemCode)}
                {d.version && !d.signed && (
                  <span className="chip warn" title="O Windows não reporta assinatura digital para o driver instalado">
                    sem assinatura
                  </span>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
}
