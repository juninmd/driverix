import { useCallback, useState } from "react";
import { api, type InstallReport, type Offer } from "../lib/api";
import { hresultHex } from "../lib/format";
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
      // The backup is written before Windows Update runs, so a failed install can still leave one to restore.
      onDone();
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
    // A selected update the Windows Update search no longer returns gets no outcome row at all.
    // A failed Windows Update call leaves no outcomes; those updates were not "not found", so do not count them.
    const missing = report.installError ? 0 : Math.max(0, offers.length - report.outcomes.length);
    return (
      <Dialog
        title={report.installError ? "Instalação não concluída" : "Instalação concluída"}
        onClose={close}
        actions={
          <button type="button" className="primary" onClick={onClose}>
            Fechar
          </button>
        }
      >
        {report.installError && (
          <p className="error" role="alert">
            O Windows Update falhou: {report.installError}. Alguns drivers podem ter sido instalados antes da falha;
            confira os dispositivos abaixo. O backup continua na aba Backups.
          </p>
        )}
        <p>{RESTORE_TEXT[report.restorePoint]}</p>
        <p>
          {Math.max(0, offers.length - failed.length - missing)} de {offers.length} instalada(s). {report.backedUp}{" "}
          pacote(s) com backup.
        </p>
        {missing > 0 && (
          <p className="error">
            {missing} não foi(ram) encontrada(s) pelo Windows Update na hora da instalação e não foi(ram) instalada(s).
          </p>
        )}
        {failed.length > 0 && (
          <>
            <p className="error">
              {failed.length} {failed.length === 1 ? "falhou" : "falharam"}:
            </p>
            <ul className="plain">
              {failed.map((o) => {
                const id = o.updateId.toLowerCase();
                const title = offers.find((x) => x.candidate.updateId.toLowerCase() === id)?.candidate.title;
                return (
                  <li key={o.updateId}>
                    <strong>{title ?? o.updateId}</strong> · {hresultHex(o.hresult)}
                    {o.note && `: ${o.note}`}
                  </li>
                );
              })}
            </ul>
          </>
        )}
        {!report.verified && (
          <p className="error">
            Não foi possível verificar os dispositivos após a instalação. Confira o Gerenciador de Dispositivos ou
            busque de novo.
          </p>
        )}
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
        Antes de instalar, o Driverix cria um ponto de restauração e exporta os drivers atuais. Se a exportação falhar,
        nada é instalado. Se o ponto de restauração falhar, a instalação só segue se você marcar a opção abaixo. Os
        drivers vêm do Windows Update, assinados pela Microsoft.
      </p>
      <label className="inline">
        <input type="checkbox" checked={skipRestore} onChange={(e) => setSkipRestore(e.target.checked)} /> Continuar
        mesmo se não for possível criar o ponto de restauração
      </label>
      {running && (
        <p role="status" aria-live="polite">
          Instalando… pode levar alguns minutos. Não feche o Driverix.
        </p>
      )}
      {error && (
        <>
          <p className="error" role="alert">
            {error}
          </p>
          <p>Se um backup foi criado antes da falha, ele aparece na aba Backups.</p>
        </>
      )}
    </Dialog>
  );
}
