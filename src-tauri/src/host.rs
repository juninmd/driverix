//! Every side effect on the machine goes through `Host`, so the install/rollback flow can be
//! tested with a fake and the real implementation stays a thin layer over PowerShell and pnputil.

use crate::error::{Error, Result};
use crate::ps::{run_process, run_script, scripts};
use crate::{inventory, model::Device, sys};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateOutcome {
    pub update_id: String,
    /// WUA OperationResultCode: 2 succeeded, 3 succeeded with errors, 4 failed, 5 aborted; 0 not attempted.
    pub result_code: i32,
    pub hresult: i32,
    pub reboot_required: bool,
    pub note: String,
}

impl UpdateOutcome {
    pub fn ok(&self) -> bool {
        self.result_code == 2
    }
}

pub trait Host {
    /// Minutes since the newest restore point, `None` when there is none.
    fn restore_point_age_minutes(&self) -> Result<Option<u32>>;
    fn create_restore_point(&self) -> Result<()>;
    /// Creates `dir` (and its parent) owned by Administrators, writable only by SYSTEM and admins.
    fn secure_dir(&self, dir: &Path) -> Result<()>;
    fn export_driver(&self, oem_inf: &str, dir: &Path) -> Result<()>;
    fn install_updates(&self, update_ids: &[String]) -> Result<Vec<UpdateOutcome>>;
    fn inventory(&self) -> Result<Vec<Device>>;
    fn delete_driver(&self, oem_inf: &str) -> Result<()>;
    /// `inf_glob` is a path whose last component may be a `*.inf` wildcard.
    fn add_driver(&self, inf_glob: &Path) -> Result<()>;
}

pub struct WindowsHost<'a> {
    pub cancel: &'a AtomicBool,
}

const LONG: Duration = Duration::from_secs(30 * 60);
const SHORT: Duration = Duration::from_secs(120);

impl WindowsHost<'_> {
    fn tool(&self, what: &'static str, exe: &str, args: &[&std::ffi::OsStr]) -> Result<()> {
        let mut cmd = Command::new(sys::system32(exe));
        cmd.args(args);
        let done = run_process(what, cmd, SHORT * 5, self.cancel)?;
        // 3010 is "success, restart required".
        match done.code {
            Some(0) | Some(3010) => Ok(()),
            _ => Err(Error::Command {
                what,
                detail: format!("{} {}", done.stdout.trim(), done.stderr.trim())
                    .trim()
                    .to_string(),
            }),
        }
    }

    fn pnputil(&self, what: &'static str, args: &[&std::ffi::OsStr]) -> Result<()> {
        self.tool(what, "pnputil.exe", args)
    }

    fn lock_down(&self, dir: &Path) -> Result<()> {
        if sys::is_reparse_point(dir) {
            return Err(Error::Denied(format!(
                "{} é um link/junção; recusando usar como pasta de backup",
                dir.display()
            )));
        }
        // Owner first: whoever created the folder keeps implicit rights until ownership moves.
        // /T applies to everything already inside: a backup folder a standard user planted before
        // the first admin run would otherwise keep its attacker-set ACL, and its owner can rewrite it.
        self.tool(
            "ajuste de dono da pasta de backup",
            "icacls.exe",
            &[
                dir.as_os_str(),
                "/setowner".as_ref(),
                "*S-1-5-32-544".as_ref(),
                "/T".as_ref(),
            ],
        )?;
        self.tool(
            "permissões da pasta de backup",
            "icacls.exe",
            &[
                dir.as_os_str(),
                "/T".as_ref(),
                "/inheritance:r".as_ref(),
                "/grant:r".as_ref(),
                "*S-1-5-18:(OI)(CI)F".as_ref(),
                "/grant:r".as_ref(),
                "*S-1-5-32-544:(OI)(CI)F".as_ref(),
                "/grant:r".as_ref(),
                "*S-1-5-32-545:(OI)(CI)RX".as_ref(),
            ],
        )
    }
}

impl Host for WindowsHost<'_> {
    fn restore_point_age_minutes(&self) -> Result<Option<u32>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Age {
            age_minutes: Option<u32>,
        }
        let out = run_script(
            "consulta de pontos de restauração",
            scripts::RESTORE_AGE,
            None,
            SHORT,
            self.cancel,
        )?;
        Ok(serde_json::from_str::<Age>(out.trim())?.age_minutes)
    }

    fn create_restore_point(&self) -> Result<()> {
        run_script(
            "ponto de restauração",
            scripts::RESTORE_POINT,
            None,
            SHORT * 3,
            self.cancel,
        )
        .map(|_| ())
    }

    fn secure_dir(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        if let Some(parent) = dir.parent() {
            self.lock_down(parent)?;
        }
        self.lock_down(dir)
    }

    fn export_driver(&self, oem_inf: &str, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        self.pnputil(
            "exportação do driver",
            &["/export-driver".as_ref(), oem_inf.as_ref(), dir.as_os_str()],
        )
    }

    fn install_updates(&self, update_ids: &[String]) -> Result<Vec<UpdateOutcome>> {
        let input = serde_json::json!({ "updateIds": update_ids }).to_string();
        let out = run_script(
            "instalação via Windows Update",
            scripts::INSTALL_UPDATES,
            Some(&input),
            LONG,
            self.cancel,
        )?;
        Ok(serde_json::from_str(out.trim())?)
    }

    fn inventory(&self) -> Result<Vec<Device>> {
        inventory::collect(self.cancel)
    }

    fn delete_driver(&self, oem_inf: &str) -> Result<()> {
        self.pnputil(
            "remoção do driver",
            &[
                "/delete-driver".as_ref(),
                oem_inf.as_ref(),
                "/uninstall".as_ref(),
            ],
        )
    }

    fn add_driver(&self, inf_glob: &Path) -> Result<()> {
        self.pnputil(
            "reinstalação do driver",
            &[
                "/add-driver".as_ref(),
                inf_glob.as_os_str(),
                "/install".as_ref(),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_result_code_two_counts_as_success() {
        let mk = |c| UpdateOutcome {
            result_code: c,
            ..Default::default()
        };
        assert!(mk(2).ok());
        assert!(!mk(3).ok() && !mk(4).ok() && !mk(5).ok() && !mk(0).ok());
    }

    #[test]
    fn outcome_parses_the_script_shape() {
        let json =
            r#"[{"updateId":"x","resultCode":2,"hresult":0,"rebootRequired":true,"note":""}]"#;
        let v: Vec<UpdateOutcome> = serde_json::from_str(json).unwrap();
        assert!(v[0].ok() && v[0].reboot_required);
    }
}
