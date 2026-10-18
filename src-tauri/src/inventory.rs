use crate::error::Result;
use crate::model::Device;
use crate::ps::{run_script, scripts};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub fn parse_devices(json: &str) -> Result<Vec<Device>> {
    let json = json.trim();
    if json.is_empty() {
        return Ok(Vec::new());
    }
    let mut devices: Vec<Device> = serde_json::from_str(json)?;
    devices.retain(|d| !d.instance_id.is_empty());
    devices.sort_by(|a, b| a.class.cmp(&b.class).then_with(|| a.name.cmp(&b.name)));
    Ok(devices)
}

pub fn collect(cancel: &AtomicBool) -> Result<Vec<Device>> {
    parse_devices(&run_script(
        "inventário",
        scripts::INVENTORY,
        None,
        Duration::from_secs(120),
        cancel,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_device_with_and_without_a_driver() {
        let json = r#"[
          {"instanceId":"PCI\\VEN_1&DEV_2\\3","name":"Display Adapter","class":"Display","manufacturer":"Acme",
           "provider":"Acme","version":"31.0.1.2","date":"2024-05-06","inf":"oem7.inf",
           "hardwareIds":["PCI\\VEN_1&DEV_2"],"compatibleIds":["PCI\\CC_0300"],"signed":true,"signer":"Microsoft","problemCode":0},
          {"instanceId":"USB\\VID_9&PID_9\\1","name":"","class":"","manufacturer":"","provider":"","version":"",
           "date":null,"inf":"","hardwareIds":[],"compatibleIds":[],"signed":false,"signer":"","problemCode":28}
        ]"#;
        let devices = parse_devices(json).unwrap();
        assert_eq!(devices.len(), 2);
        let broken = devices.iter().find(|d| d.problem_code == 28).unwrap();
        assert!(broken.date.is_none() && broken.inf.is_empty());
        let ok = devices.iter().find(|d| d.inf == "oem7.inf").unwrap();
        assert_eq!(ok.hardware_ids, vec!["PCI\\VEN_1&DEV_2"]);
    }

    #[test]
    fn empty_output_means_no_devices() {
        assert!(parse_devices("  ").unwrap().is_empty());
        assert!(parse_devices("[]").unwrap().is_empty());
    }

    #[test]
    fn rows_without_an_instance_id_are_dropped() {
        let devices = parse_devices(r#"[{"instanceId":"","name":"ghost"}]"#).unwrap();
        assert!(devices.is_empty());
    }

    #[test]
    fn malformed_json_is_an_error_not_an_empty_list() {
        assert!(parse_devices("{not json").is_err());
    }
}
