use crate::error::{Error, Result};
use crate::guard::{is_backup_id, is_oem_inf};
use crate::model::Device;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// Every device that used `old_inf` before the update; rollback re-reads what they use now.
    pub instance_ids: Vec<String>,
    pub device_name: String,
    /// Package the devices used before the update (`oem7.inf`).
    pub old_inf: String,
    pub old_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub id: String,
    pub created_unix: u64,
    /// `recent`, `created` or `skipped`.
    pub restore_point: String,
    pub entries: Vec<Entry>,
}

pub fn dir_for(root: &Path, id: &str) -> Result<PathBuf> {
    if !is_backup_id(id) {
        return Err(Error::Denied("Identificador de backup inválido".into()));
    }
    Ok(root.join(id))
}

/// Folder inside a backup that holds one exported package.
pub fn package_dir(backup: &Path, inf: &str) -> PathBuf {
    backup.join(inf.trim_end_matches(".inf").trim_end_matches(".INF"))
}

/// One entry per third-party package in use. Inbox drivers are skipped: they are not in the store
/// as `oem*.inf`, so the restore point is their only way back.
pub fn plan_entries(devices: &[Device], target_ids: &[String]) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    for d in devices
        .iter()
        .filter(|d| target_ids.contains(&d.instance_id) && is_oem_inf(&d.inf))
    {
        match entries
            .iter_mut()
            .find(|e| e.old_inf.eq_ignore_ascii_case(&d.inf))
        {
            Some(e) => e.instance_ids.push(d.instance_id.clone()),
            None => entries.push(Entry {
                instance_ids: vec![d.instance_id.clone()],
                device_name: d.name.clone(),
                old_inf: d.inf.clone(),
                old_version: d.version.clone(),
            }),
        }
    }
    entries
}

pub fn write_manifest(root: &Path, m: &Manifest) -> Result<()> {
    let dir = dir_for(root, &m.id)?;
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("manifest.json"), serde_json::to_vec_pretty(m)?)?;
    Ok(())
}

pub fn read_manifest(root: &Path, id: &str) -> Result<Manifest> {
    let raw = std::fs::read(dir_for(root, id)?.join("manifest.json"))?;
    let m: Manifest = serde_json::from_slice(&raw)?;
    if m.id != id {
        return Err(Error::Parse("manifesto não corresponde ao backup".into()));
    }
    // The manifest sits in a folder an attacker may once have written to; only plain `oemN.inf`
    // names may reach a path join or pnputil.
    if let Some(bad) = m.entries.iter().find(|e| !is_oem_inf(&e.old_inf)) {
        return Err(Error::Parse(format!(
            "nome de INF inválido no manifesto: {}",
            bad.old_inf
        )));
    }
    Ok(m)
}

/// Newest first; unreadable folders are skipped so one corrupt backup does not hide the others.
pub fn list(root: &Path) -> Vec<Manifest> {
    let Ok(rd) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut all: Vec<Manifest> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|id| is_backup_id(id))
        .filter_map(|id| read_manifest(root, &id).ok())
        .collect();
    all.sort_by_key(|m| std::cmp::Reverse(m.created_unix));
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(id: &str, inf: &str) -> Device {
        Device {
            instance_id: id.into(),
            inf: inf.into(),
            version: "1.0".into(),
            ..Default::default()
        }
    }

    fn tmp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("driverix-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn plans_one_entry_per_third_party_package_with_every_device() {
        let devices = [
            dev("a", "oem7.inf"),
            dev("b", "oem7.inf"),
            dev("c", "netrasa.inf"),
            dev("d", "oem9.inf"),
        ];
        let ids: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        let plan = plan_entries(&devices, &ids);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].old_inf, "oem7.inf");
        assert_eq!(plan[0].instance_ids, vec!["a", "b"]);
    }

    #[test]
    fn a_tampered_manifest_cannot_smuggle_a_path_through_the_inf_name() {
        let root = tmp("tamper");
        for evil in [
            r"C:\attacker\pkg",
            r"..\..\x",
            "oem7.inf\\..\\..\\y",
            "notoem.inf",
        ] {
            let m = Manifest {
                id: "7".into(),
                created_unix: 1,
                restore_point: "x".into(),
                entries: vec![Entry {
                    instance_ids: vec![],
                    device_name: String::new(),
                    old_inf: evil.into(),
                    old_version: String::new(),
                }],
            };
            write_manifest(&root, &m).unwrap();
            assert!(read_manifest(&root, "7").is_err(), "accepted {evil}");
            assert!(list(&root).is_empty());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manifest_round_trips_and_lists_newest_first() {
        let root = tmp("rt");
        for (id, t) in [("100", 100), ("200", 200)] {
            let m = Manifest {
                id: id.into(),
                created_unix: t,
                restore_point: "created".into(),
                entries: vec![],
            };
            write_manifest(&root, &m).unwrap();
        }
        let all = list(&root);
        assert_eq!(
            all.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            ["200", "100"]
        );
        assert_eq!(read_manifest(&root, "100").unwrap().created_unix, 100);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_ids_that_escape_the_backup_root() {
        let root = tmp("esc");
        assert!(dir_for(&root, "..").is_err());
        assert!(read_manifest(&root, "../x").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_manifest_claiming_another_id_is_rejected() {
        let root = tmp("id");
        let m = Manifest {
            id: "5".into(),
            created_unix: 1,
            restore_point: "x".into(),
            entries: vec![],
        };
        write_manifest(&root, &m).unwrap();
        std::fs::rename(root.join("5"), root.join("6")).unwrap();
        assert!(read_manifest(&root, "6").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_dir_drops_the_extension() {
        assert_eq!(
            package_dir(Path::new("b"), "oem7.inf"),
            Path::new("b").join("oem7")
        );
    }
}
