//! Headless, read-only front end: `driverix-cli devices|hardware|scan [--json]`. It never installs;
//! installing stays in the app, where the confirmation and backup flow live.

use driverix_lib::{
    hardware, inventory,
    model::Decision,
    scan::{scan_all, ScanResult},
};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let never = AtomicBool::new(false);
    match args.first().map(String::as_str) {
        Some("devices") => match inventory::collect(&never) {
            Ok(devices) if json => print_json(&devices),
            Ok(devices) => {
                let broken = devices.iter().filter(|d| d.problem_code != 0).count();
                println!("{} dispositivos, {broken} com problema", devices.len());
                for d in devices.iter().filter(|d| d.problem_code != 0) {
                    println!("  [{}] {} ({})", d.problem_code, d.name, d.class);
                }
                ExitCode::SUCCESS
            }
            Err(e) => fail(e),
        },
        Some("hardware") => match hardware::collect(&never) {
            Ok(view) if json => print_json(&view.sections),
            Ok(view) => {
                print!("{}", view.report);
                ExitCode::SUCCESS
            }
            Err(e) => fail(e),
        },
        Some("scan") => match scan_all(&never, |p| eprintln!("... {p}")) {
            Ok(r) if json => print_json_with(&r, exit_code_for(&r)),
            Ok(r) => {
                if let Some(msg) = &r.updates_error {
                    eprintln!("aviso: não foi possível consultar o Windows Update: {msg}");
                }
                let newer = r
                    .offers
                    .iter()
                    .filter(|o| o.decision == Decision::Newer)
                    .count();
                println!(
                    "{} dispositivos, {} ofertas, {newer} mais novas que o instalado",
                    r.devices.len(),
                    r.offers.len()
                );
                for o in r.offers.iter().filter(|o| o.installable()) {
                    println!("  {} ({})", o.candidate.title, o.reason);
                }
                ExitCode::from(exit_code_for(&r))
            }
            Err(e) => fail(e),
        },
        _ => {
            eprintln!("uso: driverix-cli <devices|hardware|scan> [--json]");
            ExitCode::from(2)
        }
    }
}

/// 0 = scan completed; 3 = devices listed but Windows Update could not be queried, so "no offers"
/// must not be read as "everything is up to date".
fn exit_code_for(r: &ScanResult) -> u8 {
    if r.updates_error.is_some() {
        3
    } else {
        0
    }
}

fn print_json<T: serde::Serialize>(v: &T) -> ExitCode {
    print_json_with(v, 0)
}

fn print_json_with<T: serde::Serialize>(v: &T, code: u8) -> ExitCode {
    match serde_json::to_string_pretty(v) {
        Ok(s) => {
            println!("{s}");
            ExitCode::from(code)
        }
        Err(e) => fail(e),
    }
}

fn fail(e: impl std::fmt::Display) -> ExitCode {
    eprintln!("erro: {e}");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(err: Option<&str>) -> ScanResult {
        ScanResult {
            devices: Vec::new(),
            offers: Vec::new(),
            updates_error: err.map(String::from),
        }
    }

    #[test]
    fn a_failed_windows_update_query_is_not_reported_as_success() {
        assert_eq!(exit_code_for(&scan(Some("serviço parado"))), 3);
        assert_eq!(exit_code_for(&scan(None)), 0);
    }
}
