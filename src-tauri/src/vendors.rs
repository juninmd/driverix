//! Offline hardware-ID database: turns `PCI\VEN_10DE&DEV_2684` into "NVIDIA" and a class hint.
//! A device stuck at problem 28 often has no name at all, and its IDs are the only clue left.

use crate::model::Device;

/// PCI vendor IDs (pcisig.com registry) of the vendors that ship most Windows drivers.
const PCI_VENDORS: &[(&str, &str)] = &[
    ("1002", "AMD / ATI"),
    ("1022", "AMD"),
    ("1028", "Dell"),
    ("103C", "HP"),
    ("1043", "ASUS"),
    ("104C", "Texas Instruments"),
    ("105B", "Foxconn"),
    ("10DE", "NVIDIA"),
    ("10EC", "Realtek"),
    ("1106", "VIA"),
    ("1131", "NXP"),
    ("1180", "Ricoh"),
    ("11AB", "Marvell"),
    ("1217", "O2 Micro"),
    ("12D8", "Pericom"),
    ("1414", "Microsoft"),
    ("144D", "Samsung"),
    ("14C3", "MediaTek"),
    ("14E4", "Broadcom"),
    ("1558", "Clevo"),
    ("15B7", "SanDisk"),
    ("15AD", "VMware"),
    ("168C", "Qualcomm Atheros"),
    ("1969", "Qualcomm Atheros"),
    ("1987", "Phison"),
    ("1AE0", "Google"),
    ("1AF4", "Red Hat (virtio)"),
    ("1B21", "ASMedia"),
    ("1B4B", "Marvell"),
    ("1C5C", "SK hynix"),
    ("1CC1", "ADATA"),
    ("1D17", "Zhaoxin"),
    ("1D97", "Longsys"),
    ("1E0F", "KIOXIA"),
    ("1E49", "YMTC"),
    ("1E95", "Solid State Storage"),
    ("1912", "Renesas"),
    ("1B73", "Fresco Logic"),
    ("1D6A", "Aquantia / Marvell"),
    ("17AA", "Lenovo"),
    ("1A03", "ASPEED"),
    ("1B36", "Red Hat (QEMU)"),
    ("1BB1", "Seagate"),
    ("1C58", "HGST"),
    ("8086", "Intel"),
    ("80EE", "VirtualBox"),
    ("9710", "MosChip"),
];

/// USB vendor IDs (usb.org registry), peripherals and the Bluetooth/Wi-Fi dongles that need drivers.
const USB_VENDORS: &[(&str, &str)] = &[
    ("03F0", "HP"),
    ("045E", "Microsoft"),
    ("046D", "Logitech"),
    ("0489", "Foxconn / Hon Hai"),
    ("04CA", "Lite-On"),
    ("04D9", "Holtek"),
    ("04F2", "Chicony"),
    ("04F3", "Elan"),
    ("054C", "Sony"),
    ("056A", "Wacom"),
    ("0586", "ZyXEL"),
    ("05AC", "Apple"),
    ("05C8", "Cheng Uei Precision"),
    ("0603", "Novatek"),
    ("0781", "SanDisk"),
    ("0846", "Netgear"),
    ("0930", "Toshiba"),
    ("093A", "Pixart"),
    ("0951", "Kingston"),
    ("0A5C", "Broadcom"),
    ("0A12", "Cambridge Silicon Radio"),
    ("0B05", "ASUS"),
    ("0BDA", "Realtek"),
    ("0CF3", "Qualcomm Atheros"),
    ("0E8D", "MediaTek"),
    ("0FCE", "Sony Mobile"),
    ("1038", "SteelSeries"),
    ("1050", "Yubico"),
    ("1532", "Razer"),
    ("1A2C", "China Resource Semico"),
    ("1B1C", "Corsair"),
    ("1BCF", "Sunplus"),
    ("2109", "VIA Labs"),
    ("2357", "TP-Link"),
    ("258A", "SinoWealth"),
    ("2717", "Xiaomi"),
    ("27C6", "Goodix"),
    ("2C7C", "Quectel"),
    ("30C9", "Luxvisions"),
    ("8087", "Intel"),
    ("8564", "Transcend"),
    ("0B95", "ASIX"),
    ("0955", "NVIDIA"),
    ("1A86", "QinHeng (CH340)"),
    ("0403", "FTDI"),
    ("067B", "Prolific"),
    ("10C4", "Silicon Labs"),
    ("18D1", "Google"),
    ("04E8", "Samsung"),
    ("12D1", "Huawei"),
    ("0E0F", "VMware"),
];

/// Base class of `PCI\CC_BBSS..` (PCI-SIG class codes).
fn pci_class(code: &str) -> Option<&'static str> {
    let code = code.to_ascii_uppercase();
    // A 4-digit code is exact; a 2-digit code (like `PCI\CC_03`) must still reach the base-class table.
    let exact = match code.get(..4) {
        Some("0101") => Some("Controlador IDE"),
        Some("0106") => Some("Controlador SATA"),
        Some("0108") => Some("Controlador NVMe"),
        Some("0200") => Some("Placa de rede Ethernet"),
        Some("0280") => Some("Placa de rede sem fio"),
        Some("0300") => Some("Controlador de vídeo"),
        Some("0302") => Some("Controlador 3D"),
        Some("0401") => Some("Controlador de áudio"),
        Some("0403") => Some("Áudio de alta definição"),
        Some("0C03") => Some("Controlador USB"),
        Some("0C05") => Some("Controlador SMBus"),
        Some("0D11") => Some("Controlador Bluetooth"),
        Some("0D40") => Some("Controlador sem fio"),
        Some("0604") => Some("Ponte PCI"),
        _ => None,
    };
    if exact.is_some() {
        return exact;
    }
    Some(match code.get(..2)? {
        "01" => "Controlador de armazenamento",
        "02" => "Controlador de rede",
        "03" => "Controlador de vídeo",
        "04" => "Controlador multimídia",
        "05" => "Controlador de memória",
        "06" => "Ponte do chipset",
        "07" => "Controlador de comunicação",
        "08" => "Periférico do sistema",
        "09" => "Controlador de entrada",
        "0C" => "Barramento serial",
        "0D" => "Controlador sem fio",
        "11" => "Processamento de sinais",
        "12" => "Acelerador de processamento",
        _ => return None,
    })
}

fn lookup(table: &[(&'static str, &'static str)], id: &str) -> Option<&'static str> {
    table
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(id))
        .map(|(_, v)| *v)
}

/// What the hardware IDs of one device say about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub vendor: String,
    /// `PCI 10DE:2684 · Controlador de vídeo`; the class part is left out when unknown.
    pub hint: String,
}

/// Value of a `KEY_xxxx` token inside an ID such as `PCI\VEN_10DE&DEV_2684&SUBSYS_...`.
fn token<'a>(id: &'a str, key: &str) -> Option<&'a str> {
    let start = id.find(key)? + key.len();
    let rest = &id[start..];
    let end = rest.find(['&', '\\']).unwrap_or(rest.len());
    let val = &rest[..end];
    (!val.is_empty()).then_some(val)
}

fn class_of(ids: &[String]) -> Option<&'static str> {
    ids.iter()
        .filter_map(|id| id.strip_prefix("PCI\\CC_"))
        .find_map(|cc| pci_class(cc.split(['&', '\\']).next().unwrap_or(cc)))
}

/// Reads the first PCI or USB ID found in the hardware IDs, then the compatible ones, because
/// class-only matches (`PCI\CC_0300`) carry no vendor.
pub fn identify(hardware: &[String], compatible: &[String]) -> Option<Identity> {
    let class = class_of(hardware).or_else(|| class_of(compatible));
    for id in hardware.iter().chain(compatible) {
        let upper = id.to_ascii_uppercase();
        if let Some(rest) = upper.strip_prefix("PCI\\") {
            let (Some(ven), dev) = (token(rest, "VEN_"), token(rest, "DEV_")) else {
                continue;
            };
            let vendor = lookup(PCI_VENDORS, ven).unwrap_or("Fornecedor desconhecido");
            let mut hint = format!(
                "PCI {ven}{}",
                dev.map(|d| format!(":{d}")).unwrap_or_default()
            );
            if let Some(c) = class {
                hint.push_str(&format!(" · {c}"));
            }
            return Some(Identity {
                vendor: vendor.into(),
                hint,
            });
        }
        if let Some(rest) = upper.strip_prefix("USB\\") {
            let (Some(vid), pid) = (token(rest, "VID_"), token(rest, "PID_")) else {
                continue;
            };
            let vendor = lookup(USB_VENDORS, vid).unwrap_or("Fornecedor desconhecido");
            let hint = format!(
                "USB {vid}{}",
                pid.map(|p| format!(":{p}")).unwrap_or_default()
            );
            return Some(Identity {
                vendor: vendor.into(),
                hint,
            });
        }
    }
    class.map(|c| Identity {
        vendor: String::new(),
        hint: c.into(),
    })
}

/// Fills `vendor` and `hint` from the IDs; `name` and `manufacturer` stay as Windows reported them.
pub fn annotate(d: &mut Device) {
    if let Some(id) = identify(&d.hardware_ids, &d.compatible_ids) {
        d.vendor = id.vendor;
        d.hint = id.hint;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn reads_pci_vendor_device_and_class() {
        let id = identify(
            &v(&["PCI\\VEN_10DE&DEV_2684&SUBSYS_00001043&REV_A1"]),
            &v(&["PCI\\VEN_10DE&DEV_2684", "PCI\\CC_030000", "PCI\\CC_0300"]),
        )
        .unwrap();
        assert_eq!(id.vendor, "NVIDIA");
        assert_eq!(id.hint, "PCI 10DE:2684 · Controlador de vídeo");
    }

    #[test]
    fn reads_usb_ids_case_insensitively() {
        let id = identify(&v(&["usb\\vid_046d&pid_c52b&rev_1210"]), &[]).unwrap();
        assert_eq!(id.vendor, "Logitech");
        assert_eq!(id.hint, "USB 046D:C52B");
    }

    #[test]
    fn unknown_vendor_is_labelled_not_guessed() {
        let id = identify(&v(&["PCI\\VEN_ABCD&DEV_0001"]), &[]).unwrap();
        assert_eq!(id.vendor, "Fornecedor desconhecido");
        assert_eq!(id.hint, "PCI ABCD:0001");
    }

    #[test]
    fn class_only_ids_give_a_hint_without_a_vendor() {
        let id = identify(&[], &v(&["PCI\\CC_0280"])).unwrap();
        assert!(id.vendor.is_empty());
        assert_eq!(id.hint, "Placa de rede sem fio");
    }

    #[test]
    fn falls_back_to_the_base_class() {
        assert_eq!(pci_class("0C0A00"), Some("Barramento serial"));
        assert_eq!(pci_class("FF00"), None);
        // Compatible IDs carry only the 2-digit base class.
        assert_eq!(pci_class("03"), Some("Controlador de vídeo"));
    }

    #[test]
    fn non_pci_usb_ids_are_ignored() {
        assert!(identify(&v(&["ACPI\\PNP0A08", "ROOT\\SYSTEM"]), &[]).is_none());
        assert!(identify(&v(&["PCI\\VEN_&DEV_1"]), &[]).is_none());
    }

    #[test]
    fn annotate_sets_the_device_fields() {
        let mut d = Device {
            hardware_ids: v(&["PCI\\VEN_8086&DEV_A0F0"]),
            ..Default::default()
        };
        annotate(&mut d);
        assert_eq!(d.vendor, "Intel");
        assert_eq!(d.hint, "PCI 8086:A0F0");
    }

    #[test]
    fn tables_have_unique_well_formed_ids() {
        for table in [PCI_VENDORS, USB_VENDORS] {
            let mut seen = std::collections::HashSet::new();
            for (id, name) in table {
                assert!(
                    id.len() == 4 && id.chars().all(|c| c.is_ascii_hexdigit()),
                    "{id}"
                );
                assert!(id.chars().all(|c| !c.is_ascii_lowercase()), "{id}");
                assert!(!name.is_empty());
                assert!(seen.insert(*id), "duplicate vendor id {id}");
            }
        }
    }
}
