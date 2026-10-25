//! The install flow, in the order that keeps every step undoable:
//! restore point -> export current packages -> install -> verify. Nothing is installed
//! unless the restore point and every backup export succeeded first.

use crate::backup::{self, Manifest};
use crate::error::{Error, Result};
use crate::guard::is_oem_inf;
use crate::host::{Host, UpdateOutcome};
use crate::matching::decide;
use crate::model::{Decision, Device, Offer};
use serde::Serialize;
use std::path::Path;

/// Windows allows one restore point per 24 h through its API; a newer one already covers us.
const RESTORE_POINT_REUSE_MINUTES: u32 = 24 * 60;

#[derive(Debug, Default, Clone, Copy)]
pub struct InstallOptions {
    /// Continue even though no restore point could be made (System Restore off).
    pub allow_no_restore_point: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReport {
    pub backup_id: String,
    /// `recent`, `created` or `skipped`.
    pub restore_point: String,
    pub backed_up: usize,
    pub outcomes: Vec<UpdateOutcome>,
    pub reboot_required: bool,
    /// Devices of this batch that report a problem after the install.
    pub problem_devices: Vec<String>,
    /// False when the post-install inventory could not run, so `problem_devices` is not proof.
    pub verified: bool,
}

fn ensure_restore_point(host: &dyn Host, opts: &InstallOptions) -> Result<&'static str> {
    if let Ok(Some(age)) = host.restore_point_age_minutes() {
        if age < RESTORE_POINT_REUSE_MINUTES {
            return Ok("recent");
        }
    }
    match host.create_restore_point() {
        Ok(()) => Ok("created"),
        Err(_) if opts.allow_no_restore_point => Ok("skipped"),
        Err(e) => Err(Error::Denied(format!(
            "Não foi possível criar o ponto de restauração ({e}). Ative a Proteção do Sistema ou marque a opção para continuar sem ele."
        ))),
    }
}

/// The scan can be minutes old: a driver installed since then may make the offer a downgrade.
fn still_newer(offer: &Offer, before: &[Device]) -> bool {
    !offer.device_ids.is_empty()
        && offer.device_ids.iter().all(|id| {
            before
                .iter()
                .find(|d| &d.instance_id == id)
                .is_some_and(|d| decide(d, &offer.candidate).0 == Decision::Newer)
        })
}

pub fn install(
    host: &dyn Host,
    offers: &[Offer],
    root: &Path,
    now_unix: u64,
    opts: &InstallOptions,
) -> Result<InstallReport> {
    let before = host.inventory()?;
    if let Some(stale) = offers.iter().find(|o| !still_newer(o, &before)) {
        return Err(Error::Denied(format!(
            "'{}' não é mais novo que o driver instalado; busque de novo",
            stale.candidate.title
        )));
    }
    let target_ids: Vec<String> = offers
        .iter()
        .flat_map(|o| o.device_ids.iter().cloned())
        .collect();

    let restore_point = ensure_restore_point(host, opts)?;

    // A backup id is a timestamp; refuse to reuse a folder rather than overwrite a manifest.
    let id = now_unix.to_string();
    let dir = backup::dir_for(root, &id)?;
    host.secure_dir(root)?;
    if dir.exists() {
        return Err(Error::Denied(
            "Já existe um backup com este identificador".into(),
        ));
    }
    let entries = backup::plan_entries(&before, &target_ids);
    for e in &entries {
        let pkg = backup::package_dir(&dir, &e.old_inf);
        host.export_driver(&e.old_inf, &pkg)?;
        let has_files = std::fs::read_dir(&pkg)
            .map(|mut r| r.next().is_some())
            .unwrap_or(false);
        if !has_files {
            return Err(Error::Denied(format!(
                "Backup de {} ficou vazio; nada foi instalado",
                e.old_inf
            )));
        }
    }
    // Written before the install so an interrupted run still leaves a usable record.
    backup::write_manifest(
        root,
        &Manifest {
            id: id.clone(),
            created_unix: now_unix,
            restore_point: restore_point.into(),
            entries: entries.clone(),
        },
    )?;

    let update_ids: Vec<String> = offers
        .iter()
        .map(|o| o.candidate.update_id.clone())
        .collect();
    let outcomes = host.install_updates(&update_ids)?;

    let (problem_devices, verified) = match host.inventory() {
        Ok(after) => (
            after
                .iter()
                .filter(|d| target_ids.contains(&d.instance_id) && d.problem_code != 0)
                .map(|d| d.name.clone())
                .collect(),
            true,
        ),
        Err(_) => (Vec::new(), false),
    };

    Ok(InstallReport {
        backup_id: id,
        restore_point: restore_point.into(),
        backed_up: entries.len(),
        reboot_required: outcomes.iter().any(|o| o.reboot_required),
        outcomes,
        problem_devices,
        verified,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackReport {
    pub restored: usize,
    pub errors: Vec<String>,
}

/// Removes whatever third-party package the affected devices use now, then puts the exported one
/// back. The order matters: pnputil will not install a lower-ranked package over a better one that
/// is still present. The package to delete is read from a fresh inventory, never from the
/// manifest, so it is right even if the install was interrupted, `oemN` numbers were reused, or
/// a previous rollback got half way. Success is judged by the driver version the devices report.
pub fn rollback(host: &dyn Host, root: &Path, backup_id: &str) -> Result<RollbackReport> {
    // A backup may predate every install in this session; lock the tree before trusting it.
    host.secure_dir(root)?;
    let manifest = backup::read_manifest(root, backup_id)?;
    let dir = backup::dir_for(root, backup_id)?;
    let current = host.inventory()?;
    let mut report = RollbackReport {
        restored: 0,
        errors: Vec::new(),
    };
    let mut attempted = Vec::new();
    for e in &manifest.entries {
        let pkg = backup::package_dir(&dir, &e.old_inf);
        if !pkg.is_dir() {
            report.errors.push(format!(
                "{}: arquivos do backup não encontrados; este item não pode ser restaurado",
                e.device_name
            ));
            continue;
        }
        let mut stale: Vec<&str> = current
            .iter()
            .filter(|d| e.instance_ids.contains(&d.instance_id))
            .map(|d| d.inf.as_str())
            .filter(|inf| is_oem_inf(inf) && !inf.eq_ignore_ascii_case(&e.old_inf))
            .collect();
        stale.sort_unstable_by_key(|s| s.to_ascii_lowercase());
        stale.dedup_by_key(|s| s.to_ascii_lowercase());
        if let Some(err) = stale.iter().find_map(|inf| host.delete_driver(inf).err()) {
            report.errors.push(format!("{}: {err}", e.device_name));
            continue;
        }
        match host.add_driver(&pkg.join("*.inf")) {
            Ok(()) => attempted.push(e),
            Err(err) => report.errors.push(format!("{}: {err}", e.device_name)),
        }
    }
    if attempted.is_empty() {
        return Ok(report);
    }
    match host.inventory() {
        Ok(after) => {
            for e in attempted {
                let back = after
                    .iter()
                    .filter(|d| e.instance_ids.contains(&d.instance_id))
                    .all(|d| d.version == e.old_version);
                if back {
                    report.restored += 1;
                } else {
                    report.errors.push(format!(
                        "{}: o dispositivo não voltou à versão {}",
                        e.device_name, e.old_version
                    ));
                }
            }
        }
        Err(err) => report.errors.push(format!(
            "Não foi possível verificar o resultado da restauração: {err}"
        )),
    }
    Ok(report)
}

#[cfg(test)]
#[path = "install_tests.rs"]
mod tests;
