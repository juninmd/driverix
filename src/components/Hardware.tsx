import { useEffect, useState } from "react";
import type { HardwareState } from "../hooks/useHardware";
import type { HwSection } from "../lib/api";

interface Props {
  state: HardwareState;
  onLoad: () => void;
}

export function Hardware({ state, onLoad }: Props) {
  const [selected, setSelected] = useState("summary");
  const [copied, setCopied] = useState<"" | "ok" | "fail">("");

  useEffect(() => {
    if (state.kind === "idle") onLoad();
  }, [state.kind, onLoad]);

  if (state.kind === "idle" || (state.kind === "loading" && !state.previous)) {
    return (
      <p className="progress" role="status">
        Lendo o hardware…
      </p>
    );
  }
  if (state.kind === "error") {
    return (
      <div className="error" role="alert">
        {state.message}{" "}
        <button type="button" onClick={onLoad}>
          Tentar de novo
        </button>
      </div>
    );
  }

  // A refresh keeps the previous view on screen, marked as loading, instead of blanking the panel.
  const view = state.kind === "done" ? state.view : state.previous;
  if (!view) return null;
  const { sections, report } = view;
  const refreshing = state.kind === "loading";
  const current = sections.find((s) => s.id === selected) ?? sections[0];

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(report);
      setCopied("ok");
    } catch {
      setCopied("fail");
    }
    setTimeout(() => setCopied(""), 2500);
  };

  return (
    <div className="hw">
      <nav className="hw-nav" aria-label="Categorias de hardware">
        {sections.map((s) => (
          <button
            key={s.id}
            type="button"
            className={s.id === current.id ? "active" : ""}
            aria-current={s.id === current.id ? "page" : undefined}
            onClick={() => setSelected(s.id)}
          >
            {s.title}
          </button>
        ))}
        <div className="hw-actions">
          <button type="button" onClick={onLoad} disabled={refreshing}>
            {refreshing ? "Atualizando…" : "Atualizar"}
          </button>
          <button type="button" onClick={copy}>
            Copiar relatório
          </button>
          <span className="sub" role="status">
            {copied === "ok" && "Copiado."}
            {copied === "fail" && "Não foi possível copiar."}
          </span>
        </div>
      </nav>
      <Props section={current} />
    </div>
  );
}

function Props({ section }: { section: HwSection }) {
  return (
    <section className="hw-props" aria-label={section.title}>
      <h2>{section.title}</h2>
      {section.rows.length === 0 ? (
        <p className="empty">Nada detectado nesta categoria.</p>
      ) : (
        <dl>
          {section.rows.map((r, i) =>
            r.heading ? (
              // biome-ignore lint/suspicious/noArrayIndexKey: rows are static for a loaded view and labels repeat
              <dt key={i} className="hw-heading">
                {r.label}
              </dt>
            ) : (
              // biome-ignore lint/suspicious/noArrayIndexKey: rows are static for a loaded view and labels repeat
              <div key={i} className="hw-row">
                <dt>{r.label}</dt>
                <dd>
                  {r.value}
                  {r.meter !== null && (
                    <span
                      className={`meter${r.meter >= 90 ? " hot" : ""}`}
                      role="progressbar"
                      aria-label={r.label}
                      aria-valuenow={r.meter}
                      aria-valuemin={0}
                      aria-valuemax={100}
                    >
                      <span style={{ width: `${r.meter}%` }} />
                    </span>
                  )}
                </dd>
              </div>
            ),
          )}
        </dl>
      )}
    </section>
  );
}
