# AGENTS.md

Driverix: Tauri 2 desktop driver updater for Windows. Rust backend (`src-tauri/`) inventories hardware, asks Windows Update for driver updates, backs up and installs them, and can roll back; React 19 + Tailwind v4 frontend (`src/`). UI copy is pt-BR; code, comments and commits are English. Sibling of [Faxina](https://github.com/juninmd/faxina): same stack and conventions.

## Commands

| Task | Command |
|---|---|
| Install | `bun install` |
| Run app | `bun tauri dev` (install/rollback need an elevated terminal) |
| Lint / format | `bun run lint` / `bun run format` (Biome) |
| Types / tests | `bun run typecheck` / `bun run test` |
| Rust gates | `cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` |
| Read-only live check | `cargo run --bin driverix-cli -- devices\|hardware\|scan [--json]` (never installs) |

`cargo` needs `../dist` (`tauri::generate_context!` embeds it): run `bun run build` once on a clean checkout.

## Layout

- `matching.rs` pure: version parsing/compare, candidate-vs-device decision, offers. `model.rs` serde shapes shared with `src/lib/api.ts` (keep in sync).
- `ps.rs` runs the fixed scripts in `scripts/*.ps1` (`-EncodedCommand`); caller data goes in the `DRIVERIX_INPUT` env var as JSON, never into the script text.
- `inventory.rs` / `updates.rs` parse script output (pure, tested); `guard.rs` validates every request; `host.rs` is the `Host` trait plus `WindowsHost` (PowerShell + `pnputil`); `install.rs` is the install/rollback flow, tested against a fake `Host`; `vendors.rs` offline PCI/USB vendor + class table (pure, tested; fills `Device.vendor`/`hint`); `hardware.rs` + `scripts/hardware.ps1` the AIDA64-style overview (script -> `HardwareInfo` -> labelled `Section`s shared by the UI and the text report; read-only); `backup.rs` manifests; `commands.rs` the only Tauri surface.

## Safety invariants (do not weaken)

- Windows Update (`Type='Driver'`) is the only update source. Vendor endpoints (NVIDIA, AMD, Intel) are undocumented or ToS-restricted; see `docs/RESEARCH.md` before adding one.
- The webview sends update IDs and backup IDs only. IDs are re-checked in `guard::select_offers` against the last scan (GUID shape, present, `Decision::Newer`, admin, batch cap). Never accept a path, URL or INF name from the webview.
- Order is restore point -> export current packages -> install -> verify. A failed restore point or an empty export aborts before anything is installed (`install.rs` tests pin this). Restore point policy: reuse one younger than 24 h, else create; skipping needs the explicit `allow_no_restore_point` opt-in.
- Downgrades never install: `Decision::Older`/`Same`/`Unknown` are not installable, and one downgrading device blocks a shared offer.
- Backups live in `%SystemDrive%\ProgramData\Driverix\backups`, locked down by `Host::secure_dir` (owner Administrators, write for SYSTEM/admins only, junctions refused before creating any directory) before the first export. Never move them to the user profile: rollback re-installs from there as admin. `read_manifest` accepts only `oemN.inf` names.
- Rollback reads the package to delete from a fresh inventory (never from the manifest), deletes it without `/force`, re-adds the exported one, and counts success only if the devices report the old version. pnputil will not install a lower-ranked package over a present better one, hence delete first.
- Install and rollback are not cancellable (they get a flag nobody sets); `cancel_operation` only stops a scan. `run_process` puts a timeout on every child, `pnputil` included.
- System tools run from `GetSystemDirectoryW` by absolute path, never via `PATH` or env vars.
- EULAs are never auto-accepted; an update with an unaccepted EULA is reported, not installed.
- A device with an empty driver row is only offered an update when Config Manager reports problem 28; otherwise it is `Unknown` (could be a WMI gap, and there would be no backup).
- `Win32_VideoController.AdapterRAM` is a 32-bit field capped at 4 GB; `hardware.ps1` reads `HardwareInformation.qwMemorySize` from the display class key instead. Serial numbers are never collected, so a copied report is safe to paste.
- Dates are UTC midnight stored in CIM as local time: convert with `ToUniversalTime()` or the day shifts.

## Gotchas

- PowerShell read from stdin ends a block at the first blank line and exits 0 with no output; that is why scripts travel as `-EncodedCommand`.
- The Windows Update search takes ~1 minute on a cold machine; it is cancellable by killing the child.
- `scripts/hardware.ps1` was written and its output parsing tested without a Windows machine; run `driverix-cli hardware` on a real one before a release.
- Install and rollback paths are covered by fake-`Host` unit tests only; they need a manual run on a real elevated machine before a release.

<!-- base-actions-main-policy -->
## GitHub Actions — padrão obrigatório (@main)

- Referencie as actions internas sempre pelo branch, nunca por SHA:
  - `uses: juninmd/base-actions/<action>@main`
  - `uses: juninmd/base-actions/.github/workflows/<arquivo>.yml@main`
- Não fixe `@<sha>` em `juninmd/base-actions`: a atualização é centralizada no
  `main` e passa a valer no próximo run.
- PRs cujos checks não iniciam por billing/spending limit são mesclados pelo bot central
  `juninmd/base-actions/.github/workflows/auto-merge-billing-blocked.yml`.
  Para impedir o merge automático de um PR, aplique a label `do-not-merge`.