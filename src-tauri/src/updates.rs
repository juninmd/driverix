use crate::error::{Error, Result};
use crate::guard::is_guid;
use crate::matching::version_from_title;
use crate::model::Candidate;
use crate::ps::{run_script, scripts};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub fn parse_candidates(json: &str) -> Result<Vec<Candidate>> {
    let json = json.trim();
    if json.is_empty() {
        return Ok(Vec::new());
    }
    // serde's own message is English and says where the JSON broke; the user needs the outcome instead.
    let mut list: Vec<Candidate> = serde_json::from_str(json)
        .map_err(|_| Error::Parse("a busca não devolveu uma lista válida".into()))?;
    // Only well-formed GUIDs may become install targets later.
    list.retain(|c| is_guid(&c.update_id) && !c.hardware_id.is_empty());
    for c in &mut list {
        c.version = version_from_title(&c.title);
    }
    Ok(list)
}

/// Asks Windows Update for pending driver updates. Takes about a minute on a cold machine.
pub fn scan(cancel: &AtomicBool) -> Result<Vec<Candidate>> {
    let out = run_script(
        "busca no Windows Update",
        scripts::SCAN_UPDATES,
        None,
        Duration::from_secs(600),
        cancel,
    )?;
    parse_candidates(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "0b9a4d52-3f3c-4a49-9d8e-2f6f4d1c7a10";

    #[test]
    fn a_malformed_search_result_is_reported_in_portuguese_without_serde_text() {
        let err = parse_candidates("not json").unwrap_err().to_string();
        assert_eq!(
            err,
            "dados inválidos (a busca não devolveu uma lista válida)"
        );
    }

    fn row(id: &str, hw: &str, title: &str) -> String {
        format!(
            r#"{{"updateId":"{id}","revision":200,"title":"{title}","class":"Net","manufacturer":"Acme","provider":"Acme",
            "model":"Acme NIC","hardwareId":"{hw}","date":"2024-01-02","sizeBytes":1048576,"rebootRequired":false}}"#
        )
    }

    #[test]
    fn extracts_the_version_from_the_title() {
        let c = parse_candidates(&format!(
            "[{}]",
            // Doubled backslashes: the row is embedded in JSON, where `\` must be escaped.
            row(GOOD, "PCI\\\\VEN_1&DEV_2", "Acme - Net - 22.250.1.2")
        ))
        .unwrap();
        assert_eq!(c[0].version.as_deref(), Some("22.250.1.2"));
        assert_eq!(c[0].size_bytes, 1_048_576);
        assert_eq!(c[0].hardware_id, "PCI\\VEN_1&DEV_2");
    }

    #[test]
    fn drops_rows_with_a_malformed_update_id() {
        let json = format!(
            "[{},{}]",
            row("not-a-guid; calc", "X", "t 1.0"),
            row(GOOD, "X", "t 1.0")
        );
        let c = parse_candidates(&json).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].update_id, GOOD);
    }

    #[test]
    fn drops_rows_without_a_hardware_id() {
        assert!(parse_candidates(&format!("[{}]", row(GOOD, "", "t 1.0")))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn no_updates_is_an_empty_list() {
        assert!(parse_candidates("[]").unwrap().is_empty());
        assert!(parse_candidates("").unwrap().is_empty());
    }
}
