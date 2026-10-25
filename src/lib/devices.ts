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
