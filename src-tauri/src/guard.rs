//! Checks that run before anything on the machine changes. The webview only ever sends update
//! IDs and backup IDs; paths, URLs and INF names are never taken from it.

use crate::error::{Error, Result};
use crate::model::Offer;
use std::collections::HashSet;

pub const MAX_BATCH: usize = 25;

pub fn is_guid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5
        && parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(p, n)| p.len() == n && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Third-party packages land in the driver store as `oem<N>.inf`; inbox INFs cannot be exported.
pub fn is_oem_inf(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    lower
        .strip_prefix("oem")
        .and_then(|r| r.strip_suffix(".inf"))
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Backup IDs are unix seconds; digits only keeps them from escaping the backups directory.
pub fn is_backup_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 12 && s.bytes().all(|b| b.is_ascii_digit())
}

/// Resolves requested update IDs against the offers of the current scan.
pub fn select_offers(requested: &[String], offers: &[Offer], elevated: bool) -> Result<Vec<Offer>> {
    if !elevated {
        return Err(Error::Denied(
            "Instalar drivers exige executar como administrador".into(),
        ));
    }
    if requested.is_empty() {
        return Err(Error::Denied("Nenhuma atualização selecionada".into()));
    }
    if requested.len() > MAX_BATCH {
        return Err(Error::Denied(format!(
            "No máximo {MAX_BATCH} atualizações por vez"
        )));
    }
    let mut seen = HashSet::new();
    let mut claimed_devices = HashSet::new();
    let mut picked = Vec::with_capacity(requested.len());
    for id in requested {
        if !is_guid(id) || !seen.insert(id.to_ascii_lowercase()) {
            return Err(Error::Denied(format!(
                "Identificador inválido ou repetido: {id}"
            )));
        }
        let offer = offers
            .iter()
            .find(|o| o.candidate.update_id.eq_ignore_ascii_case(id))
            .ok_or_else(|| {
                Error::Denied("Atualização não consta na última busca; busque de novo".into())
            })?;
        if !offer.installable() {
            return Err(Error::Denied(format!(
                "'{}' não é mais novo que o driver instalado",
                offer.candidate.title
            )));
        }
        // Two updates for one device would race inside Windows Update and blur the backup.
        if !offer
            .device_ids
            .iter()
            .all(|d| claimed_devices.insert(d.as_str()))
        {
            return Err(Error::Denied(
                "Duas atualizações para o mesmo dispositivo; instale uma por vez".into(),
            ));
        }
        picked.push(offer.clone());
    }
    Ok(picked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Candidate, Decision};

    const ID: &str = "0b9a4d52-3f3c-4a49-9d8e-2f6f4d1c7a10";

    fn offer(id: &str, decision: Decision) -> Offer {
        Offer {
            candidate: Candidate {
                update_id: id.into(),
                title: "t".into(),
                ..Default::default()
            },
            device_ids: vec![],
            decision,
            reason: String::new(),
            fixes_problem: false,
            via_compatible: false,
        }
    }

    #[test]
    fn guid_shape_is_strict() {
        assert!(is_guid(ID));
        assert!(!is_guid("0b9a4d52-3f3c-4a49-9d8e-2f6f4d1c7a1"));
        assert!(!is_guid("0b9a4d52-3f3c-4a49-9d8e-2f6f4d1c7a1g"));
        assert!(!is_guid("0b9a4d52-3f3c-4a49-9d8e-2f6f4d1c7a10; rm"));
    }

    #[test]
    fn only_numbered_oem_infs_are_exportable() {
        assert!(is_oem_inf("oem12.inf") && is_oem_inf("OEM3.INF"));
        assert!(
            !is_oem_inf("netrasa.inf") && !is_oem_inf("oem.inf") && !is_oem_inf("oem1.inf.bak")
        );
        assert!(!is_oem_inf("..\\oem1.inf"));
    }

    #[test]
    fn backup_ids_cannot_traverse() {
        assert!(is_backup_id("1760800000"));
        assert!(
            !is_backup_id("..")
                && !is_backup_id("17\\60")
                && !is_backup_id("")
                && !is_backup_id("1/2")
        );
    }

    #[test]
    fn refuses_when_not_elevated() {
        let r = select_offers(&[ID.into()], &[offer(ID, Decision::Newer)], false);
        assert!(matches!(r, Err(Error::Denied(_))));
    }

    #[test]
    fn refuses_ids_that_the_scan_never_produced() {
        let other = "11111111-2222-3333-4444-555555555555";
        assert!(select_offers(&[other.into()], &[offer(ID, Decision::Newer)], true).is_err());
    }

    #[test]
    fn refuses_downgrades_and_same_versions() {
        for d in [Decision::Older, Decision::Same, Decision::Unknown] {
            assert!(select_offers(&[ID.into()], &[offer(ID, d)], true).is_err());
        }
    }

    #[test]
    fn refuses_duplicates_and_empty_and_oversized_batches() {
        let offers = [offer(ID, Decision::Newer)];
        assert!(select_offers(&[ID.into(), ID.to_uppercase()], &offers, true).is_err());
        assert!(select_offers(&[], &offers, true).is_err());
        let many = vec![ID.to_string(); MAX_BATCH + 1];
        assert!(select_offers(&many, &offers, true).is_err());
    }

    #[test]
    fn refuses_two_offers_for_the_same_device() {
        let other = "11111111-2222-3333-4444-555555555555";
        let mut a = offer(ID, Decision::Newer);
        let mut b = offer(other, Decision::Newer);
        a.device_ids = vec!["PCI_X".into()];
        b.device_ids = vec!["PCI_X".into()];
        assert!(select_offers(&[ID.into(), other.into()], &[a, b], true).is_err());
    }

    #[test]
    fn accepts_a_newer_offer_case_insensitively() {
        let r = select_offers(&[ID.to_uppercase()], &[offer(ID, Decision::Newer)], true).unwrap();
        assert_eq!(r.len(), 1);
    }
}
