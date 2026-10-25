import type { Device } from "./api";

export type DeviceSort = "inventory" | "problems" | "name" | "oldest";

export const DEVICE_SORTS: Record<DeviceSort, string> = {
  inventory: "Ordem do sistema",
  problems: "Problemas primeiro",
  name: "Nome (A–Z)",
  oldest: "Driver mais antigo",
};

/** Returns a new array; the input keeps the inventory order. Ties keep that order too. */
export function sortDevices(devices: Device[], sort: DeviceSort): Device[] {
  const copy = [...devices];
  switch (sort) {
    case "problems":
      return copy.sort((a, b) => Number(b.problemCode !== 0) - Number(a.problemCode !== 0));
    case "name":
      return copy.sort((a, b) => (a.name || a.instanceId).localeCompare(b.name || b.instanceId, "pt-BR"));
    case "oldest":
      // Unknown dates sort last; ISO strings compare correctly as text.
      return copy.sort((a, b) => (a.date ?? "9999").localeCompare(b.date ?? "9999"));
    default:
      return copy;
  }
}

/** Tab-separated rows with a header, for pasting into a spreadsheet or a bug report. */
export function devicesToTsv(devices: Device[]): string {
  const clean = (s?: string | null) => {
    const flat = (s ?? "").replace(/[\t\r\n]+/g, " ").trim();
    // Spreadsheets run a cell that starts with = + - @ as a formula; a leading quote keeps it text.
    return /^[=+\-@]/.test(flat) ? `'${flat}` : flat;
  };
  const rows = devices.map((d) =>
    [clean(d.name) || clean(d.instanceId), clean(d.class), clean(d.version), clean(d.instanceId)]
      .map((v) => v || "—")
      .join("\t"),
  );
  return ["Dispositivo\tClasse\tVersão\tID", ...rows].join("\n");
}
