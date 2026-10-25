import { useCallback, useEffect, useState } from "react";
import { api, type Backup, type RollbackReport } from "../lib/api";
import { formatUnix } from "../lib/format";
import { Dialog } from "./Dialog";

export function Backups({ elevated, refreshKey }: { elevated: boolean; refreshKey: number }) {
  const [list, setList] = useState<Backup[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [target, setTarget] = useState<Backup | null>(null);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<RollbackReport | null>(null);
  const [error, setError] = useState("");

  // biome-ignore lint/correctness/useExhaustiveDependencies: refreshKey is the reload trigger
  useEffect(() => {
    api
      .backups()
      .then(setList, (e) => setError(String(e)))
      .finally(() => setLoaded(true));
  }, [refreshKey]);

  // Closing mid-rollback would hide a half-applied change; the dialog stays until the run ends.
  const close = useCallback(() => {
    if (!busy) setTarget(null);
  }, [busy]);

  // Restoring re-installs drivers as admin; a second click must not start another rollback.
  const run = async () => {
    if (!target || busy) return;
    setBusy(true);
    setError("");
    try {
      setResult(await api.rollback(target.id));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      setTarget(null);
    }
  };

  if (!loaded && !error) {
    return (
      <p className="empty" role="status">
        Carregando backups…
      </p>
    );
  }
  if (list.length === 0 && !error) {
    return <p className="empty">Nenhum backup ainda. Um backup é criado antes de cada instalação.</p>;
  }

  return (
    <>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {result && (
        <p className={result.errors.length ? "error" : "okbox"} role="status">
          {result.restored} driver(s) restaurado(s).
          {result.errors.length > 0 && ` Falhas: ${result.errors.join("; ")}.`}
          {result.restored > 0 && " A lista de atualizações ainda mostra o estado anterior: busque de novo."}
        </p>
      )}
      <p className="summary">{list.length} backup(s), do mais recente ao mais antigo.</p>
      <ul className="cards">
        {list.map((b) => (
          <li key={b.id}>
            <div>
              <div className="title">{formatUnix(b.createdUnix)}</div>
              <div className="sub">
                {b.entries.length} pacote(s) · ponto de restauração: {b.restorePoint}
              </div>
              <div className="sub mono">{b.entries.map((e) => `${e.deviceName} ${e.oldVersion}`).join(" · ")}</div>
            </button>
            <button
              type="button"
              aria-label={`Restaurar backup de ${formatUnix(b.createdUnix)}`}
              title={elevated ? undefined : "Clique em “Reabrir como administrador” no topo para restaurar"}
              disabled={!elevated || busy || b.entries.length === 0}
              onClick={() => setTarget(b)}
            >
              Restaurar
            </button>
          </li>
        ))}
      </ul>
      {target && (
        <Dialog
          title="Restaurar drivers anteriores?"
          onClose={close}
          actions={
            <>
              <button type="button" onClick={close} disabled={busy}>
                Cancelar
              </button>
              <button type="button" className="primary" disabled={busy} onClick={run}>
                Restaurar
              </button>
            </>
          }
        >
          <p>
            Remove os pacotes instalados na atualização de {formatUnix(target.createdUnix)} e reinstala os anteriores a
            partir do backup. Dispositivos podem reiniciar por alguns segundos.
          </p>
          {busy && (
            <p role="status" aria-live="polite">
              Restaurando… não feche o Driverix.
            </p>
          )}
        </Dialog>
      )}
    </>
  );
}
