use crate::model::{Candidate, Decision, Device, Offer};
use std::cmp::Ordering;

/// Dotted numeric version; any non-numeric part makes it unparseable on purpose,
/// because vendor strings like `R470 U3` cannot be ordered safely.
pub fn parse_version(s: &str) -> Option<Vec<u64>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    s.split('.').map(|p| p.parse::<u64>().ok()).collect()
}

pub fn cmp_version(a: &[u64], b: &[u64]) -> Ordering {
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| {
            a.get(i)
                .copied()
                .unwrap_or(0)
                .cmp(&b.get(i).copied().unwrap_or(0))
        })
        .find(|o| *o != Ordering::Equal)
        .unwrap_or(Ordering::Equal)
}

/// Windows Update has no version field for drivers; titles end with it
/// (`Intel - Net - 22.250.1.2`), so take the last dotted token.
pub fn version_from_title(title: &str) -> Option<String> {
    title
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | '(' | ')' | '[' | ']'))
        .rev()
        .find(|t| t.contains('.') && parse_version(t).is_some())
        .map(str::to_string)
}

fn has_driver(d: &Device) -> bool {
    !d.version.trim().is_empty() || !d.inf.trim().is_empty()
}

pub fn decide(device: &Device, cand: &Candidate) -> (Decision, String) {
    if !has_driver(device) {
        // Only Config Manager's "no driver installed" (28) proves the device really has none; an
        // empty driver row can also be a gap in WMI, and replacing a driver we cannot back up is
        // not worth the risk.
        return if device.problem_code == 28 {
            (Decision::Newer, "dispositivo sem driver instalado".into())
        } else {
            (
                Decision::Unknown,
                "driver instalado não identificado".into(),
            )
        };
    }
    let iv = parse_version(&device.version);
    let cv = cand.version.as_deref().and_then(parse_version);
    if let (Some(iv), Some(cv)) = (iv, cv) {
        return match cmp_version(&cv, &iv) {
            Ordering::Greater => (Decision::Newer, "versão maior que a instalada".into()),
            Ordering::Equal => (Decision::Same, "mesma versão".into()),
            Ordering::Less => (Decision::Older, "versão menor que a instalada".into()),
        };
    }
    match (&cand.date, &device.date) {
        (Some(c), Some(i)) => match c.cmp(i) {
            Ordering::Greater => (
                Decision::Newer,
                "data mais recente (versão não comparável)".into(),
            ),
            Ordering::Equal => (Decision::Same, "mesma data".into()),
            Ordering::Less => (
                Decision::Older,
                "data mais antiga (versão não comparável)".into(),
            ),
        },
        _ => (Decision::Unknown, "sem versão ou data comparável".into()),
    }
}

fn id_matches(ids: &[String], wanted: &str) -> bool {
    ids.iter().any(|i| i.eq_ignore_ascii_case(wanted))
}

/// Pair each candidate with the devices it targets. Candidates matching no device are dropped:
/// Windows Update lists drivers for hardware that is absent or already served.
pub fn build_offers(devices: &[Device], candidates: &[Candidate]) -> Vec<Offer> {
    candidates
        .iter()
        .filter_map(|c| {
            let hw: Vec<&Device> = devices
                .iter()
                .filter(|d| id_matches(&d.hardware_ids, &c.hardware_id))
                .collect();
            let (matched, via_compatible) = if hw.is_empty() {
                (
                    devices
                        .iter()
                        .filter(|d| id_matches(&d.compatible_ids, &c.hardware_id))
                        .collect(),
                    true,
                )
            } else {
                (hw, false)
            };
            // The weakest device decides: one device that would downgrade blocks the whole offer.
            let (decision, reason) = matched
                .iter()
                .map(|d| decide(d, c))
                .min_by_key(|(d, _)| rank(*d))?;
            Some(Offer {
                candidate: c.clone(),
                device_ids: matched.iter().map(|d| d.instance_id.clone()).collect(),
                fixes_problem: matched.iter().any(|d| d.problem_code != 0),
                decision,
                reason,
                via_compatible,
            })
        })
        .collect()
}

fn rank(d: Decision) -> u8 {
    match d {
        Decision::Older => 0,
        Decision::Unknown => 1,
        Decision::Same => 2,
        Decision::Newer => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(id: &str, hw: &str, version: &str, date: Option<&str>) -> Device {
        Device {
            instance_id: id.into(),
            hardware_ids: vec![hw.into()],
            version: version.into(),
            inf: if version.is_empty() {
                String::new()
            } else {
                "oem1.inf".into()
            },
            date: date.map(Into::into),
            ..Default::default()
        }
    }

    fn cand(hw: &str, version: Option<&str>, date: Option<&str>) -> Candidate {
        Candidate {
            hardware_id: hw.into(),
            version: version.map(Into::into),
            date: date.map(Into::into),
            ..Default::default()
        }
    }

    #[test]
    fn versions_compare_numerically_not_lexically() {
        let a = parse_version("10.0.9").unwrap();
        let b = parse_version("10.0.10").unwrap();
        assert_eq!(cmp_version(&b, &a), Ordering::Greater);
        assert_eq!(
            cmp_version(
                &parse_version("1.2").unwrap(),
                &parse_version("1.2.0.0").unwrap()
            ),
            Ordering::Equal
        );
    }

    #[test]
    fn non_numeric_versions_are_not_ordered() {
        assert!(parse_version("R470 U3").is_none());
        assert!(parse_version("1.2.x").is_none());
        assert!(parse_version("").is_none());
    }

    #[test]
    fn title_version_is_the_last_dotted_token() {
        assert_eq!(
            version_from_title("Intel - Net - 22.250.1.2"),
            Some("22.250.1.2".into())
        );
        assert_eq!(
            version_from_title("Realtek Audio (6.0.9.1) extra"),
            Some("6.0.9.1".into())
        );
        assert_eq!(version_from_title("Acme Widget 3"), None);
    }

    #[test]
    fn a_downgrade_is_never_newer() {
        let (d, _) = decide(
            &dev("a", "X", "2.0.0.0", Some("2024-01-01")),
            &cand("X", Some("1.9.9.9"), Some("2025-01-01")),
        );
        assert_eq!(d, Decision::Older);
    }

    #[test]
    fn falls_back_to_date_only_when_a_version_is_unparseable() {
        let (d, _) = decide(
            &dev("a", "X", "R470", Some("2023-01-01")),
            &cand("X", Some("1.0"), Some("2024-01-01")),
        );
        assert_eq!(d, Decision::Newer);
        let (d, _) = decide(
            &dev("a", "X", "R470", None),
            &cand("X", Some("1.0"), Some("2024-01-01")),
        );
        assert_eq!(d, Decision::Unknown);
    }

    #[test]
    fn device_reported_as_driverless_accepts_any_candidate() {
        let mut d = dev("a", "X", "", None);
        d.problem_code = 28;
        let (decision, _) = decide(&d, &cand("X", None, None));
        assert_eq!(decision, Decision::Newer);
    }

    #[test]
    fn an_empty_driver_row_without_problem_28_is_not_assumed_driverless() {
        let (decision, _) = decide(&dev("a", "X", "", None), &cand("X", Some("1.0"), None));
        assert_eq!(decision, Decision::Unknown);
    }

    #[test]
    fn hardware_id_match_ignores_case_and_beats_compatible() {
        let mut d = dev("a", "PCI\\VEN_8086&DEV_1", "1.0", Some("2020-01-01"));
        d.compatible_ids = vec!["PCI\\CC_0300".into()];
        let offers = build_offers(&[d], &[cand("pci\\ven_8086&dev_1", Some("2.0"), None)]);
        assert_eq!(offers.len(), 1);
        assert!(!offers[0].via_compatible);
        assert!(offers[0].installable());
    }

    #[test]
    fn compatible_id_match_is_flagged() {
        let mut d = dev("a", "PCI\\VEN_8086&DEV_1", "1.0", Some("2020-01-01"));
        d.compatible_ids = vec!["PCI\\CC_0300".into()];
        let offers = build_offers(&[d], &[cand("PCI\\CC_0300", Some("2.0"), None)]);
        assert!(offers[0].via_compatible);
    }

    #[test]
    fn candidates_for_absent_hardware_are_dropped() {
        let offers = build_offers(
            &[dev("a", "X", "1.0", None)],
            &[cand("Y", Some("2.0"), None)],
        );
        assert!(offers.is_empty());
    }

    #[test]
    fn one_downgrading_device_blocks_the_offer() {
        let devices = [dev("a", "X", "1.0", None), dev("b", "X", "3.0", None)];
        let offers = build_offers(&devices, &[cand("X", Some("2.0"), None)]);
        assert_eq!(offers[0].decision, Decision::Older);
        assert!(!offers[0].installable());
        assert_eq!(offers[0].device_ids, vec!["a", "b"]);
    }

    #[test]
    fn problem_devices_are_flagged() {
        let mut d = dev("a", "X", "", None);
        d.problem_code = 28;
        let offers = build_offers(&[d], &[cand("X", Some("1.0"), None)]);
        assert!(offers[0].fixes_problem);
    }
}
