import type { Device } from "./api";
import { isOldDriver } from "./format";

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

export interface IndexedDevice {
  device: Device;
  /** Lowercased text the search box matches against; built once per inventory, not per keystroke. */
  searchText: string;
  old: boolean;
}

export interface DeviceFilter {
  query: string;
  onlyProblems: boolean;
  onlyOld: boolean;
  cls: string;
}

/** Precomputes what filtering needs, so typing only does string `includes` per device. */
export function indexDevices(devices: Device[], now: Date = new Date()): IndexedDevice[] {
  return devices.map((device) => ({
    device,
    searchText:
      `${device.name} ${device.manufacturer} ${device.provider} ${device.vendor} ${device.hint} ${device.instanceId}`.toLowerCase(),
    old: isOldDriver(device.date, device.provider, now),
  }));
}

export function selectDevices(index: IndexedDevice[], filter: DeviceFilter): Device[] {
  const q = filter.query.trim().toLowerCase();
  const out: Device[] = [];
  for (const { device, searchText, old } of index) {
    if (filter.onlyProblems && device.problemCode === 0) continue;
    if (filter.onlyOld && !old) continue;
    if (filter.cls && device.class !== filter.cls) continue;
    if (q && !searchText.includes(q)) continue;
    out.push(device);
  }
  return out;
}
