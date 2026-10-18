// Fake Tauri backend for README screenshots: inject before the app loads (e.g. Playwright
// addInitScript) while `bun run dev` serves the UI in a plain browser. Everything is invented.
(() => {
  const dev = (i, name, cls, mfr, version, date, extra = {}) => ({
    instanceId: `PCI\\VEN_DEMO&DEV_${1000 + i}\\4&1A2B3C&0&00E${i}`,
    name,
    class: cls,
    manufacturer: mfr,
    provider: mfr,
    version,
    date,
    inf: `oem${40 + i}.inf`,
    hardwareIds: [`PCI\\VEN_DEMO&DEV_${1000 + i}`],
    compatibleIds: [],
    signed: true,
    signer: "Microsoft Windows Hardware Compatibility Publisher",
    problemCode: 0,
    ...extra,
  });

  const devices = [
    dev(1, "NVIDIA GeForce RTX 4070", "Display", "NVIDIA", "31.0.15.3623", "2023-09-12"),
    dev(2, "Intel(R) Wi-Fi 6E AX211 160MHz", "Net", "Intel", "22.230.0.8", "2023-06-02"),
    dev(3, "Realtek PCIe 2.5GbE Family Controller", "Net", "Realtek", "10.66.1019.2023", "2023-10-19"),
    dev(4, "Realtek High Definition Audio", "MEDIA", "Realtek", "6.0.9549.1", "2023-04-11"),
    dev(5, "Intel(R) Management Engine Interface", "System", "Intel", "2306.2.5.0", "2023-07-04"),
    dev(6, "Intel(R) USB 3.20 eXtensible Host Controller", "USB", "Intel", "10.0.22621.1", "2022-06-21"),
    dev(7, "Standard NVM Express Controller", "SCSIAdapter", "Microsoft", "10.0.22621.2506", "2023-10-10"),
    dev(8, "Intel(R) Bluetooth(R)", "Bluetooth", "Intel", "23.20.0.4", "2023-05-18"),
    dev(9, "HID-compliant touch pad", "HIDClass", "Microsoft", "10.0.22621.1", "2022-06-21"),
    dev(10, "Generic PnP Monitor", "Monitor", "Microsoft", "10.0.22621.2506", "2023-10-10"),
    dev(11, "Dispositivo desconhecido", "Unknown", "", "", null, { problemCode: 28, signed: false, signer: "" }),
  ];

  const cand = (n, title, cls, mfr, version, date, size, reboot) => ({
    candidate: {
      updateId: `00000000-0000-4000-8000-00000000000${n}`,
      revision: 1,
      title,
      class: cls,
      manufacturer: mfr,
      provider: mfr,
      model: "",
      hardwareId: devices[n - 1].hardwareIds[0],
      version,
      date,
      sizeBytes: size,
      rebootRequired: reboot,
    },
    deviceIds: [devices[n - 1].instanceId],
    decision: "newer",
    reason: `Versão ${version} é mais nova que ${devices[n - 1].version || "nenhuma"}`,
    fixesProblem: false,
    viaCompatible: false,
  });

  const offers = [
    cand(1, "NVIDIA - Display - 31.0.15.5222", "Display", "NVIDIA", "31.0.15.5222", "2024-03-04", 612_000_000, true),
    cand(2, "Intel - Net - 23.50.0.7", "Net", "Intel", "23.50.0.7", "2024-02-13", 14_200_000, false),
    cand(4, "Realtek - MEDIA - 6.0.9700.1", "MEDIA", "Realtek", "6.0.9700.1", "2024-01-22", 9_800_000, false),
    cand(5, "Intel - System - 2403.2.8.0", "System", "Intel", "2403.2.8.0", "2024-03-19", 5_300_000, true),
  ];

  const backups = [
    {
      id: "1730000000",
      createdUnix: 1730000000,
      restorePoint: "created",
      entries: [
        {
          instanceIds: [devices[1].instanceId],
          deviceName: devices[1].name,
          oldInf: "oem41.inf",
          oldVersion: "22.230.0.8",
        },
        {
          instanceIds: [devices[3].instanceId],
          deviceName: devices[3].name,
          oldInf: "oem43.inf",
          oldVersion: "6.0.9549.1",
        },
      ],
    },
  ];

  const handlers = {
    get_status: () => ({ elevated: true, backupsDir: "C:\\ProgramData\\Driverix\\backups" }),
    load_devices: () => devices,
    scan_updates: () => ({ devices, offers }),
    list_backups: () => backups,
    cancel_operation: () => null,
  };

  let cbId = 1;
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
  window.__TAURI_INTERNALS__ = {
    transformCallback: () => cbId++,
    unregisterCallback() {},
    convertFileSrc: (p) => p,
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main", windowLabel: "main" } },
    invoke: async (cmd) => {
      if (cmd.startsWith("plugin:event|")) return 0;
      const h = handlers[cmd];
      if (!h) throw new Error(`demo mock: unhandled ${cmd}`);
      return h();
    },
  };
})();
