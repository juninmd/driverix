//! Process-level facts about the host: elevation and absolute paths to system locations, taken
//! from Win32 calls rather than environment variables, which a lower-privileged parent controls.

use std::path::PathBuf;

#[cfg(windows)]
pub fn system_dir() -> PathBuf {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;

    let mut buf = [0u16; 260];
    // SAFETY: `buf` is a valid writable buffer of the length passed.
    let n = unsafe { GetSystemDirectoryW(buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if n == 0 || n > buf.len() {
        return PathBuf::from(r"C:\Windows\System32");
    }
    PathBuf::from(std::ffi::OsString::from_wide(&buf[..n]))
}

#[cfg(not(windows))]
pub fn system_dir() -> PathBuf {
    PathBuf::from("/usr/bin")
}

pub fn system32(rel: &str) -> PathBuf {
    system_dir().join(rel)
}

/// Backups live under `<system drive>\ProgramData`, not the user profile: the install runs
/// elevated and later re-installs from here, so a user-writable folder would be a way to make an
/// admin process install an attacker's (validly signed, vulnerable) driver.
pub fn backups_root() -> PathBuf {
    let drive_root = system_dir()
        .ancestors()
        .last()
        .map(PathBuf::from)
        .unwrap_or_default();
    drive_root
        .join("ProgramData")
        .join("Driverix")
        .join("backups")
}

/// Junctions and symlinks: a pre-planted one would redirect elevated writes elsewhere.
/// Fails closed: a path that cannot be inspected is treated as a link, except when it does not exist.
pub fn is_reparse_point(path: &std::path::Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        match std::fs::symlink_metadata(path) {
            Ok(m) => m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0,
            Err(e) => e.kind() != std::io::ErrorKind::NotFound,
        }
    }
    #[cfg(not(windows))]
    {
        match std::fs::symlink_metadata(path) {
            Ok(m) => m.file_type().is_symlink(),
            Err(e) => e.kind() != std::io::ErrorKind::NotFound,
        }
    }
}

#[cfg(windows)]
pub fn is_elevated() -> bool {
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // SAFETY: plain Win32 calls on the current process token; the handle is closed before returning
    // and `elevation` is a correctly sized out-buffer for TokenElevation.
    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut returned = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        );
        CloseHandle(token);
        ok != 0 && elevation.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn system_dir_is_the_real_system32() {
        assert!(system_dir().join("pnputil.exe").exists());
    }

    #[test]
    fn backups_root_is_outside_the_user_profile() {
        let root = backups_root();
        assert!(root.ends_with(r"ProgramData\Driverix\backups"), "{root:?}");
        let profile = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_default();
        assert!(!root.starts_with(profile));
    }

    #[test]
    fn plain_directories_are_not_reparse_points() {
        assert!(!is_reparse_point(&std::env::temp_dir()));
    }

    #[test]
    fn missing_paths_are_not_reparse_points() {
        assert!(!is_reparse_point(
            &std::env::temp_dir().join("driverix-no-such-dir-7f3a")
        ));
    }

    #[test]
    fn uninspectable_paths_fail_closed() {
        // A NUL byte makes the metadata call fail with InvalidInput, not NotFound.
        assert!(is_reparse_point(std::path::Path::new("bad\0name")));
    }
}
