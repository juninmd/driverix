# Research notes: how driver updaters work

Why Driverix is built the way it is. Sections 1-7 come from public sources; no proprietary binary
was downloaded, run or decompiled for them. The EULA question was never checked against the actual
EULA texts. Since 2026-10-28 the owner explicitly allows static analysis (decompilation) of Driver
Booster Free to find features (section 8); that decision is the owner's, it does not extend to other
products. Where the vendors document nothing, this file says UNCONFIRMED instead of guessing.

Scope of sections 1-7: public sources only (Microsoft Learn, vendor support pages, search snippets).
Legend: **[V]** verified by fetching the page in this session. **[S]** seen only in a search snippet. **[U]** UNCONFIRMED (could not verify; where I looked is listed).

Fetch failures this session (HTTP 404, so not used): the SDI Origin GitHub URL I guessed, the `win32/cimwin32prov` and `whqlprov` Win32_PnPSignedDriver URLs I tried directly, the Win32_PnPSignedDriver class page via search snippet only, and the `introduction-to-windows-driver-signing` page.

---

## 1. Hardware detection

| Source | What it exposes | Status |
|---|---|---|
| `Win32_PnPSignedDriver` (WMI, `root\cimv2`) | `DeviceID`, `HardWareID` (note capitalisation), `CompatID`, `DriverVersion`, `DriverDate` (build date set by manufacturer), `DriverProviderName`, `InfName`, `IsSigned`, `Signer` | [V] field list from the class page at https://learn.microsoft.com/en-us/previous-versions/windows/desktop/whqlprov/win32-pnpsigneddriver (via search snippet, [S]). `DriverDate` is a WMI datetime string that must be converted. Source: https://devblogs.microsoft.com/scripting/?p=67723 [S] |
| `Win32_PnPEntity` | `PNPDeviceID`, `ConfigManagerErrorCode` (uint32), `Status` | [V] property exists, https://learn.microsoft.com/en-ie/Windows/Win32/cimwin32prov/win32-pnpentity [S]. Meanings of codes 0-5 come only from a third-party API reference (ILNumerics), not Microsoft: 0 OK, 1 not configured, 2 driver cannot load, 3 corrupt driver or low resources, 4 not working (driver/registry), 5 resource conflict. **[U]** codes above 5 and the official table; looked at the MS "Device Manager error codes" page, not found. |
| `pnputil /enum-devices /drivers` | Matching and installed drivers per device, and `/deviceids` (hardware and compatible IDs), `/problem [code]`, `/connected` | [V] https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/pnputil-command-syntax |
| SetupAPI / CfgMgr32 (`SetupDiGetDeviceRegistryProperty` with `SPDRP_HARDWAREID`/`SPDRP_COMPATIBLEIDS`, `CM_Get_DevNode_Status`) | Native equivalents of the WMI data; used by SDI Origin-style scanners | **[U]** exact function list not fetched; from training knowledge, verify before use. |
| Registry `HKLM\SYSTEM\CurrentControlSet\Enum\<bus>\<id>` | Per-device `HardwareID`, `CompatibleIDs`, `Driver` (class GUID + instance), `Service` | **[U]** not fetched; standard layout, verify. |
| `HKLM\SYSTEM\CurrentControlSet\Control\Class\<ClassGUID>\<NNNN>` | Installed driver key per class, `DriverDesc`, `DriverVersion`, `DriverDate` | **[U]** not fetched. |

Fields that matter for an updater: `HardwareID` (most specific match), `CompatibleID`, `DriverVersion` + `DriverDate` (both, since version strings are vendor-specific), `DriverProviderName`, `InfName` (ties the device to a `oem#.inf` in the driver store), `IsSigned`/`Signer`, `ConfigManagerErrorCode`, and the device's `InstanceId` (needed for rollback).

## 2. Driver matching and ranking

### 2.1 Matching
- A package matches when one of the device's hardware or compatible IDs appears in an INF `Models` entry whose target OS fits. [S] https://learn.microsoft.com/en-us/windows-hardware/drivers/install/overview-of-the-driver-selection-process
- Hardware IDs (more specific, e.g. `PCI\VEN_xxxx&DEV_yyyy&SUBSYS_...`) outrank compatible IDs (generic class-level, e.g. `PCI\CC_0300`). **[U]** the exact identifier-score values for each match type were not fetched; the `identifier-score` page is the source to check.

### 2.2 Rank (verified format)
[V] https://learn.microsoft.com/en-us/windows-hardware/drivers/install/how-windows-ranks-driver-packages

- Rank is a DWORD `0xSSGGTHHH`: signature score (SS), feature score (GG), identifier score (THHH). **Lower is better**.
- Unknown signature score: `0xFF000000`. Unspecified feature score: `0x00FF0000`.

Signature score (SS) [V] https://learn.microsoft.com/en-us/windows-hardware/drivers/install/signature-score--windows-vista-and-later-:
1. Best: trusted signature. Includes WHQL (standard and Premium), inbox packages, Windows SE, and third-party Authenticode from an Enterprise CA, a Class 3 CA, or MakeCert.
2. Unsigned or invalid, but installed by a DDInstall section with an `.nt` platform extension.
3. Unsigned or invalid, not `.nt`-specific.
4. Worst: unsigned or unknown signing state.

Signed Microsoft packages outrank Authenticode-signed ones unless the `AllSignersEqual` policy is on. [S] (snippet, default not confirmed; check the rank page.)

### 2.3 Final selection is more than rank
[V] The rank page itself says rank is one input. The selection page says INF date, driver version, and critical/automatic/optional category also matter. [S] https://learn.microsoft.com/en-us/windows-hardware/drivers/install/overview-of-the-driver-selection-process
- Windows Update prefers critical or automatic packages, and offers the best match, not necessarily the newest. [S]
- `pnputil /add-driver` does not force a non-top-ranked driver onto a device. [V] (note in the pnputil page).

### 2.4 Deciding "newer" (recommended rule, not a Microsoft rule)
Compare in this order, and only offer an update when all hold:
1. Same device identity: the candidate's HWID matches the device's current HWID (not only a compatible ID), and the INF's OS target fits the running build.
2. Signature: the candidate is at least as trusted as the installed package (SS value not worse). Never downgrade trust.
3. Version: compare `DriverVersion` as a dotted numeric tuple. Do not use the string. Some vendors use non-monotonic schemes, so also compare `DriverDate`.
4. Date tie-break only when versions are equal or unparseable.
5. Never auto-offer a candidate whose rank is worse than the installed package's rank.

This is a design recommendation; verify against your test matrix.

## 3. Update sources

### 3.1 Windows Update Agent (WUA) search
[V] https://learn.microsoft.com/en-us/windows/win32/api/wuapi/nf-wuapi-iupdatesearcher-search
- Criteria is a SQL-WHERE-like string, case-insensitive for strings. Driver search uses `Type`:
  - `Type='Driver'` (string, `=` or `!=`). Example: `IsInstalled=0 and Type='Driver' and IsHidden=0`.
- Default criteria: `(IsInstalled = 0 and IsHidden = 0)`. Note that `OR` is only allowed at the top level.
- `IsAssigned=1`: at most one assigned Windows driver update per local device.
- `BrowseOnly=1`: optional updates. `RebootRequired=1`: needs restart.
- Requires `ISearchResult` via COM (`Microsoft.Update.Session`). Usable from PowerShell: `New-Object -ComObject Microsoft.Update.Session`.
- Restrictions: WUA is a Windows Update path; the server returns what Microsoft publishes. A third-party tool cannot search for vendor-unpublished drivers this way.

### 3.2 Microsoft Update Catalog
- Source: https://catalog.update.microsoft.com (FAQ: "a listing of updates that can be distributed over a corporate network"). [S] https://catalog.update.microsoft.com/Faq.aspx
- Privacy statement: the Catalog uses a GUID stored on the machine to count download success or failure. [S] https://www.catalog.update.microsoft.com/Privacy.aspx
- **ToS / scraping: UNCONFIRMED.** I searched for the Catalog's own terms and did not find them. The Microsoft Services Agreement clause that bans scraping covers AI services, not the Catalog. Read the site's current legal notice before automating it. Do not assume scraping is permitted.
- Third-party parser exists (Poushec.UpdateCatalogParser, NuGet), which shows programmatic access is common. It does not establish permission. [S] https://www.nuget.org/packages/poushec.updatecatalogparser

### 3.3 winget
- **UNCONFIRMED** for driver coverage. winget manifests are for applications; I did not verify any driver-manifest support. Check https://learn.microsoft.com/en-us/windows/package-manager/ and https://github.com/microsoft/winget-pkgs before relying on it.

### 3.4 NVIDIA
- No official public API for the driver lookup endpoint was found. [S] only.
- Unofficial reference: the nv-driver-locator script in the nvidia-patch project queries a "GeForce Experience" endpoint, with params for OS version/build, language (1033 default), 64-bit, mobile, and beta/DCH flags. It also scrapes nvidia.com's download pages with numeric product-family IDs. Source: https://git.blob42.xyz/Archives/nvidia-patch/src/commit/07691bbcd46a7cf361eefce199483e695346cf7b/tools/nv-driver-locator [S]
- **UNCONFIRMED:** exact endpoint URL and params from the official side. Treat as unstable, and do not rely on undocumented endpoints in a shipped product.
- ToS: NVIDIA's site terms restrict automated access; **UNCONFIRMED** text not fetched.

### 3.5 AMD
- **UNCONFIRMED.** Not researched beyond the task scope. AMD's auto-detect runs through AMD Software/Adrenalin; no public endpoint documentation was found.

### 3.6 Intel Driver & Support Assistant (DSA)
- [S] https://www.intel.com/content/www/us/en/support/articles/000056718.html
- Scans Intel graphics, active Wi-Fi, certain Ethernet adapters, Intel NUC, and RST. Does not cover server, Linux, or chipset drivers (those go to Download Center).
- Network: localhost ports 28385-28389; connects to `www.intel.com`, `dsadata.intel.com`, `downloadmirror.intel.com` on 443. [S]
- Claim that DSA replaces the older Intel Driver Update Utility: **[U]**, only seen on a third-party blog.

### 3.7 Commercial updaters (Driver Easy, CCleaner Driver Updater, IObit Driver Booster)
- Public detection and matching logic is **not documented** by the vendors. I found no vendor technical doc for Driver Easy's or CCleaner's internals. **[U]**.
- Driver Booster: Malwarebytes classes it as PUP.Optional.DriverBooster, usually installed through bundles. [S] https://www.malwarebytes.com/blog/detections/pup-optional-driverbooster
- Reports that the tool blocks installation of found updates unless the user buys Pro. [S] (secondary security site, not verified on the vendor site.)

### 3.8 Snappy Driver Installer Origin (SDI-O)
- Open source, portable, described as a fork of Snappy Driver Installer. **[U]** license not verified. Uses DriverPacks (indexes plus packs, some >10 GB), offline USB use, and expert mode with "Found in driverpacks" / "Absent in driverpacks". [S] (search snippets, not fetched.)
- Pack contents come from a third-party pack distributor (SamLab per secondary manual). Packs are the trust boundary: signatures and provenance of each pack are the user's risk.
- Repo URL: **[U]**. My fetch of the guessed GitHub URL returned 404. Confirm the canonical URL (snappy-driver-installer.org / SourceForge are cited by a Scribd manual, secondary) before linking.
- **UNCONFIRMED:** the exact hardware-ID index format and matching code. Read the repo source to verify, do not infer.

### 3.9 DriverStore Explorer (RAPR)
- **UNCONFIRMED** (not fetched). Expected to enumerate `pnputil /enum-drivers` output and driver store contents; verify on GitHub before citing.

## 4. Safe install pipeline

### 4.1 Signature verification
- Catalog (`.cat`) files bind an INF plus its files to a signature. [U] (the intro-to-driver-signing page returned 404; from training knowledge.)
- WHQL = Microsoft-signed, the top SS category in section 2.2. [V]
- Authenticode third-party driver packages get the next category. [V]
- Verify with `WinVerifyTrust` (Wintrust.dll, `WINTRUST_ACTION_GENERIC_VERIFY_V2`) on the catalog and the `.inf`/`.sys`, and check the signer chain. **[U]** exact GUID/struct not fetched; verify against https://learn.microsoft.com/en-us/windows/win32/api/wintrust/nf-wintrust-wintrustverify before coding.
- Refuse any package that is unsigned or has an invalid signature. Authenticode from unknown CAs is a supply-chain risk and should require explicit opt-in.

### 4.2 pnputil (verified syntax)
[V] https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/pnputil-command-syntax

```
PNPUTIL /add-driver <filename.inf | *.inf> [/subdirs] [/install] [/reboot]
PNPUTIL /delete-driver <oem#.inf> [/uninstall] [/force] [/reboot]
PNPUTIL /export-driver <oem#.inf | *> <target directory>
PNPUTIL /enum-drivers [/class <name | GUID>] [/files]
PNPUTIL /enum-devices [/instanceid <id> | /deviceid <id>] [/drivers] [/deviceids] [/problem [<code>]]
PNPUTIL /scan-devices [/instanceid <id>] [/async]
PNPUTIL /restart-device [<instance ID> | /deviceid <device ID>] [/reboot]
```

- Run elevated. Available from Windows 10 1607 (add/delete/export/enum-drivers); `/enum-devices` from 1903.
- `/add-driver` only adds to the store. Add `/install` to apply to matching devices. `/install` does not force a lower-ranked package.
- `/reboot` lets pnputil reboot if needed. An updater should prefer no `/reboot` and detect reboot pending itself.
- Legacy forms still work: `-a [-i]`, `-d [-f]`, `-e`. Use the `/` forms.
- **Exit codes:** not fetched. **[U]**: check the pnputil examples page and test each code empirically.
- Use `/export-driver` before replacing a package to create a rollback copy. Use `/delete-driver oem#.inf /uninstall` to revert a newly added package only after the old package is re-added with `/add-driver /install`.

### 4.3 Restore point
- `Checkpoint-Computer -Description "<text>" -RestorePointType DEVICE_DRIVER_INSTALL`. Accepted values: `APPLICATION_INSTALL` (default), `APPLICATION_UNINSTALL`, `DEVICE_DRIVER_INSTALL`, `MODIFY_SETTINGS`, `CANCELLED_OPERATION` (unsupported). [V] https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.management/checkpoint-computer
- Frequency limit: at most one restore point per 24 hours via API/PowerShell. Error: "A new system restore point cannot be created because one has already been created within the past 24 hours." [V] (same page)
- Override: `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore`, DWORD `SystemRestorePointCreationFrequency`, minutes, default 1440, 0 disables the skip. [S] Sources: https://www.ninjaone.com/blog/change-system-restore-point-creation-frequency-in-windows/ and https://elevenforum.com/t/change-system-restore-point-creation-frequency-in-windows-11.3613/. HowToGeek says the default is 0, which conflicts with Microsoft's 24h behavior; treat as wrong. **Do not ship a registry override** to bypass the limit: it changes system-wide behavior. If a restore point fails, abort the install and say so.
- Restore points are client-OS only. Some systems have System Restore disabled, so detect and report that, do not silently proceed.

### 4.4 Rollback
1. Before install: `/export-driver` the current package to a per-run folder, record old `oem#.inf`, `DriverVersion`, `InstanceId`, and the restore point name.
2. Install with `/add-driver ... /install`.
3. Verify: re-query `Win32_PnPEntity.ConfigManagerErrorCode` for the device. Non-zero = failed; roll back.
4. Rollback: `/add-driver <exported.inf> /install`, then `/delete-driver <new oem#.inf> /uninstall`.
5. Last resort: System Restore to the named point (`Restore-Computer` is client-only; verify before use).

### 4.5 Reboot detection
- `pnputil` `/reboot` triggers an automatic reboot; avoid it. **[U]**: the reboot-pending registry paths (e.g. `Component Based Servicing\RebootPending`, `Session Manager\PendingFileRenameOperations`) were not fetched. Verify before relying on them.
- WUA result `RebootRequired=1` (see 3.1) is the authoritative flag for Windows-Update-delivered drivers. [V]

## 5. Known pitfalls of commercial updaters (what a better OSS tool should avoid)

| Pitfall | Evidence | Status |
|---|---|---|
| Bundled adware / PUP installers | Driver Booster classed as PUP, usually bundled. [S] Malwarebytes | Secondary but credible |
| Paywalled install of found drivers | Driver Booster reportedly blocks install unless Pro. [S] | Not verified on vendor site |
| Exaggerated scans to sell upgrades | Malwarebytes notes "system optimizers" do this. [S] | Credible |
| Wrong or unstable driver matches | Mostly user forum reports (Steam thread). [S] | Anecdotal |
| Installing without restore point | SDI-O manual advises it; general. [S] | Advice only |
| Undocumented vendor endpoints | NVIDIA lookup exists only in unofficial code. [S] | Breaks without notice |
| Scraping a catalog without permission | Catalog ToS not confirmed. [U] | Open legal question |

Design rules for an open-source replacement:
- Verify signature and rank before offering any install. Show rank and signer in the UI.
- Never install an unsigned or unknown-signature package by default. Require explicit opt-in per package.
- Use WUA (`Type='Driver'`) as the default source, since it is the only documented, first-party driver search API. Use vendor tools only when their public download pages are documented or the vendor permits it.
- No paywall gate on the install step. No bundled third-party installers.
- Export the old driver before every change, record `InstanceId`, and verify `ConfigManagerErrorCode` after install.
- Keep vendor credentials (none expected) out of logs.

## 6. Open items (for the next research pass)
1. Confirm Win32_PnPSignedDriver and `HardWareID` ordering at the official page URL (current one returned 404 via fetch).
2. Confirm the official `identifier-score` values (HWID vs CompatID).
3. Read the SDI-O source repo to confirm its matching and index format.
4. Confirm pnputil exit codes and reboot-pending registry keys.
5. Read Catalog and NVIDIA/AMD/Intel terms of use before any automation.
6. Verify the `WinVerifyTrust` call signature from the Learn page before writing code.

## 7. Open-source competitors on GitHub (surveyed 2026-10-19)

Source: public GitHub search (`driver-updater` topic) and each project's README page; nothing was run or decompiled. Star counts are small across the board, so this is a young field.

| Project | Stack | What it does that Driverix did not | Adopted? |
|---|---|---|---|
| OpenDriverUpdater | C# / WinUI 3 | Trust scoring per candidate, `export-json`, CLI. Update Catalog and vendor sources are planned, not shipped. | JSON export already exists (`driverix-cli --json`); scoring left for later. |
| Hexnest | C# / Avalonia | Device list grouped by class with hardware IDs, OS/CPU/BIOS/memory/disk/network summary, plain-text hardware report, install queue that survives reboots, offline INF repositories. | **Yes:** Hardware tab and text report. Offline INF repositories and the resumable queue are roadmap candidates. |
| Karate | C# / WPF | Three driver channels (Windows Update, Update Catalog, NVIDIA direct), `pnputil` snapshot of third-party drivers. | No: Catalog and NVIDIA terms are unconfirmed (section 3.2 and 3.4). |
| win-driver-updater | Python / Tk | Shows Vendor/Device IDs for code 28 devices so a driver can be found by hand; category filters; Update Catalog `.cab` download. | **Yes:** the ID decoding (`vendors.rs`). The Catalog download is not adopted for the same ToS reason. |
| DriverUpdate-Pro | Electron | Only links to manufacturer download pages. | No. |

Takeaways applied: decode hardware IDs offline so unnamed devices are still identifiable; flag old third-party drivers (Microsoft inbox drivers excluded, their 2006 dates are by design); give users an AIDA64-like hardware overview they can copy into a forum post. Hardware vendor ID tables come from the public PCI-SIG and USB-IF registries; extend them in `vendors.rs`, the test there rejects duplicates and malformed IDs.

### 7.1 More projects (GitHub API search + READMEs, 2026-10-28)

Metadata from `gh api search/repositories` and each README; no code was run. Licenses matter: GPL
projects are read for ideas only, nothing is copied.

| Project | Stack / license | What it adds beyond Driverix | Verdict |
|---|---|---|---|
| lostindark/DriverStoreExplorer (RAPR, ~12k stars, active) | C#, GPL-2.0 | Driver-store browser: smart cleanup of old versions, three backends (native, DISM, PnPUtil), offline images, CSV export, device association. | Idea: "old versions" cleanup for the backups and store. |
| Hanatarou/DriverStoreManager | Rust, MIT | Marks a package *Old*, or *Review* when version and date disagree about which is newer; backup verified file by file with SHA-256 plus a `manifest.txt` listing the devices; re-checks the live store right before removal because Windows reuses `oemNN` numbers. | **Closest in spirit.** Adopt the version-vs-date disagreement rule and per-file SHA-256 verification of exports. |
| raincleaner/RainDriver | README only for a closed commercial product | Export with checksums and a restore order (network, then storage); Device Manager code explainer; change timeline; OEM catalogs (Dell, HP, Lenovo) as sources. | Ideas only. OEM catalog terms are **UNCONFIRMED**. |
| Dev-TheNorthFace/Driver-Backup-Manager | README only | DISM export to external drive, offline restore, inventory file. | Confirms the backup/restore need. |
| HawaiiBeach/TinyNvidiaUpdateChecker (~1.9k stars, active) | C#, GPL-3.0 | NVIDIA version check and download. | No: depends on NVIDIA endpoints (section 3.4). |
| techygeekshome/DriverGeek | C#, GPL-3.0 | Windows Update driver inventory only (per a news article, README not read). | Same source as ours, no install. |
| AbdulMoizAns/win-driver-updater | Python, MIT | Already in the table above. | Done. |

Not on GitHub: Snappy Driver Installer Origin lives on SourceForge. Search also returned repos named
after IObit or "booster" with no code and star-farming forks; treat those as untrusted and do not run
anything from them.

Findings that change the plan for Driverix:
- Verify exported backups per file with SHA-256 and store the hashes in the manifest (check what
  `backup.rs` does today before adding).
- Add the *Review* state: when version order and date order disagree, do not offer an install, show why.
  This also covers the 1970 and future-date cases from section 8.2.
- Restore order when many packages are restored: network and storage drivers first.

## 8. Driver Booster Free 10.2.0: feature gap (static read, 2026-10-28)

Owner-authorised (see the header). Method: read-only inventory of the installed product folder and
of its UI string table (`Language\English.lng`, UTF-16 LE). **Observed:** which screens, tools and
options exist. **Not done:** `DriverBooster.exe` was not decompiled, nothing was run or sniffed, and
its cloud driver database and endpoints were not examined. They would break the Windows-Update-only
invariant in `AGENTS.md`. Nothing from the product (text, code, data) is copied into this repo; the
list below is our own wording of behaviour.

| Driver Booster feature | Driverix today | Verdict |
|---|---|---|
| Ignore list: skip one update, or every update of a device | none | **Adopt** (pure UI + persisted ids; `guard` still re-checks) |
| Scheduler: scan at startup or at a fixed frequency, tray, auto-update when idle | none | **Adopt scan-only** (Task Scheduler + notify); unattended install stays out |
| Fix Device Error: list Config Manager problem codes, explain, retry | reads the code, shows it raw | **Adopt** (pt-BR code table + `pnputil /restart-device`) |
| Clean invalid (disconnected) device data, driver files kept | none | **Adopt**, admin + explicit confirm (`pnputil /enum-devices /disconnected`, `/remove-device`) |
| Driver update history | manifests exist in backups, no history view | **Adopt** (derive from manifests) |
| Export list of found updates | `--json` in CLI only | **Adopt** (UI button) |
| Backup retention (max versions kept) | unbounded | **Adopt** (prune oldest, never the newest) |
| Auto reboot after install | reboot flag reported | **Maybe** (opt-in, only when `RebootRequired`) |
| System Restore tab (list restore points) | creates/reuses one only | Maybe, low value |
| Offline Driver Updater (export device info, fetch on another PC, install) | JSON device export | **Blocked**: needs a source other than Windows Update (Update Catalog, ToS unconfirmed, section 3.2) |
| WHQL-only filter | N/A: Windows Update packages are always Microsoft-signed | Skip |
| Auto restore point / auto backup toggles | always on by design | Skip (invariant) |
| Game-driver scan, vendor-direct drivers | none | Skip (vendor endpoints, section 3.4-3.6) |
| Fix network (hosts file, clock), fix sound, fix resolution | rollback covers the driver cause | Skip (not driver update; rollback is the safer fix) |
| Software updater, VPN, screen recorder, game boost, PC scan, sale/trial screens | none | Skip (out of scope; upsell surface) |

Takeaway: the useful, safe delta is small and local. None of it needs a new update source. Suggested
order: ignore list, device-error table with retry, update history + export, backup retention, ghost
device cleanup, scan-only scheduler.

### 8.1 How it works (static analysis, local files only)

Method: strings of the install-flow modules plus a Ghidra 12.1.4 headless decompile of a copy of
`DriverUpdate.exe` (Delphi, x86; the REA `analyze` run hit its 330 s provider timeout, so Ghidra was
driven directly). No network, the app was never run. **Observed** = visible in imports, strings or
decompiled code. **Inferred** = reading of names only. Delphi RTTI is stripped to `FUN_xxxxxxxx`, so
the call graph of the install flow was not recovered. Coverage is partial, not a full audit.

- **Install is out of process (observed).** The app shells out to `dpinst.exe` / `DpInstX32|64.exe`;
  `dbinst.exe` calls `UpdateDriverForPlugAndPlayDevices`. A named mutex stops a second installer
  (log line: another instance exists), and the helper's `DPINST.LOG` is copied into the app logs and
  deleted. **Driverix:** already uses `pnputil`; it has an in-process `BusyGuard`, but nothing stops
  two app instances installing at once. Candidate: single-instance lock for the elevated install path.
- **Rescan after install (observed).** `CM_Reenumerate_DevNode` and `CM_Get_DevNode_Status` are imported
  by the scanner and fault-fix modules. Equivalent for us: `pnputil /scan-devices` then re-read the
  problem code (`install.rs` already verifies).
- **Restore point (observed, string only).** `CreateRestorePoint` is a string, so it is loaded at run
  time. Order versus the install is not recovered. Driverix's order (restore point first, abort on
  failure) is already stricter by design.
- **Signature (observed).** The scanner loads `WINTRUST.DLL`/`WinVerifyTrust` dynamically and has a
  "not found" fallback; INF rows carry a WHQL flag and a `CatalogFile`. Nothing new for us: Windows
  Update packages are already Microsoft-signed.
- **Download is a separate helper (observed).** A queue is handed to `DBDownloader.exe` through
  `CreateProcessW`. Not applicable: Windows Update downloads for us.
- **Reboot (observed, decompiled).** One helper enables `SeShutdownPrivilege` and calls `ExitWindowsEx`
  for log off, reboot, shutdown, forced variants, or `SetSuspendState`. Only used when the user opts in.
  Candidate for the "auto reboot" option: act only when `RebootRequired` is true, never forced.
- **Ignore list (observed, strings).** The scanner logs `IgnoreByDpInst` for installed and matched
  hardware IDs: ignore keys are per device and per update, applied at scan time (inferred).
- **Not recovered:** how it ranks candidates against the installed driver, and anything about its
  cloud database. Both would need endpoints or data we are not using.

### 8.2 How it discovers drivers (static, `Scanner.dll` strings + local config)

Deliberately high level: server addresses, request/response field names and the obfuscated local
database file are left out of this repo. They describe a private service and we do not use it.

- **Discovery is server-side (observed).** The scanner enumerates devices locally (SetupAPI,
  `CM_Get_DevNode_Status`), builds a request from hardware and compatible IDs plus the installed
  version and date, sends it over its own HTTP client, and receives per device: the newest candidate,
  file name/size/hash, download URLs, a WHQL flag and a flags field. The match is a curated
  hardware-ID catalog on the vendor's side, which is why the UI says drivers pass "strict review rules".
  The equivalent open sources are in section 3 (Windows Update, Catalog, SDI-O DriverPacks).
- **The client still decides locally (observed, from log messages).** After the answer it skips a
  device when: local version >= offered version; local date is later than the offered one; the
  installed row has a 1970 date and is not Microsoft's; the offer targets another OS; the user ignored
  that hardware ID or key; or the helper installer already handled it. A flag can mark an offer
  "rare" (shown or hidden) or "deferred" (hold back a newly released driver). NVIDIA PhysX is dropped
  when no NVIDIA device exists.
- **Request hygiene (observed, `RqstFilter.ini`).** IDs starting with junk characters or digits are
  dropped before sending, with a small allow list of prefixes. OEM names from SMBIOS are normalised
  through a map (`Vendor.ini`) so OEM-specific drivers can be offered.
- **Caching and cost (observed).** Three JSON stores: last device snapshot, last server answer with an
  expiry, and a delta of devices whose core info changed since the snapshot, so repeat scans resend
  only what changed. A database version is compared to invalidate the cache.
- **Priority (observed, `InstConf.ini`).** PCI and HDAUDIO devices are marked important. A list of
  setup result codes is treated as non-fatal after install. We should not copy that: `install.rs`
  counts success only when the device reports the new version.

### 8.3 Where its drivers come from (static, strings of the downloader modules)

- **Two kinds of source (observed from field and module names).** Packages hosted by the vendor's own
  download servers in a repackaged archive format, and "external links" that point at the original
  hardware maker's download with its own hash and an archive-type hint. Which one a driver gets is
  decided server-side.
- **Provenance (inferred, not provable offline).** Nothing in the binaries says how the catalog is fed.
  The WHQL flag and the "review rules" wording suggest hand-curated or crawled maker packages plus
  Microsoft-signed ones. Treat the catalog as unaudited.
- **Transport and integrity (observed).** Plain HTTP, with an MD5 check after download; SHA-1/SHA-256
  and a fingerprint helper exist in the same module, but the file-check messages name MD5. MD5 over HTTP
  is not tamper-proof; what protects the user is the driver signature check done by Windows at install.
  Driverix should never rely on hash-over-HTTP; it gets Microsoft-signed packages from Windows Update.
- **Telemetry (observed).** The modules contain endpoints for usage statistics and for per-install and
  per-download events. A design rule we already have ("no telemetry") stays a differentiator.
- **Error-code help (observed).** `ErrCodeSpec\<language>.ini` is the Device Manager error text, one
  section per code. It is Microsoft's own help text, so we write our own pt-BR table from Microsoft
  Learn instead of copying it.

Takeaways for Driverix (none needs a new source):
1. **Persist the last scan** (result + timestamp) and show its age; a cold Windows Update search takes
   about a minute. A cached result must never feed `guard::select_offers`: installs still need a fresh
   in-memory scan.
2. **Date sanity**: `matching.rs` decides on version only. Treat a future date or a 1970 date on a
   non-Microsoft row as `Unknown` instead of trusting it.
3. **"Deferred" offers**: optionally hold back a driver published in the last N days.
4. **Ignore keys** per hardware ID and per update (already in the section 8 list).
