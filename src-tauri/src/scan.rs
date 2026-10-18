use crate::error::Result;
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
}

/// Inventory first (seconds), then Windows Update (up to minutes); `phase` tells the UI which one runs.
pub fn scan_all(cancel: &AtomicBool, phase: impl Fn(&str)) -> Result<ScanResult> {
    phase("inventory");
    let devices = inventory::collect(cancel)?;
    phase("windows-update");
    let candidates = updates::scan(cancel)?;
    let offers = build_offers(&devices, &candidates);
    Ok(ScanResult { devices, offers })
}
