use serde::{Deserialize, Serialize};

/// A present PnP device joined with its installed driver (if any).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Device {
    pub instance_id: String,
    pub name: String,
    pub class: String,
    pub manufacturer: String,
    pub provider: String,
    pub version: String,
    /// `YYYY-MM-DD`, so string order is date order.
    pub date: Option<String>,
    /// Driver store name (`oem12.inf`) or an inbox INF.
    pub inf: String,
    pub hardware_ids: Vec<String>,
    pub compatible_ids: Vec<String>,
    pub signed: bool,
    pub signer: String,
    /// Config Manager problem code; 0 means the device works.
    pub problem_code: u32,
    /// Vendor decoded from the hardware IDs (`vendors.rs`); empty when the IDs say nothing.
    pub vendor: String,
    /// `PCI 10DE:2684 · Controlador de vídeo`; what is left to identify a device without a name.
    pub hint: String,
}

/// A driver update published by Windows Update.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Candidate {
    pub update_id: String,
    pub revision: u32,
    pub title: String,
    pub class: String,
    pub manufacturer: String,
    pub provider: String,
    pub model: String,
    pub hardware_id: String,
    pub version: Option<String>,
    pub date: Option<String>,
    pub size_bytes: u64,
    pub reboot_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    /// Candidate is newer than what is installed, or the device has no driver.
    Newer,
    Same,
    /// Candidate would downgrade the device; never offered.
    Older,
    /// Neither side has a usable version or date.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub candidate: Candidate,
    pub device_ids: Vec<String>,
    pub decision: Decision,
    pub reason: String,
    pub fixes_problem: bool,
    /// Matched only through a compatible ID, not a hardware ID.
    pub via_compatible: bool,
}

impl Offer {
    /// The only offers the UI may let the user install.
    pub fn installable(&self) -> bool {
        self.decision == Decision::Newer
    }
}
