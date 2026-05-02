use crate::device::{
    DeviceClass, IdentityConfidence, WakeDevice, WakeDeviceRaw,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityMethod {
    VidPid,
    SystemId,
    NormalizedNameFallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedIdentity {
    pub stable_id: String,
    pub confidence: IdentityConfidence,
    pub method: IdentityMethod,
}

pub fn normalize_stable_id(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

pub fn resolve_identity(raw: &WakeDeviceRaw) -> ResolvedIdentity {
    if let Some(hardware_id) = raw.hardware_id.as_deref().filter(|s| !s.trim().is_empty()) {
        return ResolvedIdentity {
            stable_id: format!("vidpid:{}", normalize_stable_id(hardware_id)),
            confidence: IdentityConfidence::High,
            method: IdentityMethod::VidPid,
        };
    }

    if let Some(system_id) = raw.system_id.as_deref().filter(|s| !s.trim().is_empty()) {
        return ResolvedIdentity {
            stable_id: format!("sys:{}", normalize_stable_id(system_id)),
            confidence: IdentityConfidence::Medium,
            method: IdentityMethod::SystemId,
        };
    }

    ResolvedIdentity {
        stable_id: format!("name:{}", normalize_stable_id(&raw.name)),
        confidence: IdentityConfidence::Low,
        method: IdentityMethod::NormalizedNameFallback,
    }
}

pub fn classify_device_class(device_name: &str) -> DeviceClass {
    let normalized = device_name.to_ascii_lowercase();
    if normalized.contains("keyboard") {
        return DeviceClass::Keyboard;
    }
    if normalized.contains("mouse") {
        return DeviceClass::Mouse;
    }
    if normalized.contains("human interface device") || normalized.contains("hid") {
        return DeviceClass::HumanInterfaceDevice;
    }
    if normalized.contains("ethernet")
        || normalized.contains("network")
        || normalized.contains("adapter")
        || normalized.contains("pci")
    {
        return DeviceClass::NetworkAdapter;
    }
    DeviceClass::Unknown
}

pub fn build_device(raw: WakeDeviceRaw) -> WakeDevice {
    let identity = resolve_identity(&raw);
    let member_name = raw.member_name.clone().unwrap_or_else(|| raw.name.clone());
    WakeDevice {
        display_name: raw.name.clone(),
        stable_id: identity.stable_id,
        member_names: vec![member_name],
        class: classify_device_class(&raw.name),
        identity_confidence: identity.confidence,
    }
}

pub fn build_device_strict(raw: WakeDeviceRaw) -> Option<WakeDevice> {
    let identity = resolve_identity(&raw);
    match identity.method {
        IdentityMethod::VidPid => {}
        IdentityMethod::SystemId if is_strict_system_identity(&identity.stable_id) => {}
        _ => return None,
    }
    let member_name = raw.member_name.clone().unwrap_or_else(|| raw.name.clone());

    Some(WakeDevice {
        display_name: raw.name.clone(),
        stable_id: identity.stable_id,
        member_names: vec![member_name],
        class: classify_device_class(&raw.name),
        identity_confidence: identity.confidence,
    })
}

fn is_strict_system_identity(stable_id: &str) -> bool {
    let normalized = stable_id.to_ascii_lowercase();
    normalized.starts_with("sys:ven_") && normalized.contains("&dev_")
}

#[cfg(test)]
mod tests {
    use super::{build_device_strict, classify_device_class, resolve_identity, IdentityMethod};
    use crate::device::{DeviceClass, DeviceSource, IdentityConfidence, WakeDeviceRaw};
    use std::time::SystemTime;

    fn raw_with_name(name: &str) -> WakeDeviceRaw {
        WakeDeviceRaw {
            name: name.to_string(),
            member_name: None,
            source: DeviceSource::TestFixture,
            observed_at: SystemTime::now(),
            system_id: None,
            hardware_id: None,
            serial_number: None,
        }
    }

    #[test]
    fn vid_pid_has_highest_priority() {
        let mut raw = raw_with_name("Keyboard");
        raw.system_id = Some("PCI\\VEN_8086".to_string());
        raw.hardware_id = Some("VID_1234&PID_8888".to_string());
        raw.serial_number = Some("SERIAL".to_string());

        let resolved = resolve_identity(&raw);
        assert_eq!(resolved.method, IdentityMethod::VidPid);
        assert_eq!(resolved.confidence, IdentityConfidence::High);
        assert_eq!(resolved.stable_id, "vidpid:vid_1234&pid_8888");
    }

    #[test]
    fn system_id_used_when_vid_pid_missing() {
        let mut raw = raw_with_name("Mouse");
        raw.system_id = Some("PCI\\VEN_8086".to_string());

        let resolved = resolve_identity(&raw);
        assert_eq!(resolved.method, IdentityMethod::SystemId);
        assert_eq!(resolved.confidence, IdentityConfidence::Medium);
    }

    #[test]
    fn fallback_to_name_for_low_confidence() {
        let raw = raw_with_name("Generic Input");
        let resolved = resolve_identity(&raw);
        assert_eq!(resolved.method, IdentityMethod::NormalizedNameFallback);
        assert_eq!(resolved.confidence, IdentityConfidence::Low);
        assert_eq!(resolved.stable_id, "name:generic input");
    }

    #[test]
    fn classify_hid_device_class() {
        let class = classify_device_class("USB Input Device (Human Interface Device)");
        assert_eq!(class, DeviceClass::HumanInterfaceDevice);
    }

    #[test]
    fn strict_builder_rejects_non_vid_pid_identity() {
        let mut raw = raw_with_name("Generic Input");
        raw.system_id = Some("PCI\\VEN_8086".to_string());
        assert!(build_device_strict(raw).is_none());
    }

    #[test]
    fn strict_builder_accepts_pci_ven_dev_identity() {
        let mut raw = raw_with_name("Intel Network Adapter");
        raw.system_id = Some("VEN_8086&DEV_1539".to_string());
        let device = build_device_strict(raw).expect("pci ven/dev should be accepted");
        assert_eq!(device.stable_id, "sys:ven_8086&dev_1539");
    }

    #[test]
    fn strict_builder_accepts_vid_pid_identity() {
        let mut raw = raw_with_name("HID Keyboard Device");
        raw.hardware_id = Some("VID_046D&PID_C52B".to_string());
        let device = build_device_strict(raw).expect("vid/pid device should be accepted");
        assert_eq!(device.stable_id, "vidpid:vid_046d&pid_c52b");
    }
}
