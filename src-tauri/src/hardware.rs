//! Machine overview in the style of AIDA64: `scripts/hardware.ps1` collects the facts, `parse`
//! reads them, `sections` turns them into the labelled rows the UI and the text report share.

use crate::error::{Error, Result};
use crate::ps::{run_script, scripts};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HardwareInfo {
    pub system: System,
    pub os: Os,
    pub board: Board,
    pub bios: Bios,
    pub cpus: Vec<Cpu>,
    pub memory: Memory,
    pub gpus: Vec<Gpu>,
    pub disks: Vec<Disk>,
    pub volumes: Vec<Volume>,
    pub network: Vec<Nic>,
    pub battery: Option<Battery>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct System {
    pub manufacturer: String,
    pub model: String,
    pub family: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Os {
    pub name: String,
    pub version: String,
    pub build: String,
    pub display_version: String,
    pub architecture: String,
    pub install_unix: u64,
    pub uptime_secs: u64,
    pub total_mem_kb: u64,
    pub free_mem_kb: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Board {
    pub manufacturer: String,
    pub product: String,
    pub version: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Bios {
    pub vendor: String,
    pub version: String,
    /// `YYYY-MM-DD`.
    pub date: Option<String>,
    pub smbios: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Cpu {
    pub name: String,
    pub manufacturer: String,
    /// `Win32_Processor.Architecture` code.
    pub architecture: u32,
    pub cores: u32,
    pub threads: u32,
    pub max_mhz: u32,
    pub socket: String,
    pub l2_kb: u32,
    pub l3_kb: u32,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Memory {
    pub slots: u32,
    pub modules: Vec<MemModule>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MemModule {
    pub slot: String,
    pub size_bytes: u64,
    pub speed_mhz: u32,
    /// SMBIOS memory type code.
    pub memory_type: u32,
    pub manufacturer: String,
    pub part: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Gpu {
    pub name: String,
    pub vendor: String,
    pub vram_bytes: u64,
    pub driver_version: String,
    pub driver_date: Option<String>,
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Disk {
    pub model: String,
    pub size_bytes: u64,
    pub media: String,
    pub bus: String,
    pub health: String,
    pub firmware: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Volume {
    pub letter: String,
    pub label: String,
    pub fs: String,
    pub size_bytes: u64,
    pub free_bytes: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Nic {
    pub name: String,
    pub mac: String,
    pub speed_bps: u64,
    pub connected: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Battery {
    pub name: String,
    /// `None` when the battery does not report a charge.
    pub charge_percent: Option<u32>,
    pub status_code: u32,
}

/// One line of a section; `heading` rows only carry a label and group the rows below them.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub label: String,
    pub value: String,
    /// 0-100 when the value is a usage the UI can draw as a bar.
    pub meter: Option<u8>,
    pub heading: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub id: String,
    pub title: String,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareView {
    pub sections: Vec<Section>,
    /// Plain-text version of `sections`, for copying into a forum post or a bug report.
    pub report: String,
}

pub fn parse(json: &str) -> Result<HardwareInfo> {
    let json = json.trim();
    if json.is_empty() {
        return Err(Error::Parse(
            "o coletor de hardware não retornou nada".into(),
        ));
    }
    Ok(serde_json::from_str(json)?)
}

pub fn collect(cancel: &AtomicBool) -> Result<HardwareView> {
    let info = parse(&run_script(
        "leitura de hardware",
        scripts::HARDWARE,
        None,
        Duration::from_secs(90),
        cancel,
    )?)?;
    Ok(view(&info))
}

pub fn view(info: &HardwareInfo) -> HardwareView {
    let sections = sections(info);
    let report = render_report(&sections);
    HardwareView { sections, report }
}

// ---- formatting -------------------------------------------------------------------------------

/// Binary units, one decimal only when it matters: `16 GB`, `1.5 GB`.
pub fn fmt_bytes(n: u64) -> String {
    if n == 0 {
        return "—".into();
    }
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 || (v - v.round()).abs() < 0.05 || v >= 100.0 {
        format!("{} {}", v.round() as u64, UNITS[i])
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

fn fmt_mhz(mhz: u32) -> String {
    match mhz {
        0 => "—".into(),
        m if m >= 1000 => format!("{:.2} GHz", m as f64 / 1000.0),
        m => format!("{m} MHz"),
    }
}

fn fmt_uptime(secs: u64) -> String {
    let (d, h, m) = (secs / 86_400, secs % 86_400 / 3600, secs % 3600 / 60);
    match (d, h) {
        (0, 0) => format!("{m} min"),
        (0, _) => format!("{h} h {m} min"),
        _ => format!("{d} d {h} h {m} min"),
    }
}

/// `2024-05-06` -> `06/05/2024`.
fn fmt_date(iso: &Option<String>) -> String {
    let parts: Vec<&str> = iso.as_deref().unwrap_or("").split('-').collect();
    match parts[..] {
        [y, m, d] if y.len() == 4 => format!("{d}/{m}/{y}"),
        _ => "—".into(),
    }
}

/// Days since the epoch -> `dd/mm/yyyy` (civil-from-days, proleptic Gregorian, UTC).
fn fmt_unix_date(secs: u64) -> String {
    if secs == 0 {
        return "—".into();
    }
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{day:02}/{month:02}/{year}")
}

fn or_dash(s: &str) -> String {
    if s.trim().is_empty() {
        "—".into()
    } else {
        s.trim().to_string()
    }
}

fn join_nonempty(parts: &[&str]) -> String {
    let joined: Vec<&str> = parts
        .iter()
        .copied()
        .filter(|p| !p.trim().is_empty())
        .collect();
    or_dash(&joined.join(" "))
}

fn memory_kind(code: u32) -> &'static str {
    match code {
        20 => "DDR",
        21 => "DDR2",
        24 => "DDR3",
        26 => "DDR4",
        27 => "LPDDR",
        28 => "LPDDR2",
        29 => "LPDDR3",
        30 => "LPDDR4",
        34 => "DDR5",
        35 => "LPDDR5",
        _ => "",
    }
}

fn cpu_arch(code: u32) -> &'static str {
    match code {
        0 => "x86",
        5 => "ARM",
        9 => "x64",
        12 => "ARM64",
        _ => "—",
    }
}

fn health_text(h: &str) -> String {
    match h {
        "Healthy" | "OK" => "Saudável".into(),
        "Warning" => "Atenção".into(),
        "Unhealthy" => "Com falha".into(),
        other => or_dash(other),
    }
}

fn battery_status(code: u32) -> &'static str {
    match code {
        1 => "Descarregando",
        2 => "Ligada na tomada",
        3 => "Carga completa",
        4 => "Baixa",
        5 => "Crítica",
        6..=9 => "Carregando",
        _ => "Desconhecido",
    }
}

/// The first three octets identify the manufacturer; the rest identify the device and must not
/// be pasted with the report.
fn mask_mac(mac: &str) -> String {
    let octets: Vec<String> = mac
        .split([':', '-'])
        .filter(|o| !o.is_empty())
        .take(3)
        .map(|o| o.to_ascii_uppercase())
        .collect();
    if octets.len() < 3 {
        return "—".into();
    }
    format!("{}:••:••:••", octets.join(":"))
}

fn fmt_link_speed(bps: u64) -> String {
    // Disconnected adapters report u64::MAX / i64::MAX instead of a speed.
    if bps == 0 || bps >= 1_000_000_000_000 {
        return "—".into();
    }
    if bps >= 1_000_000_000 {
        // Whole tenths of a Gbps, so a 2.5 GbE link is not shown as 2.
        let tenths = (bps + 50_000_000) / 100_000_000;
        if tenths.is_multiple_of(10) {
            format!("{} Gbps", tenths / 10)
        } else {
            format!("{}.{} Gbps", tenths / 10, tenths % 10)
        }
    } else {
        format!("{} Mbps", bps / 1_000_000)
    }
}

fn pct(part: u64, whole: u64) -> Option<u8> {
    (whole > 0).then(|| (part.min(whole) * 100 / whole) as u8)
}

// ---- sections ---------------------------------------------------------------------------------

struct Builder(Vec<Row>);

impl Builder {
    fn row(&mut self, label: &str, value: impl Into<String>) {
        self.0.push(Row {
            label: label.into(),
            value: value.into(),
            meter: None,
            heading: false,
        });
    }

    fn meter(&mut self, label: &str, value: impl Into<String>, meter: Option<u8>) {
        self.0.push(Row {
            label: label.into(),
            value: value.into(),
            meter,
            heading: false,
        });
    }

    fn heading(&mut self, label: impl Into<String>) {
        self.0.push(Row {
            label: label.into(),
            value: String::new(),
            meter: None,
            heading: true,
        });
    }
}

fn section(id: &str, title: &str, build: impl FnOnce(&mut Builder)) -> Section {
    let mut b = Builder(Vec::new());
    build(&mut b);
    Section {
        id: id.into(),
        title: title.into(),
        rows: b.0,
    }
}

fn total_mem_bytes(info: &HardwareInfo) -> u64 {
    let modules: u64 = info.memory.modules.iter().map(|m| m.size_bytes).sum();
    // The OS figure excludes hardware-reserved memory, so the installed modules win when known.
    if modules > 0 {
        modules
    } else {
        info.os.total_mem_kb * 1024
    }
}

pub fn sections(info: &HardwareInfo) -> Vec<Section> {
    let mut out = vec![summary(info)];

    out.push(section("os", "Sistema operacional", |b| {
        let os = &info.os;
        b.row("Sistema", or_dash(&os.name));
        b.row(
            "Versão",
            join_nonempty(&[&os.display_version, &format!("(build {})", os.build)]),
        );
        b.row("Kernel", or_dash(&os.version));
        b.row("Arquitetura", or_dash(&os.architecture));
        b.row("Instalado em", fmt_unix_date(os.install_unix));
        b.row("Ligado há", fmt_uptime(os.uptime_secs));
    }));

    out.push(section("board", "Placa-mãe e BIOS", |b| {
        b.row(
            "Fabricante do computador",
            or_dash(&info.system.manufacturer),
        );
        b.row("Modelo", or_dash(&info.system.model));
        b.row("Família", or_dash(&info.system.family));
        b.row("Fabricante da placa", or_dash(&info.board.manufacturer));
        b.row(
            "Placa-mãe",
            join_nonempty(&[&info.board.product, &info.board.version]),
        );
        b.heading("BIOS");
        b.row("Fabricante", or_dash(&info.bios.vendor));
        b.row("Versão", or_dash(&info.bios.version));
        b.row("Data", fmt_date(&info.bios.date));
        b.row("SMBIOS", or_dash(&info.bios.smbios));
    }));

    out.push(section("cpu", "Processador", |b| {
        for (i, c) in info.cpus.iter().enumerate() {
            if info.cpus.len() > 1 {
                b.heading(format!("Soquete {}", i + 1));
            }
            b.row("Modelo", or_dash(&c.name));
            b.row("Fabricante", or_dash(&c.manufacturer));
            b.row("Arquitetura", cpu_arch(c.architecture));
            b.row("Núcleos / threads", format!("{} / {}", c.cores, c.threads));
            b.row("Clock máximo", fmt_mhz(c.max_mhz));
            b.row("Soquete", or_dash(&c.socket));
            b.row("Cache L2", fmt_bytes(u64::from(c.l2_kb) * 1024));
            b.row("Cache L3", fmt_bytes(u64::from(c.l3_kb) * 1024));
        }
    }));

    out.push(section("memory", "Memória", |b| {
        let total = info.os.total_mem_kb * 1024;
        let used = total.saturating_sub(info.os.free_mem_kb * 1024);
        b.row("Instalada", fmt_bytes(total_mem_bytes(info)));
        if total > 0 {
            b.meter(
                "Em uso",
                format!("{} de {}", fmt_bytes(used), fmt_bytes(total)),
                pct(used, total),
            );
        }
        let slots = if info.memory.slots > 0 {
            format!("{} de {}", info.memory.modules.len(), info.memory.slots)
        } else {
            info.memory.modules.len().to_string()
        };
        b.row("Slots ocupados", slots);
        for m in &info.memory.modules {
            b.heading(or_dash(&m.slot));
            let kind = memory_kind(m.memory_type);
            let speed = if m.speed_mhz > 0 {
                format!("{} MT/s", m.speed_mhz)
            } else {
                String::new()
            };
            b.row(
                "Capacidade",
                join_nonempty(&[&fmt_bytes(m.size_bytes), kind, &speed]),
            );
            b.row("Fabricante", or_dash(&m.manufacturer));
            b.row("Modelo", or_dash(&m.part));
        }
    }));

    out.push(section("video", "Vídeo", |b| {
        for g in &info.gpus {
            b.heading(or_dash(&g.name));
            b.row("Fabricante", or_dash(&g.vendor));
            b.row("Memória de vídeo", fmt_bytes(g.vram_bytes));
            b.row(
                "Driver",
                join_nonempty(&[
                    &g.driver_version,
                    &format!("({})", fmt_date(&g.driver_date)),
                ]),
            );
            if g.width > 0 {
                b.row(
                    "Resolução",
                    format!("{}×{} a {} Hz", g.width, g.height, g.refresh_hz),
                );
            }
        }
    }));

    out.push(section("storage", "Armazenamento", |b| {
        for d in &info.disks {
            b.heading(or_dash(&d.model));
            b.row("Capacidade", fmt_bytes(d.size_bytes));
            b.row("Tipo", join_nonempty(&[&d.media, &d.bus]));
            b.row("Saúde", health_text(&d.health));
            b.row("Firmware", or_dash(&d.firmware));
        }
        for v in &info.volumes {
            let used = v.size_bytes.saturating_sub(v.free_bytes);
            let name = join_nonempty(&[&v.letter, &v.label]);
            b.meter(
                &name,
                format!(
                    "{} livres de {} · {}",
                    fmt_bytes(v.free_bytes),
                    fmt_bytes(v.size_bytes),
                    or_dash(&v.fs)
                ),
                pct(used, v.size_bytes),
            );
        }
    }));

    out.push(section("network", "Rede", |b| {
        for n in &info.network {
            b.heading(or_dash(&n.name));
            b.row("Endereço MAC", mask_mac(&n.mac));
            b.row(
                "Estado",
                if n.connected {
                    "Conectado"
                } else {
                    "Desconectado"
                },
            );
            b.row("Velocidade", fmt_link_speed(n.speed_bps));
        }
    }));

    if let Some(bat) = &info.battery {
        out.push(section("battery", "Bateria", |b| {
            b.row("Modelo", or_dash(&bat.name));
            match bat.charge_percent {
                Some(p) => b.meter("Carga", format!("{p}%"), Some(p.min(100) as u8)),
                None => b.row("Carga", "desconhecida"),
            }
            b.row("Estado", battery_status(bat.status_code));
        }));
    }
    out
}

fn summary(info: &HardwareInfo) -> Section {
    section("summary", "Resumo", |b| {
        b.row(
            "Computador",
            join_nonempty(&[&info.system.manufacturer, &info.system.model]),
        );
        b.row(
            "Sistema",
            join_nonempty(&[&info.os.name, &info.os.display_version]),
        );
        for c in &info.cpus {
            b.row(
                "Processador",
                format!("{} ({}c/{}t)", or_dash(&c.name), c.cores, c.threads),
            );
        }
        b.row("Memória", fmt_bytes(total_mem_bytes(info)));
        for g in &info.gpus {
            b.row(
                "Vídeo",
                format!("{} · {}", or_dash(&g.name), fmt_bytes(g.vram_bytes)),
            );
        }
        for d in &info.disks {
            b.row(
                "Disco",
                format!("{} · {}", or_dash(&d.model), fmt_bytes(d.size_bytes)),
            );
        }
        b.row("Ligado há", fmt_uptime(info.os.uptime_secs));
    })
}

pub fn render_report(sections: &[Section]) -> String {
    let mut out = String::from("Driverix · relatório de hardware\n");
    for s in sections {
        out.push_str(&format!("\n== {} ==\n", s.title));
        let width = s
            .rows
            .iter()
            .filter(|r| !r.heading)
            .map(|r| r.label.chars().count())
            .max()
            .unwrap_or(0);
        for r in &s.rows {
            if r.heading {
                out.push_str(&format!("\n  [{}]\n", r.label));
            } else {
                let pad = " ".repeat(width - r.label.chars().count());
                out.push_str(&format!("  {}{pad}  {}\n", r.label, r.value));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "system":{"manufacturer":"ASUS","model":"ROG Strix","family":"ROG"},
      "os":{"name":"Microsoft Windows 11 Pro","version":"10.0.22631","build":"22631","displayVersion":"23H2",
            "architecture":"64-bit","installUnix":1700000000,"uptimeSecs":93784,
            "totalMemKb":16000000,"freeMemKb":4000000},
      "board":{"manufacturer":"ASUSTeK","product":"B650-A","version":"Rev 1.xx"},
      "bios":{"vendor":"AMI","version":"2801","date":"2024-03-09","smbios":"3.6"},
      "cpus":[{"name":"AMD Ryzen 7 7700X","manufacturer":"AuthenticAMD","architecture":9,"cores":8,"threads":16,
               "maxMhz":4500,"socket":"AM5","l2Kb":8192,"l3Kb":32768}],
      "memory":{"slots":4,"modules":[
         {"slot":"DIMM_A1","sizeBytes":8589934592,"speedMhz":5600,"memoryType":34,"manufacturer":"Kingston","part":"KF556"},
         {"slot":"DIMM_B1","sizeBytes":8589934592,"speedMhz":5600,"memoryType":34,"manufacturer":"Kingston","part":"KF556"}]},
      "gpus":[{"name":"NVIDIA RTX 4070","vendor":"NVIDIA","vramBytes":12884901888,"driverVersion":"32.0.15.6094",
               "driverDate":"2024-10-01","width":2560,"height":1440,"refreshHz":165}],
      "disks":[{"model":"Samsung SSD 980","sizeBytes":1000204886016,"media":"SSD","bus":"NVMe","health":"Healthy","firmware":"1B2Q"}],
      "volumes":[{"letter":"C:","label":"Sistema","fs":"NTFS","sizeBytes":1000000000000,"freeBytes":250000000000}],
      "network":[{"name":"Realtek 2.5GbE","mac":"AA:BB:CC:DD:EE:FF","speedBps":2500000000,"connected":true},
                 {"name":"Wi-Fi","mac":"","speedBps":9223372036854775807,"connected":false}],
      "battery":null
    }"#;

    fn sample() -> HardwareInfo {
        parse(SAMPLE).unwrap()
    }

    fn find<'a>(s: &'a [Section], id: &str) -> &'a Section {
        s.iter().find(|s| s.id == id).unwrap()
    }

    fn value<'a>(s: &'a Section, label: &str) -> &'a str {
        &s.rows.iter().find(|r| r.label == label).unwrap().value
    }

    #[test]
    fn parses_the_script_output() {
        let i = sample();
        assert_eq!(i.cpus[0].threads, 16);
        assert_eq!(i.memory.modules.len(), 2);
        assert!(i.battery.is_none());
    }

    #[test]
    fn missing_fields_default_instead_of_failing() {
        let i = parse(r#"{"os":{"name":"Windows"}}"#).unwrap();
        assert!(i.cpus.is_empty() && i.gpus.is_empty());
        assert!(!sections(&i).is_empty());
    }

    #[test]
    fn empty_or_malformed_output_is_an_error() {
        assert!(parse("  ").is_err());
        assert!(parse("{nope").is_err());
    }

    #[test]
    fn formats_sizes_clocks_and_uptime() {
        assert_eq!(fmt_bytes(0), "—");
        assert_eq!(fmt_bytes(8 * 1024 * 1024 * 1024), "8 GB");
        assert_eq!(fmt_bytes(1536 * 1024 * 1024), "1.5 GB");
        assert_eq!(fmt_bytes(1000204886016), "932 GB");
        assert_eq!(fmt_mhz(4500), "4.50 GHz");
        assert_eq!(fmt_mhz(800), "800 MHz");
        assert_eq!(fmt_uptime(93_784), "1 d 2 h 3 min");
        assert_eq!(fmt_uptime(300), "5 min");
    }

    #[test]
    fn converts_unix_time_to_a_civil_date() {
        assert_eq!(fmt_unix_date(0), "—");
        assert_eq!(fmt_unix_date(1_700_000_000), "14/11/2023");
        assert_eq!(fmt_unix_date(951_782_400), "29/02/2000");
    }

    #[test]
    fn summary_lists_the_main_parts() {
        let s = sections(&sample());
        let sum = find(&s, "summary");
        assert_eq!(value(sum, "Computador"), "ASUS ROG Strix");
        assert_eq!(value(sum, "Processador"), "AMD Ryzen 7 7700X (8c/16t)");
        assert_eq!(value(sum, "Memória"), "16 GB");
        assert_eq!(value(sum, "Vídeo"), "NVIDIA RTX 4070 · 12 GB");
    }

    #[test]
    fn memory_shows_modules_slots_and_usage() {
        let s = sections(&sample());
        let mem = find(&s, "memory");
        assert_eq!(value(mem, "Slots ocupados"), "2 de 4");
        let used = mem.rows.iter().find(|r| r.label == "Em uso").unwrap();
        assert_eq!(used.meter, Some(75));
        assert!(mem.rows.iter().any(|r| r.heading && r.label == "DIMM_A1"));
        assert_eq!(value(mem, "Capacidade"), "8 GB DDR5 5600 MT/s");
    }

    #[test]
    fn volumes_get_a_usage_meter() {
        let s = sections(&sample());
        let c = find(&s, "storage")
            .rows
            .iter()
            .find(|r| r.label == "C: Sistema")
            .unwrap();
        assert_eq!(c.meter, Some(75));
        assert!(c.value.contains("NTFS"));
    }

    #[test]
    fn disconnected_adapters_have_no_speed() {
        let s = sections(&sample());
        let net = find(&s, "network");
        let speeds: Vec<&str> = net
            .rows
            .iter()
            .filter(|r| r.label == "Velocidade")
            .map(|r| r.value.as_str())
            .collect();
        assert_eq!(speeds, ["2.5 Gbps", "—"]);
    }

    #[test]
    fn link_speed_keeps_its_fraction() {
        assert_eq!(fmt_link_speed(1_000_000_000), "1 Gbps");
        assert_eq!(fmt_link_speed(2_500_000_000), "2.5 Gbps");
        assert_eq!(fmt_link_speed(10_000_000_000), "10 Gbps");
        assert_eq!(fmt_link_speed(100_000_000), "100 Mbps");
    }

    #[test]
    fn a_mac_shows_only_its_manufacturer_prefix() {
        assert_eq!(mask_mac("AA:BB:CC:DD:EE:FF"), "AA:BB:CC:••:••:••");
        assert_eq!(mask_mac("aa-bb-cc-dd-ee-ff"), "AA:BB:CC:••:••:••");
        assert_eq!(mask_mac(""), "—");
    }

    #[test]
    fn battery_section_only_exists_with_a_battery() {
        assert!(sections(&sample()).iter().all(|s| s.id != "battery"));
        let mut i = sample();
        i.battery = Some(Battery {
            name: "Main".into(),
            charge_percent: Some(80),
            status_code: 6,
        });
        let s = sections(&i);
        assert_eq!(value(find(&s, "battery"), "Estado"), "Carregando");
    }

    #[test]
    fn an_unreported_charge_is_not_shown_as_zero() {
        let mut i = sample();
        i.battery = Some(Battery {
            name: "Main".into(),
            charge_percent: None,
            status_code: 2,
        });
        let s = sections(&i);
        assert_eq!(value(find(&s, "battery"), "Carga"), "desconhecida");
    }

    #[test]
    fn report_aligns_labels_and_lists_every_section() {
        let report = view(&sample()).report;
        assert!(report.starts_with("Driverix · relatório de hardware"));
        assert!(report.contains("== Processador =="));
        assert!(report.contains("AMD Ryzen 7 7700X"));
        assert!(report.contains("\n  [DIMM_A1]\n"));
    }

    #[test]
    fn meters_never_exceed_100() {
        assert_eq!(pct(5, 0), None);
        assert_eq!(pct(200, 100), Some(100));
    }
}
