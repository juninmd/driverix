//! Runs the embedded PowerShell scripts. Scripts are fixed text; caller data travels in the
//! `DRIVERIX_INPUT` environment variable as JSON, never spliced into the script.

use crate::error::{Error, Result};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
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
    // One deadline for both pipes: a grandchild usually holds stdout and stderr together.
    let deadline = Instant::now() + DRAIN_GRACE;
    Ok(Finished {
        code: status.code(),
        stdout: collect(out, deadline, what)?,
        stderr: collect(err, deadline, what)?,
    })
}

/// How long to wait for a pipe to close after the child exits; a grandchild can keep it open.
const DRAIN_GRACE: Duration = Duration::from_secs(5);

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

/// Largest output a child may write. Scripts print a few hundred KB at most; a bigger stream is a
/// fault, and reading it all would hold it in memory before the JSON parse fails on the truncation.
const MAX_OUTPUT: usize = 16 * 1024 * 1024;

/// A pipe being read on its own thread: `buf` grows as bytes arrive, `done` fires at EOF, and
/// `overflow` is set when the stream passed the size limit (the reader then stops).
struct Drain {
    buf: Arc<Mutex<Vec<u8>>>,
    done: Receiver<()>,
    overflow: Arc<AtomicBool>,
}

fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> Drain {
    spawn_reader(pipe, MAX_OUTPUT)
}

fn spawn_reader<R: Read + Send + 'static>(pipe: Option<R>, limit: usize) -> Drain {
    let buf = Arc::new(Mutex::new(Vec::new()));
    let overflow = Arc::new(AtomicBool::new(false));
    let (tx, done) = mpsc::channel();
    let (sink, flag) = (Arc::clone(&buf), Arc::clone(&overflow));
    std::thread::spawn(move || {
        if let Some(mut p) = pipe {
            let mut chunk = [0u8; 8192];
            while let Ok(n) = p.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                if let Ok(mut b) = sink.lock() {
                    if b.len() + n > limit {
                        flag.store(true, Ordering::Relaxed);
                        break;
                    }
                    b.extend_from_slice(&chunk[..n]);
                }
            }
        }
        let _ = tx.send(());
    });
    Drain {
        buf,
        done,
        overflow,
    }
}

/// Returns what the child wrote. Once the child has exited, everything it wrote is already in
/// the pipe, so a pipe still held open by a grandchild only costs the wait, not the output.
/// An overflowing stream is an error, never a silently cut-off result.
fn collect(d: Drain, deadline: Instant, what: &'static str) -> Result<String> {
    let _ = d
        .done
        .recv_timeout(deadline.saturating_duration_since(Instant::now()));
    if d.overflow.load(Ordering::Relaxed) {
        return Err(Error::Command {
            what,
            detail: format!("saída maior que {} MiB", MAX_OUTPUT >> 20),
        });
    }
    let bytes = d.buf.lock().map(|b| b.clone()).unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes)
        .trim_start_matches('\u{feff}')
        .to_string())
}

pub mod scripts {
    pub const INVENTORY: &str = include_str!("../scripts/inventory.ps1");
    pub const HARDWARE: &str = include_str!("../scripts/hardware.ps1");
    pub const SCAN_UPDATES: &str = include_str!("../scripts/scan_updates.ps1");
    pub const INSTALL_UPDATES: &str = include_str!("../scripts/install_updates.ps1");
    pub const RESTORE_AGE: &str = include_str!("../scripts/restore_age.ps1");
    pub const RESTORE_POINT: &str = include_str!("../scripts/restore_point.ps1");
}

#[cfg(test)]
mod tests {
    use super::scripts::*;
    use super::*;

    #[cfg(windows)]
    #[test]
    fn a_grandchild_holding_the_pipe_cannot_block_past_the_deadline() {
        // `start /B` leaves a process that inherits our stdout, so the pipe stays open after cmd exits.
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start /B ping -n 20 127.0.0.1"]);
        let cancel = AtomicBool::new(false);
        let started = Instant::now();
        let done = run_process("teste", cmd, Duration::from_secs(30), &cancel)
            .expect("the child exited cleanly, so its result must not turn into a failure");
        assert_eq!(done.code, Some(0));
        // ping runs for ~20 s; the call must return after the drain grace, not wait for it.
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "{:?}",
            started.elapsed()
        );
    }

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
            HARDWARE,
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
            HARDWARE,
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

    #[test]
    fn output_at_the_limit_is_kept_whole() {
        let d = spawn_reader(Some(std::io::Cursor::new(b"0123456789".to_vec())), 10);
        let out = collect(d, Instant::now() + Duration::from_secs(5), "teste").unwrap();
        assert_eq!(out, "0123456789");
    }

    #[test]
    fn output_over_the_limit_is_an_error_not_a_truncated_result() {
        let d = spawn_reader(Some(std::io::Cursor::new(vec![b'x'; 100])), 10);
        let err = collect(d, Instant::now() + Duration::from_secs(5), "teste").unwrap_err();
        assert!(
            matches!(err, Error::Command { what: "teste", .. }),
            "{err:?}"
        );
    }
}
