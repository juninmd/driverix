export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n <= 0) return "—";
  const units = ["B", "KB", "MB", "GB"];
  let v = n;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v >= 100 || i === 0 ? Math.round(v) : v.toFixed(1)} ${units[i]}`;
}

/** `2024-05-06` -> `06/05/2024`, without going through Date so the day never shifts with the timezone. */
export function formatDate(iso: string | null): string {
  const m = iso?.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  return m ? `${m[3]}/${m[2]}/${m[1]}` : "—";
}

// Config Manager problem codes users actually hit; anything else falls back to the number.
const PROBLEMS: Record<number, string> = {
  1: "Mal configurado",
  10: "Não consegue iniciar",
  22: "Desativado",
  28: "Sem driver instalado",
  31: "Não está funcionando direito",
  43: "Parado pelo Windows por falha",
};

export function problemText(code: number): string {
  return code === 0 ? "OK" : (PROBLEMS[code] ?? `Problema (código ${code})`);
}

export function formatUnix(secs: number): string {
  return new Date(secs * 1000).toLocaleString("pt-BR");
}
