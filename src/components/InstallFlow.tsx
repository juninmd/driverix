import { useCallback, useState } from "react";
import { api, type InstallReport, type Offer } from "../lib/api";
import { Dialog } from "./Dialog";

interface Props {
  offers: Offer[];
  onClose: () => void;
  onDone: () => void;
}

const RESTORE_TEXT = {
  recent: "Já existia um ponto de restauração das últimas 24 h.",
  created: "Ponto de restauração criado.",
  skipped: "Sem ponto de restauração (você optou por seguir).",
} as const;

export function InstallFlow({ offers, onClose, onDone }: Props) {
  const [skipRestore, setSkipRestore] = useState(false);
  const [running, setRunning] = useState(false);
  const [report, setReport] = useState<InstallReport | null>(null);
  const [error, setError] = useState("");

  const run = async () => {
    setRunning(true);
    setError("");
    try {
      setReport(
        await api.install(
          offers.map((o) => o.candidate.updateId),
          skipRestore,
        ),
      );
      onDone();
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  };

  // The dialog must not be dismissible mid-install: closing would hide a half-applied change.
  const close = useCallback(() => {
    if (!running) onClose();
  }, [running, onClose]);

  if (report) {
    const failed = report.outcomes.filter((o) => o.resultCode !== 2);
    return (
      <Dialog
        title="Instalação concluída"
        onClose={close}
        actions={
          <button type="button" className="primary" onClick={onClose}>
            Fechar
          </button>
        }
      >
        <p>{RESTORE_TEXT[report.restorePoint]}</p>
        <p>
          {report.outcomes.length - failed.length} de {report.outcomes.length} instalada(s). {report.backedUp} pacote(s)
          com backup.
        </p>
        {failed.length > 0 && <p className="error">{failed.length} falharam; veja o Windows Update para detalhes.</p>}
        {!report.verified && <p className="error">Não foi possível verificar os dispositivos depois da instalação.</p>}
        {report.problemDevices.length > 0 && (
          <p className="error">
            Com problema agora: {report.problemDevices.join(", ")}. Restaure o backup na aba Backups.
          </p>
        )}
        {report.staleDevices.length > 0 && (
          <p className="error">
            Ainda na versão anterior: {report.staleDevices.join(", ")}. Pode ser preciso reiniciar; se persistir, busque
            de novo.
          </p>
        )}
        {report.rebootRequired && <p>Reinicie o computador para concluir.</p>}
      </Dialog>
    );
  }

  return (
    <Dialog
      title={`Instalar ${offers.length} atualização(ões)?`}
      onClose={close}
      actions={
        <>
          <button type="button" onClick={close} disabled={running}>
            Cancelar
          </button>
          <button type="button" className="primary" onClick={run} disabled={running}>
            {running ? "Instalando…" : "Instalar"}
          </button>
        </>
      }
    >
      <ul className="plain">
        {offers.map((o) => (
          <li key={o.candidate.updateId}>{o.candidate.title}</li>
        ))}
      </ul>
      <p>
        Antes de instalar, o Driverix cria um ponto de restauração e exporta os drivers atuais. Se algum desses passos
        falhar, nada é instalado. Os drivers vêm do Windows Update, assinados pela Microsoft.
      </p>
      <label className="inline">
        <input type="checkbox" checked={skipRestore} onChange={(e) => setSkipRestore(e.target.checked)} /> Continuar
        mesmo se não for possível criar o ponto de restauração
      </label>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </Dialog>
  );
}
