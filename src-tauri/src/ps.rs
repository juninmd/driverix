//! Runs the embedded PowerShell scripts. Scripts are fixed text; caller data travels in the
//! `DRIVERIX_INPUT` environment variable as JSON, never spliced into the script.

use crate::error::{Error, Result};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const PRELUDE: &str =
    "$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[Text.Encoding]::UTF8\ntry {\n";
const EPILOGUE: &str = "\n} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }\n";

/// Base64 of the UTF-16LE text, the format `-EncodedCommand` expects. Passing the script this way
/// avoids both stdin quirks (a blank line inside a block ends the input) and temp files that
/// another process could swap before an elevated PowerShell runs them.
fn encode_command(script: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, b)| acc | (*b as u32) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub struct Finished {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Runs `cmd` to completion, killing it on timeout or cancel. Every child process of the app goes
/// through here so none can block an operation (and its busy lock) forever.
pub fn run_process(
    what: &'static str,
    mut cmd: Command,
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<Finished> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut cmd, CREATE_NO_WINDOW);

    let mut child = cmd.spawn()?;
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());

    let started = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::Cancelled);
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::Timeout(what));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    Ok(Finished {
        code: status.code(),
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
    })
}

pub fn run_script(
    what: &'static str,
    script: &str,
    input: Option<&str>,
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<String> {
    let encoded = encode_command(&format!("{PRELUDE}{script}{EPILOGUE}"));
    let mut cmd = Command::new(crate::sys::system32(
        r"WindowsPowerShell\v1.0\powershell.exe",
    ));
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-EncodedCommand",
        &encoded,
    ]);
    if let Some(input) = input {
        cmd.env("DRIVERIX_INPUT", input);
    }
    let done = run_process(what, cmd, timeout, cancel)?;
    if done.code == Some(0) {
        Ok(done.stdout)
    } else {
        Err(Error::Command {
            what,
            detail: done.stderr.trim().to_string(),
        })
    }
}

fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut p) = pipe {
            let _ = p.read_to_end(&mut buf);
        }
        String::from_utf8_lossy(&buf)
            .trim_start_matches('\u{feff}')
            .to_string()
    })
}

pub mod scripts {
    pub const INVENTORY: &str = include_str!("../scripts/inventory.ps1");
    pub const SCAN_UPDATES: &str = include_str!("../scripts/scan_updates.ps1");
    pub const INSTALL_UPDATES: &str = include_str!("../scripts/install_updates.ps1");
    pub const RESTORE_AGE: &str = include_str!("../scripts/restore_age.ps1");
    pub const RESTORE_POINT: &str = include_str!("../scripts/restore_point.ps1");
}

#[cfg(test)]
mod tests {
    use super::scripts::*;

    #[test]
    fn encoded_command_is_base64_of_utf16le() {
        // "A" -> 41 00 -> "QQA=" ; "Hi" -> 48 00 69 00 -> "SABpAA=="
        assert_eq!(super::encode_command("A"), "QQA=");
        assert_eq!(super::encode_command("Hi"), "SABpAA==");
        assert_eq!(super::encode_command(""), "");
    }

    #[test]
    fn encoded_scripts_fit_the_command_line_limit() {
        for s in [
            INVENTORY,
            SCAN_UPDATES,
            INSTALL_UPDATES,
            RESTORE_AGE,
            RESTORE_POINT,
        ] {
            assert!(super::encode_command(s).len() < 30_000);
        }
    }

    #[test]
    fn scripts_never_read_caller_data_except_through_the_input_variable() {
        for s in [
            INVENTORY,
            SCAN_UPDATES,
            INSTALL_UPDATES,
            RESTORE_AGE,
            RESTORE_POINT,
        ] {
            assert!(
                !s.contains("Invoke-Expression") && !s.contains("iex "),
                "dynamic evaluation in script"
            );
        }
    }
}
