import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// Keep in sync with the serde shapes in src-tauri/src/{model,host,install,backup,scan,hardware,commands}.rs.
export type Decision = "newer" | "same" | "older" | "unknown";

export interface Device {
  instanceId: string;
  name: string;
  class: string;
  manufacturer: string;
  provider: string;
  version: string;
  date: string | null;
  inf: string;
  hardwareIds: string[];
  compatibleIds: string[];
  signed: boolean;
  signer: string;
  problemCode: number;
  vendor: string;
  hint: string;
}

export interface Candidate {
  updateId: string;
  revision: number;
  title: string;
  class: string;
  manufacturer: string;
  provider: string;
  model: string;
  hardwareId: string;
  version: string | null;
  date: string | null;
  sizeBytes: number;
  rebootRequired: boolean;
}

export interface Offer {
  candidate: Candidate;
  deviceIds: string[];
  decision: Decision;
  reason: string;
  fixesProblem: boolean;
  viaCompatible: boolean;
}

export interface ScanResult {
  devices: Device[];
  offers: Offer[];
  /** Windows Update failure; the devices are still valid when this is set. */
  updatesError: string | null;
}

export interface Status {
  elevated: boolean;
  backupsDir: string;
}

export interface UpdateOutcome {
  updateId: string;
  resultCode: number;
  hresult: number;
  rebootRequired: boolean;
  note: string;
}

export interface InstallReport {
  backupId: string;
  restorePoint: "recent" | "created" | "skipped";
  backedUp: number;
  outcomes: UpdateOutcome[];
  rebootRequired: boolean;
  problemDevices: string[];
  /** Devices still on an older driver than the one offered, though Windows Update reported success. */
  staleDevices: string[];
  verified: boolean;
  /** Set when Windows Update itself failed; the backup is still there. */
  installError: string | null;
}

export interface BackupEntry {
  instanceIds: string[];
  deviceName: string;
  oldInf: string;
  oldVersion: string;
}

export interface Backup {
  id: string;
  createdUnix: number;
  restorePoint: string;
  entries: BackupEntry[];
}

export interface RollbackReport {
  restored: number;
  errors: string[];
}

export interface HwRow {
  label: string;
  value: string;
  /** 0-100 usage the row can be drawn as a bar; null when not a usage. */
  meter: number | null;
  /** Groups the rows below it; carries no value. */
  heading: boolean;
}

export interface HwSection {
  id: string;
  title: string;
  rows: HwRow[];
}

export interface HardwareView {
  sections: HwSection[];
  report: string;
}

export type ScanPhase = "inventory" | "windows-update";

export const api = {
  status: () => invoke<Status>("get_status"),
  loadDevices: () => invoke<Device[]>("load_devices"),
  loadHardware: () => invoke<HardwareView>("load_hardware"),
  scanUpdates: () => invoke<ScanResult>("scan_updates"),
  cancel: () => invoke<void>("cancel_operation"),
  install: (updateIds: string[], allowNoRestorePoint: boolean) =>
    invoke<InstallReport>("install_updates", { updateIds, allowNoRestorePoint }),
  backups: () => invoke<Backup[]>("list_backups"),
  rollback: (id: string) => invoke<RollbackReport>("rollback_backup", { id }),
  deleteBackup: (id: string) => invoke<void>("delete_backup", { id }),
  relaunchElevated: () => invoke<void>("relaunch_elevated"),
  onScanPhase: (cb: (p: ScanPhase) => void) => listen<ScanPhase>("scan-phase", (e) => cb(e.payload)),
};
