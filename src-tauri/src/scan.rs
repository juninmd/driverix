use crate::error::{Error, Result};
use crate::matching::build_offers;
use crate::model::{Device, Offer};
use crate::{inventory, updates};
use serde::Serialize;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub devices: Vec<Device>,
    pub offers: Vec<Offer>,
    /// Set when Windows Update failed: the devices are still listed, with no offers.
    pub updates_error: Option<String>,
}

/// Inventory first (seconds), then Windows Update (up to minutes); `phase` tells the UI which one runs.
pub fn scan_all(cancel: &AtomicBool, phase: impl Fn(&str)) -> Result<ScanResult> {
    phase("inventory");
    let devices = inventory::collect(cancel)?;
    phase("windows-update");
    let (offers, updates_error) = match updates::scan(cancel) {
        Ok(candidates) => (build_offers(&devices, &candidates), None),
        // A user cancel is not a failure of the update search; the caller reports it as cancelled.
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(e) => (Vec::new(), Some(e.to_string())),
    };
    Ok(ScanResult {
        devices,
        offers,
        updates_error,
    })
}
