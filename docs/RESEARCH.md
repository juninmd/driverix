# Research notes: how driver updaters work

Why Driverix is built the way it is. Everything here comes from public sources; no proprietary
binary (Driver Easy, CCleaner, Driver Booster, ...) was downloaded, run or decompiled, and their
EULAs forbid it. Where the vendors document nothing, this file says UNCONFIRMED instead of guessing.

Scope: public sources only (Microsoft Learn, vendor support pages, search snippets). No proprietary binary was downloaded or decompiled.
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
