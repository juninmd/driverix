import { useMemo, useState } from "react";
import type { Offer } from "../lib/api";
import { formatBytes, formatDate } from "../lib/format";
import { OFFER_SORTS, type OfferSort, sortOffers } from "../lib/offerSort";

interface Props {
  offers: Offer[];
  installedVersion: (offer: Offer) => string;
  busy: boolean;
  elevated: boolean;
  onInstall: (offers: Offer[]) => void;
}

export function Updates({ offers, installedVersion, busy, elevated, onInstall }: Props) {
  const installable = useMemo(() => offers.filter((o) => o.decision === "newer"), [offers]);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [sort, setSort] = useState<OfferSort>("source");
  const ordered = useMemo(() => sortOffers(installable, sort), [installable, sort]);

  const toggle = (id: string) =>
    setPicked((s) => {
      const next = new Set(s);
      if (!next.delete(id)) next.add(id);
      return next;
    });
  const allOn = installable.length > 0 && installable.every((o) => picked.has(o.candidate.updateId));
  const toggleAll = () => setPicked(allOn ? new Set() : new Set(installable.map((o) => o.candidate.updateId)));
  const chosen = installable.filter((o) => picked.has(o.candidate.updateId));

  if (installable.length === 0) {
    return (
      <p className="empty" role="status">
        Nenhum driver mais novo no Windows Update. Seus drivers estão em dia para o que a Microsoft publica.
      </p>
    );
  }

  return (
    <>
      <div className="filters">
        <select aria-label="Ordenar atualizações" value={sort} onChange={(e) => setSort(e.target.value as OfferSort)}>
          {(Object.keys(OFFER_SORTS) as OfferSort[]).map((key) => (
            <option key={key} value={key}>
              {OFFER_SORTS[key]}
            </option>
          ))}
        </select>
      </div>
      <table className="data-table">
        <thead>
          <tr>
            <th className="check">
              <input type="checkbox" checked={allOn} onChange={toggleAll} aria-label="Selecionar todas" />
            </th>
            <th>Atualização</th>
            <th>Instalado → novo</th>
            <th>Data</th>
            <th>Tamanho</th>
          </tr>
        </thead>
        <tbody>
          {ordered.map((o) => {
            const c = o.candidate;
            return (
              <tr key={c.updateId}>
                <td className="check">
                  <input
                    type="checkbox"
                    checked={picked.has(c.updateId)}
                    onChange={() => toggle(c.updateId)}
                    aria-label={`Selecionar ${c.title}`}
                  />
                </td>
                <td>
                  <div className="title">{c.title}</div>
                  <div className="sub">
                    <span className="chip">{c.class || "Driver"}</span>
                    {o.fixesProblem && <span className="chip warn">corrige dispositivo com problema</span>}
                    {c.rebootRequired && <span className="chip">pede reinício</span>}
                    {o.viaCompatible && <span className="chip">só ID compatível</span>}
                  </div>
                </td>
                <td className="mono">
                  {installedVersion(o) || "—"} → {c.version ?? "?"}
                </td>
                <td>{formatDate(c.date)}</td>
                <td>{formatBytes(c.sizeBytes)}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <div className="actionbar">
        <span>{chosen.length} selecionada(s)</span>
        <button
          type="button"
          className="primary"
          disabled={busy || chosen.length === 0 || !elevated}
          title={elevated ? undefined : "Execute como administrador para instalar"}
          onClick={() => onInstall(chosen)}
        >
          Instalar selecionadas
        </button>
      </div>
    </>
  );
}
