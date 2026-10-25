import { useCallback, useEffect, useState } from "react";
import { api, type Backup, type RollbackReport } from "../lib/api";
import { formatUnix } from "../lib/format";
import { Dialog } from "./Dialog";

export function Backups({ elevated, refreshKey }: { elevated: boolean; refreshKey: number }) {
  const [list, setList] = useState<Backup[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [target, setTarget] = useState<Backup | null>(null);
  const [doomed, setDoomed] = useState<Backup | null>(null);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<RollbackReport | null>(null);
  const [error, setError] = useState("");

  // biome-ignore lint/correctness/useExhaustiveDependencies: refreshKey is the reload trigger
  useEffect(() => {
    api
      .backups()
      .then(
        (l) => {
          setList(l);
          // A reload that works clears an earlier failed load instead of leaving its alert up.
          setError("");
        },
        (e) => setError(String(e)),
      )
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
      {!elevated && <p className="sub">Execute como administrador para restaurar um backup.</p>}
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
            </div>
            <div className="row-actions">
              <button
                type="button"
                title={elevated ? undefined : "Clique em “Reabrir como administrador” no topo para restaurar"}
                disabled={!elevated || busy || b.entries.length === 0}
                onClick={() => setTarget(b)}
              >
                Restaurar
              </button>
              <button
                type="button"
                title={elevated ? undefined : "Clique em “Reabrir como administrador” no topo para apagar"}
                disabled={!elevated || busy}
                onClick={() => setDoomed(b)}
              >
                Apagar
              </button>
            </div>
          </li>
        ))}
      </ul>
      {doomed && (
        <Dialog
          title="Apagar este backup?"
          onClose={() => setDoomed(null)}
          actions={
            <>
              <button type="button" onClick={() => setDoomed(null)}>
                Cancelar
              </button>
              <button
                type="button"
                className="primary"
                onClick={async () => {
                  const id = doomed.id;
                  setDoomed(null);
                  try {
                    await api.deleteBackup(id);
                    setList((l) => l.filter((b) => b.id !== id));
                  } catch (e) {
                    setError(String(e));
                  }
                }}
              >
                Apagar
              </button>
            </>
          }
        >
          <p>
            Apaga a cópia dos drivers de {formatUnix(doomed.createdUnix)}. Depois disso não será possível restaurar esse
            estado. Não pode ser desfeito.
          </p>
        </Dialog>
      )}
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
