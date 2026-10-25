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

/** Windows reports HRESULTs as signed 32-bit ints; users and Microsoft's docs use the unsigned hex form. */
export function hresultHex(code: number): string {
  return `0x${(code >>> 0).toString(16).toUpperCase().padStart(8, "0")}`;
}

export function formatUnix(secs: number): string {
  return new Date(secs * 1000).toLocaleString("pt-BR");
}

const OLD_DRIVER_YEARS = 3;

/** Whole years from an ISO date to `now`, in UTC so the day never shifts; null when unknown. */
export function ageYears(iso: string | null, now: Date = new Date()): number | null {
  const m = iso?.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  if (!m) return null;
  const [y, mo, d] = [Number(m[1]), Number(m[2]), Number(m[3])];
  const beforeAnniversary = now.getUTCMonth() + 1 < mo || (now.getUTCMonth() + 1 === mo && now.getUTCDate() < d);
  return Math.max(0, now.getUTCFullYear() - y - (beforeAnniversary ? 1 : 0));
}

/**
 * A third-party driver older than three years. Microsoft's inbox drivers are dated 2006-2009 on
 * purpose and are updated through Windows itself, so they never count as old.
 */
export function isOldDriver(date: string | null, provider: string, now: Date = new Date()): boolean {
  if (/microsoft/i.test(provider)) return false;
  const years = ageYears(date, now);
  return years !== null && years >= OLD_DRIVER_YEARS;
}
