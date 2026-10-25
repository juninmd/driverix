use super::*;
use crate::model::{Candidate, Decision, Device};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;

const UPD: &str = "0b9a4d52-3f3c-4a49-9d8e-2f6f4d1c7a10";

#[derive(Default)]
struct Fake {
    log: RefCell<Vec<String>>,
    age: Option<u32>,
    restore_fails: bool,
    secure_fails: bool,
    export_fails: bool,
    export_empty: bool,
    install_fails: bool,
    add_fails: bool,
    delete_fails: bool,
    /// Successive `inventory()` results; `None` makes that call fail.
    inventories: RefCell<VecDeque<Option<Vec<Device>>>>,
}

impl Fake {
    fn calls(&self) -> Vec<String> {
        self.log.borrow().clone()
    }
    fn push(&self, s: impl Into<String>) {
        self.log.borrow_mut().push(s.into());
    }
}

impl Host for Fake {
    fn restore_point_age_minutes(&self) -> Result<Option<u32>> {
        Ok(self.age)
    }
    fn create_restore_point(&self) -> Result<()> {
        self.push("restore_point");
        if self.restore_fails {
            Err(Error::Timeout("rp"))
        } else {
            Ok(())
        }
    }
    fn secure_dir(&self, dir: &Path) -> Result<()> {
        self.push("secure");
        if self.secure_fails {
            return Err(Error::Timeout("acl"));
        }
        std::fs::create_dir_all(dir)?;
        Ok(())
    }
    fn export_driver(&self, oem: &str, dir: &Path) -> Result<()> {
        self.push(format!("export {oem}"));
        if self.export_fails {
            return Err(Error::Timeout("export"));
        }
        std::fs::create_dir_all(dir)?;
        if !self.export_empty {
            std::fs::write(dir.join("a.inf"), b"x")?;
        }
        Ok(())
    }
    fn install_updates(&self, ids: &[String]) -> Result<Vec<UpdateOutcome>> {
        self.push("install");
        if self.install_fails {
            return Err(Error::Timeout("install"));
        }
        Ok(ids
            .iter()
            .map(|i| UpdateOutcome {
                update_id: i.clone(),
                result_code: 2,
                reboot_required: true,
                ..Default::default()
            })
            .collect())
    }
    fn inventory(&self) -> Result<Vec<Device>> {
        match self.inventories.borrow_mut().pop_front() {
            Some(Some(d)) => Ok(d),
            _ => Err(Error::Timeout("inventory")),
        }
    }
    fn delete_driver(&self, oem: &str) -> Result<()> {
        self.push(format!("delete {oem}"));
        if self.delete_fails {
            Err(Error::Timeout("delete"))
        } else {
            Ok(())
        }
    }
    fn add_driver(&self, glob: &Path) -> Result<()> {
        self.push(format!(
            "add {}",
            glob.file_name().unwrap().to_string_lossy()
        ));
        if self.add_fails {
            Err(Error::Timeout("add"))
        } else {
            Ok(())
        }
    }
}

fn dev_at(id: &str, inf: &str, version: &str, problem: u32) -> Device {
    Device {
        instance_id: id.into(),
        name: "NIC".into(),
        inf: inf.into(),
        version: version.into(),
        problem_code: problem,
        ..Default::default()
    }
}

fn dev(inf: &str, problem: u32) -> Device {
    dev_at(
        "PCI\\A",
        inf,
        if inf == "oem7.inf" { "1.0" } else { "2.0" },
        problem,
    )
}

fn offer() -> Offer {
    Offer {
        candidate: Candidate {
            update_id: UPD.into(),
            version: Some("2.0".into()),
            ..Default::default()
        },
        device_ids: vec!["PCI\\A".into()],
        decision: Decision::Newer,
        reason: String::new(),
        fixes_problem: false,
        via_compatible: false,
    }
}

fn with_inventories(invs: Vec<Option<Vec<Device>>>) -> Fake {
    Fake {
        inventories: RefCell::new(invs.into()),
        ..Default::default()
    }
}

/// before: oem7 (1.0); after install: oem9 (2.0).
fn fake() -> Fake {
    with_inventories(vec![
        Some(vec![dev("oem7.inf", 0)]),
        Some(vec![dev("oem9.inf", 0)]),
    ])
}

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("driverix-install-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn installed(tag: &str) -> PathBuf {
    let r = root(tag);
    install(&fake(), &[offer()], &r, 10, &InstallOptions::default()).unwrap();
    r
}

#[test]
fn no_restore_point_means_nothing_is_touched() {
    let r = root("norp");
    let f = Fake {
        restore_fails: true,
        ..fake()
    };
    let err = install(&f, &[offer()], &r, 10, &InstallOptions::default()).unwrap_err();
    assert!(matches!(err, Error::Denied(_)));
    assert_eq!(f.calls(), ["restore_point"]);
    assert!(backup::list(&r).is_empty());
}

#[test]
fn the_user_can_opt_out_of_the_restore_point() {
    let r = root("optout");
    let f = Fake {
        restore_fails: true,
        ..fake()
    };
    let rep = install(
        &f,
        &[offer()],
        &r,
        10,
        &InstallOptions {
            allow_no_restore_point: true,
        },
    )
    .unwrap();
    assert_eq!(rep.restore_point, "skipped");
    assert!(f.calls().contains(&"install".to_string()));
}

#[test]
fn a_recent_restore_point_is_reused() {
    let r = root("recent");
    let f = Fake {
        age: Some(60),
        ..fake()
    };
    let rep = install(&f, &[offer()], &r, 10, &InstallOptions::default()).unwrap();
    assert_eq!(rep.restore_point, "recent");
    assert!(!f.calls().contains(&"restore_point".to_string()));
}

#[test]
fn a_stale_restore_point_triggers_a_new_one() {
    let r = root("stale");
    let f = Fake {
        age: Some(5000),
        ..fake()
    };
    assert_eq!(
        install(&f, &[offer()], &r, 10, &InstallOptions::default())
            .unwrap()
            .restore_point,
        "created"
    );
}

#[test]
fn steps_run_in_the_undoable_order() {
    let r = root("order");
    let f = fake();
    install(&f, &[offer()], &r, 10, &InstallOptions::default()).unwrap();
    assert_eq!(
        f.calls(),
        ["restore_point", "secure", "export oem7.inf", "install"]
    );
}

#[test]
fn an_unsecured_backup_folder_stops_everything() {
    let r = root("insecure");
    let f = Fake {
        secure_fails: true,
        ..fake()
    };
    assert!(install(&f, &[offer()], &r, 10, &InstallOptions::default()).is_err());
    assert!(!f
        .calls()
        .iter()
        .any(|c| c.starts_with("export") || c == "install"));
}

#[test]
fn a_failed_export_stops_before_install() {
    let r = root("exportfail");
    let f = Fake {
        export_fails: true,
        ..fake()
    };
    assert!(install(&f, &[offer()], &r, 10, &InstallOptions::default()).is_err());
    assert!(!f.calls().contains(&"install".to_string()));
}

#[test]
fn an_empty_export_stops_before_install() {
    let r = root("exportempty");
    let f = Fake {
        export_empty: true,
        ..fake()
    };
    assert!(install(&f, &[offer()], &r, 10, &InstallOptions::default()).is_err());
    assert!(!f.calls().contains(&"install".to_string()));
}

#[test]
fn an_existing_backup_id_is_never_overwritten() {
    let r = installed("collide");
    let f = fake();
    assert!(install(&f, &[offer()], &r, 10, &InstallOptions::default()).is_err());
    assert!(!f.calls().contains(&"install".to_string()));
}

#[test]
fn a_driver_installed_since_the_scan_blocks_the_install_before_any_change() {
    let r = root("stale");
    // Already at 2.0 when the install starts: the offer is no longer newer.
    let f = with_inventories(vec![Some(vec![dev("oem9.inf", 0)])]);
    let err = install(&f, &[offer()], &r, 10, &InstallOptions::default()).unwrap_err();
    assert!(matches!(err, Error::Denied(_)), "{err:?}");
    assert!(
        f.calls().is_empty(),
        "no restore point, export or install may run: {:?}",
        f.calls()
    );
}

#[test]
fn reports_reboot_need_and_a_verified_clean_result() {
    let r = root("ok");
    let rep = install(&fake(), &[offer()], &r, 10, &InstallOptions::default()).unwrap();
    assert!(rep.reboot_required && rep.verified && rep.problem_devices.is_empty());
    assert_eq!(rep.backed_up, 1);
}

#[test]
fn flags_devices_that_break_after_the_install() {
    let r = root("broken");
    let f = with_inventories(vec![
        Some(vec![dev("oem7.inf", 0)]),
        Some(vec![dev("oem9.inf", 43)]),
    ]);
    let rep = install(&f, &[offer()], &r, 10, &InstallOptions::default()).unwrap();
    assert_eq!(rep.problem_devices, vec!["NIC"]);
}

#[test]
fn an_unverifiable_install_is_reported_as_such() {
    let r = root("unverified");
    let f = with_inventories(vec![Some(vec![dev("oem7.inf", 0)]), None]);
    assert!(
        !install(&f, &[offer()], &r, 10, &InstallOptions::default())
            .unwrap()
            .verified
    );
}

#[test]
fn the_backup_survives_a_failed_install_so_it_can_be_rolled_back() {
    let r = root("installfail");
    let f = Fake {
        install_fails: true,
        ..fake()
    };
    let rep = install(&f, &[offer()], &r, 10, &InstallOptions::default()).unwrap();
    assert!(rep.install_error.is_some());
    assert!(rep.outcomes.is_empty());
    // The devices are still checked after the failure, so the report can say what state they are in.
    assert!(rep.verified);
    assert_eq!(backup::list(&r).len(), 1);
}

#[test]
fn devices_sharing_a_package_are_all_recorded_in_one_entry() {
    let r = root("shared");
    let mut o = offer();
    o.device_ids = vec!["PCI\\A".into(), "PCI\\B".into()];
    let f = with_inventories(vec![
        Some(vec![
            dev_at("PCI\\A", "oem7.inf", "1.0", 0),
            dev_at("PCI\\B", "oem7.inf", "1.0", 0),
        ]),
        Some(vec![]),
    ]);
    let rep = install(&f, &[o], &r, 10, &InstallOptions::default()).unwrap();
    assert_eq!(rep.backed_up, 1);
    assert_eq!(
        backup::read_manifest(&r, "10").unwrap().entries[0]
            .instance_ids
            .len(),
        2
    );
}

#[test]
fn rollback_deletes_what_the_device_uses_now_then_restores_and_verifies() {
    let r = installed("rollback");
    let f = with_inventories(vec![
        Some(vec![dev("oem9.inf", 0)]),
        Some(vec![dev("oem3.inf", 0).with_version("1.0")]),
    ]);
    let rep = rollback(&f, &r, "10").unwrap();
    assert_eq!((rep.restored, rep.errors.len()), (1, 0));
    assert_eq!(f.calls(), ["delete oem9.inf", "add *.inf"]);
}

#[test]
fn rollback_works_even_if_the_install_never_recorded_the_new_package() {
    // Install interrupted: the manifest holds only the old package; the device already runs oem9.
    let r = root("interrupted");
    let f = Fake {
        install_fails: true,
        ..fake()
    };
    let _ = install(&f, &[offer()], &r, 10, &InstallOptions::default());
    let g = with_inventories(vec![
        Some(vec![dev("oem9.inf", 0)]),
        Some(vec![dev("oem3.inf", 0).with_version("1.0")]),
    ]);
    let rep = rollback(&g, &r, "10").unwrap();
    assert_eq!(rep.restored, 1);
    assert_eq!(g.calls(), ["delete oem9.inf", "add *.inf"]);
}

#[test]
fn rollback_can_be_retried_after_the_delete_already_happened() {
    let r = installed("retry");
    // The device has no third-party package now: the earlier delete worked, the add failed.
    let f = with_inventories(vec![
        Some(vec![dev_at("PCI\\A", "", "", 28)]),
        Some(vec![dev("oem3.inf", 0).with_version("1.0")]),
    ]);
    let rep = rollback(&f, &r, "10").unwrap();
    assert_eq!(rep.restored, 1);
    assert_eq!(f.calls(), ["add *.inf"]);
}

#[test]
fn rollback_never_deletes_a_package_the_device_does_not_use() {
    let r = installed("unrelated");
    // The device already runs the old package again: nothing to delete.
    let f = with_inventories(vec![
        Some(vec![dev("oem7.inf", 0)]),
        Some(vec![dev("oem7.inf", 0)]),
    ]);
    rollback(&f, &r, "10").unwrap();
    assert!(!f.calls().iter().any(|c| c.starts_with("delete")));
}

#[test]
fn rollback_does_not_claim_success_when_the_old_version_did_not_come_back() {
    let r = installed("notback");
    let f = with_inventories(vec![
        Some(vec![dev("oem9.inf", 0)]),
        Some(vec![dev("oem9.inf", 0)]),
    ]);
    let rep = rollback(&f, &r, "10").unwrap();
    assert_eq!(rep.restored, 0);
    assert_eq!(rep.errors.len(), 1);
}

#[test]
fn a_failed_delete_skips_the_add_and_reports() {
    let r = installed("delfail");
    let f = Fake {
        delete_fails: true,
        ..with_inventories(vec![Some(vec![dev("oem9.inf", 0)])])
    };
    let rep = rollback(&f, &r, "10").unwrap();
    assert_eq!((rep.restored, rep.errors.len()), (0, 1));
    assert_eq!(f.calls(), ["delete oem9.inf"]);
}

#[test]
fn rollback_with_a_missing_backup_folder_reports_instead_of_deleting() {
    let r = installed("rollback-missing");
    std::fs::remove_dir_all(r.join("10").join("oem7")).unwrap();
    let f = with_inventories(vec![Some(vec![dev("oem9.inf", 0)])]);
    let rep = rollback(&f, &r, "10").unwrap();
    assert_eq!((rep.restored, rep.errors.len()), (0, 1));
    assert!(f.calls().is_empty());
}

impl Device {
    fn with_version(mut self, v: &str) -> Self {
        self.version = v.into();
        self
    }
}
