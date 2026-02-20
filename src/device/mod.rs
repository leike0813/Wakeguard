pub mod identity;
pub mod powercfg;

use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceSource {
    PowerCfg,
    TestFixture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceClass {
    Keyboard,
    Mouse,
    HumanInterfaceDevice,
    NetworkAdapter,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeDeviceRaw {
    pub name: String,
    pub source: DeviceSource,
    pub observed_at: SystemTime,
    pub system_id: Option<String>,
    pub hardware_id: Option<String>,
    pub serial_number: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeDevice {
    pub display_name: String,
    pub stable_id: String,
    pub class: DeviceClass,
    pub identity_confidence: IdentityConfidence,
}

impl WakeDevice {
    pub fn is_low_confidence(&self) -> bool {
        self.identity_confidence == IdentityConfidence::Low
    }
}
