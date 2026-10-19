//! The only Tauri surface. Every mutating command takes IDs, never paths, and re-validates them
//! against the last scan in `guard`.

use crate::backup::{self, Manifest};
use crate::error::Error;
use crate::guard;
use crate::hardware::{self, HardwareView};
use crate::host::WindowsHost;
use crate::install::{self, InstallOptions, InstallReport, RollbackReport};
use crate::model::{Device, Offer};
use crate::scan::{scan_all, ScanResult};
use crate::{inventory, ps, sys};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
pub struct AppState {
    offers: Mutex<Vec<Offer>>,
    cancel: Arc<AtomicBool>,
    busy: AtomicBool,
}

/// Held while a long operation runs so two of them never overlap (e.g. install during a scan).
struct BusyGuard<'a>(&'a AtomicBool);

impl<'a> BusyGuard<'a> {
    fn acquire(flag: &'a AtomicBool) -> Result<Self, String> {
        flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Self(flag))
            .map_err(|_| "Outra operação está em andamento".to_string())
    }
}

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn msg(e: Error) -> String {
    e.to_string()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    elevated: bool,
    backups_dir: String,
}

#[tauri::command]
pub fn get_status() -> Status {
    Status {
        elevated: sys::is_elevated(),
        backups_dir: sys::backups_root().to_string_lossy().into_owned(),
    }
}

#[tauri::command]
pub async fn load_devices() -> Result<Vec<Device>, String> {
    // Read-only and unguarded, so it gets its own flag instead of resetting a scan's cancel.
    tauri::async_runtime::spawn_blocking(move || {
        inventory::collect(&AtomicBool::new(false)).map_err(msg)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_hardware() -> Result<HardwareView, String> {
    // Read-only like `load_devices`, so it also gets its own cancel flag.
    tauri::async_runtime::spawn_blocking(move || {
        hardware::collect(&AtomicBool::new(false)).map_err(msg)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn scan_updates(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ScanResult, String> {
    let _busy = BusyGuard::acquire(&state.busy)?;
    let cancel = state.cancel.clone();
    cancel.store(false, Ordering::Relaxed);
    let result = tauri::async_runtime::spawn_blocking(move || {
        scan_all(&cancel, |p| {
            let _ = app.emit("scan-phase", p);
        })
        .map_err(msg)
    })
    .await
    .map_err(|e| e.to_string())??;
    *state
        .offers
        .lock()
        .map_err(|_| "estado corrompido".to_string())? = result.offers.clone();
    Ok(result)
}

#[tauri::command]
/// Cancels a running scan only; an install or rollback is never interruptible (see below).
pub fn cancel_operation(state: State<'_, AppState>) {
    state.cancel.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub async fn install_updates(
    update_ids: Vec<String>,
    allow_no_restore_point: bool,
    state: State<'_, AppState>,
) -> Result<InstallReport, String> {
    let _busy = BusyGuard::acquire(&state.busy)?;
    let offers = state
        .offers
        .lock()
        .map_err(|_| "estado corrompido".to_string())?
        .clone();
    let picked = guard::select_offers(&update_ids, &offers, sys::is_elevated()).map_err(msg)?;
    // Killing PowerShell mid-install would not stop Windows Update's own installer and would leave
    // a half-applied change, so install and rollback get a flag nobody can set.
    let result = tauri::async_runtime::spawn_blocking(move || {
        let never = AtomicBool::new(false);
        let host = WindowsHost { cancel: &never };
        install::install(
            &host,
            &picked,
            &sys::backups_root(),
            now_unix(),
            &InstallOptions {
                allow_no_restore_point,
            },
        )
        .map_err(msg)
    })
    .await
    .map_err(|e| e.to_string())?;
    // The installed state changed either way, so the scan the offers came from is stale.
    if let Ok(mut offers) = state.offers.lock() {
        offers.clear();
    }
    result
}

#[tauri::command]
pub fn list_backups() -> Vec<Manifest> {
    backup::list(&sys::backups_root())
}

#[tauri::command]
pub async fn rollback_backup(
    id: String,
    state: State<'_, AppState>,
) -> Result<RollbackReport, String> {
    if !sys::is_elevated() {
        return Err("Restaurar drivers exige executar como administrador".into());
    }
    let _busy = BusyGuard::acquire(&state.busy)?;
    tauri::async_runtime::spawn_blocking(move || {
        let never = AtomicBool::new(false);
        let host = WindowsHost { cancel: &never };
        install::rollback(&host, &sys::backups_root(), &id).map_err(msg)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Reopens this exe through UAC; the current instance exits only if the new one started.
#[tauri::command]
pub async fn relaunch_elevated(app: AppHandle) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let input = serde_json::json!({ "exe": exe }).to_string();
    let script =
        "Start-Process -FilePath (ConvertFrom-Json $env:DRIVERIX_INPUT).exe -Verb RunAs\n'{}'";
    let never = AtomicBool::new(false);
    tauri::async_runtime::spawn_blocking(move || {
        ps::run_script(
            "elevação",
            script,
            Some(&input),
            std::time::Duration::from_secs(120),
            &never,
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(msg)?;
    app.exit(0);
    Ok(())
}
