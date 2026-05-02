use crate::device::powercfg::{normalize_ven_dev_fragment, normalize_vid_pid_fragment};
use crate::error::WakeguardError;
use std::collections::{HashMap, HashSet};
use std::mem::{size_of, zeroed};
use std::process::Command;
use windows::core::{GUID, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_Device_IDW, CM_Get_Parent, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
    SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW, SetupDiGetDevicePropertyW, HDEVINFO,
    SP_DEVINFO_DATA, CR_SUCCESS, DIGCF_ALLCLASSES, DIGCF_PRESENT,
};
use windows::Win32::Devices::Properties::{
    DEVPROPTYPE, DEVPKEY_Device_ContainerId, DEVPKEY_Device_HardwareIds, DEVPKEY_Device_Parent,
    DEVPKEY_NAME, DEVPROP_TYPE_GUID, DEVPROP_TYPE_STRING, DEVPROP_TYPE_STRING_LIST,
};
use windows::Win32::Foundation::{GetLastError, ERROR_INSUFFICIENT_BUFFER, ERROR_NO_MORE_ITEMS, HWND};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceNode {
    pub instance_id: String,
    pub parent_instance_id: Option<String>,
    pub container_id: Option<String>,
    pub display_name: Option<String>,
    pub hardware_ids: Vec<String>,
    pub hardware_id: Option<String>,
    pub system_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeLeafObservation {
    pub instance_id: String,
    pub display_name: String,
    pub wake_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedFamily {
    pub representative_name: Option<String>,
    pub hardware_id: Option<String>,
    pub system_id: Option<String>,
    pub lineage: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyResolutionError {
    pub stage: &'static str,
    pub lineage: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DeviceTopology {
    nodes: HashMap<String, DeviceNode>,
}

impl DeviceTopology {
    pub fn from_nodes(nodes: Vec<DeviceNode>) -> Self {
        let nodes = nodes
            .into_iter()
            .map(|node| (normalize_instance_id(&node.instance_id), node))
            .collect::<HashMap<_, _>>();
        Self { nodes }
    }

    pub fn resolve_family(&self, leaf_instance_id: &str) -> Result<ResolvedFamily, FamilyResolutionError> {
        let mut current_id = normalize_instance_id(leaf_instance_id);
        let mut visited = HashSet::new();
        let mut lineage = Vec::new();
        let mut fallback_system = None;

        loop {
            if !visited.insert(current_id.clone()) {
                return Err(FamilyResolutionError {
                    stage: "parent_chain_loop",
                    lineage,
                });
            }

            let Some(node) = self.nodes.get(&current_id) else {
                return Err(FamilyResolutionError {
                    stage: if lineage.is_empty() {
                        "missing_leaf"
                    } else {
                        "parent_chain_broken"
                    },
                    lineage,
                });
            };

            lineage.push(current_id.clone());

            if let Some(hardware_id) = node.hardware_id.clone() {
                return Ok(ResolvedFamily {
                    representative_name: node.display_name.clone(),
                    hardware_id: Some(hardware_id),
                    system_id: None,
                    lineage,
                });
            }

            if fallback_system.is_none() {
                fallback_system = node.system_id.clone().map(|system_id| ResolvedFamily {
                    representative_name: node.display_name.clone(),
                    hardware_id: None,
                    system_id: Some(system_id),
                    lineage: lineage.clone(),
                });
            }

            let Some(parent_id) = node.parent_instance_id.as_deref() else {
                break;
            };
            current_id = normalize_instance_id(parent_id);
        }

        fallback_system.ok_or(FamilyResolutionError {
            stage: "no_managed_identity",
            lineage,
        })
    }
}

pub fn snapshot_present_devices() -> Result<DeviceTopology, WakeguardError> {
    let device_info_set = unsafe {
        SetupDiGetClassDevsW(
            None,
            PCWSTR::null(),
            HWND::default(),
            DIGCF_ALLCLASSES | DIGCF_PRESENT,
        )
    }
    .map_err(|err| WakeguardError::Windows(format!("SetupDiGetClassDevsW failed: {err}")))?;
    let device_info_guard = DeviceInfoSetGuard(device_info_set);

    let mut nodes = Vec::new();
    let mut index = 0u32;
    loop {
        let mut device_info: SP_DEVINFO_DATA = unsafe { zeroed() };
        device_info.cbSize = size_of::<SP_DEVINFO_DATA>() as u32;

        let enumeration = unsafe {
            SetupDiEnumDeviceInfo(device_info_guard.0, index, &mut device_info)
        };
        if let Err(err) = enumeration {
            let code = unsafe { GetLastError() };
            if code == ERROR_NO_MORE_ITEMS {
                break;
            }
            return Err(WakeguardError::Windows(format!(
                "SetupDiEnumDeviceInfo failed at index {index}: {err}"
            )));
        }

        if let Some(node) = build_device_node(device_info_guard.0, &device_info)? {
            nodes.push(node);
        }
        index += 1;
    }

    Ok(DeviceTopology::from_nodes(nodes))
}

pub fn load_wake_leaf_observations() -> Result<Vec<WakeLeafObservation>, WakeguardError> {
    let script = r#"$OutputEncoding = [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding;
$nameByPnp = @{};
Get-CimInstance Win32_PnPEntity |
Where-Object { $_.Name -and $_.PNPDeviceID } |
ForEach-Object { $nameByPnp[$_.PNPDeviceID.ToUpper()] = $_.Name };
Get-CimInstance -Namespace root/wmi -Class MSPower_DeviceWakeEnable |
ForEach-Object {
    $pnp = ($_.InstanceName -replace '_[0-9]+$','').ToUpper();
    $name = $nameByPnp[$pnp];
    if ($name) {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($name);
        $hex = [System.BitConverter]::ToString($bytes).Replace('-','');
        "{0}`t{1}`t{2}" -f $hex, $pnp, ([int][bool]$_.Enable)
    }
}"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()?;

    if !output.status.success() {
        return Err(WakeguardError::CommandFailed {
            command: "powershell MSPower_DeviceWakeEnable topology lookup".to_string(),
            details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    parse_wake_leaf_observations(&String::from_utf8_lossy(&output.stdout))
}

fn build_device_node(
    device_info_set: HDEVINFO,
    device_info: &SP_DEVINFO_DATA,
) -> Result<Option<DeviceNode>, WakeguardError> {
    let instance_id = match read_device_instance_id(device_info_set, device_info) {
        Ok(Some(value)) => value,
        Ok(None) => return Ok(None),
        Err(err) => return Err(err),
    };

    let parent_instance_id = read_parent_instance_id(device_info_set, device_info)?;
    let container_id = read_guid_property(device_info_set, device_info, &DEVPKEY_Device_ContainerId)?;
    let display_name = read_string_property(device_info_set, device_info, &DEVPKEY_NAME)?;
    let hardware_ids = read_string_list_property(device_info_set, device_info, &DEVPKEY_Device_HardwareIds)?
        .unwrap_or_default();
    let hardware_id = infer_vid_pid(&instance_id, &hardware_ids);
    let system_id = infer_ven_dev(&instance_id, &hardware_ids);

    Ok(Some(DeviceNode {
        instance_id,
        parent_instance_id,
        container_id,
        display_name,
        hardware_ids,
        hardware_id,
        system_id,
    }))
}

fn infer_vid_pid(instance_id: &str, hardware_ids: &[String]) -> Option<String> {
    normalize_vid_pid_fragment(instance_id).or_else(|| {
        hardware_ids
            .iter()
            .find_map(|hardware_id| normalize_vid_pid_fragment(hardware_id))
    })
}

fn infer_ven_dev(instance_id: &str, hardware_ids: &[String]) -> Option<String> {
    normalize_ven_dev_fragment(instance_id).or_else(|| {
        hardware_ids
            .iter()
            .find_map(|hardware_id| normalize_ven_dev_fragment(hardware_id))
    })
}

fn read_parent_instance_id(
    device_info_set: HDEVINFO,
    device_info: &SP_DEVINFO_DATA,
) -> Result<Option<String>, WakeguardError> {
    if let Some(parent_id) = read_string_property(device_info_set, device_info, &DEVPKEY_Device_Parent)? {
        return Ok(Some(normalize_instance_id(&parent_id)));
    }

    let mut parent_devinst = 0u32;
    let result = unsafe { CM_Get_Parent(&mut parent_devinst, device_info.DevInst, 0) };
    if result != CR_SUCCESS {
        return Ok(None);
    }

    let mut buffer = vec![0u16; 512];
    let result = unsafe { CM_Get_Device_IDW(parent_devinst, buffer.as_mut_slice(), 0) };
    if result != CR_SUCCESS {
        return Ok(None);
    }
    Ok(wide_buffer_to_string(&buffer).map(|value| normalize_instance_id(&value)))
}

fn read_device_instance_id(
    device_info_set: HDEVINFO,
    device_info: &SP_DEVINFO_DATA,
) -> Result<Option<String>, WakeguardError> {
    let mut required = 0u32;
    let first_attempt = unsafe {
        SetupDiGetDeviceInstanceIdW(
            device_info_set,
            device_info,
            None,
            Some(&mut required),
        )
    };
    if first_attempt.is_ok() && required == 0 {
        return Ok(None);
    }

    let code = unsafe { GetLastError() };
    if first_attempt.is_err() && code != ERROR_INSUFFICIENT_BUFFER {
        return Ok(None);
    }

    let mut buffer = vec![0u16; required.max(512) as usize];
    unsafe {
        SetupDiGetDeviceInstanceIdW(
            device_info_set,
            device_info,
            Some(buffer.as_mut_slice()),
            Some(&mut required),
        )
    }
    .map_err(|err| WakeguardError::Windows(format!("SetupDiGetDeviceInstanceIdW failed: {err}")))?;

    Ok(wide_buffer_to_string(&buffer).map(|value| normalize_instance_id(&value)))
}

fn read_string_property(
    device_info_set: HDEVINFO,
    device_info: &SP_DEVINFO_DATA,
    key: &windows::Win32::Devices::Properties::DEVPROPKEY,
) -> Result<Option<String>, WakeguardError> {
    let Some((property_type, buffer)) = read_property_bytes(device_info_set, device_info, key)? else {
        return Ok(None);
    };
    if property_type != DEVPROP_TYPE_STRING {
        return Ok(None);
    }
    Ok(wide_buffer_to_string(&utf16_units_from_bytes(&buffer)))
}

fn read_string_list_property(
    device_info_set: HDEVINFO,
    device_info: &SP_DEVINFO_DATA,
    key: &windows::Win32::Devices::Properties::DEVPROPKEY,
) -> Result<Option<Vec<String>>, WakeguardError> {
    let Some((property_type, buffer)) = read_property_bytes(device_info_set, device_info, key)? else {
        return Ok(None);
    };
    if property_type != DEVPROP_TYPE_STRING_LIST {
        return Ok(None);
    }

    let units = utf16_units_from_bytes(&buffer);
    let mut values = Vec::new();
    let mut start = 0usize;
    for (idx, unit) in units.iter().enumerate() {
        if *unit != 0 {
            continue;
        }
        if idx > start {
            values.push(String::from_utf16_lossy(&units[start..idx]));
        }
        start = idx + 1;
    }
    Ok(Some(values))
}

fn read_guid_property(
    device_info_set: HDEVINFO,
    device_info: &SP_DEVINFO_DATA,
    key: &windows::Win32::Devices::Properties::DEVPROPKEY,
) -> Result<Option<String>, WakeguardError> {
    let Some((property_type, buffer)) = read_property_bytes(device_info_set, device_info, key)? else {
        return Ok(None);
    };
    if property_type != DEVPROP_TYPE_GUID || buffer.len() < size_of::<GUID>() {
        return Ok(None);
    }

    let guid = unsafe { std::ptr::read_unaligned(buffer.as_ptr() as *const GUID) };
    Ok(Some(format!("{guid:?}").to_ascii_lowercase()))
}

fn read_property_bytes(
    device_info_set: HDEVINFO,
    device_info: &SP_DEVINFO_DATA,
    key: &windows::Win32::Devices::Properties::DEVPROPKEY,
) -> Result<Option<(DEVPROPTYPE, Vec<u8>)>, WakeguardError> {
    let mut property_type = DEVPROPTYPE(0);
    let mut required = 0u32;
    let first_attempt = unsafe {
        SetupDiGetDevicePropertyW(
            device_info_set,
            device_info,
            key,
            &mut property_type,
            None,
            Some(&mut required),
            0,
        )
    };

    if first_attempt.is_ok() && required == 0 {
        return Ok(Some((property_type, Vec::new())));
    }

    let code = unsafe { GetLastError() };
    if first_attempt.is_err() && code != ERROR_INSUFFICIENT_BUFFER {
        return Ok(None);
    }

    let mut buffer = vec![0u8; required as usize];
    unsafe {
        SetupDiGetDevicePropertyW(
            device_info_set,
            device_info,
            key,
            &mut property_type,
            Some(buffer.as_mut_slice()),
            Some(&mut required),
            0,
        )
    }
    .map_err(|err| WakeguardError::Windows(format!("SetupDiGetDevicePropertyW failed: {err}")))?;

    Ok(Some((property_type, buffer)))
}

fn parse_wake_leaf_observations(raw: &str) -> Result<Vec<WakeLeafObservation>, WakeguardError> {
    let mut leaves = Vec::new();
    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let mut parts = line.splitn(3, '\t');
        let Some(display_name_hex) = parts.next() else {
            continue;
        };
        let Some(instance_id) = parts.next() else {
            continue;
        };
        let Some(enabled_raw) = parts.next() else {
            continue;
        };

        let Some(display_name) = decode_hex_utf8(display_name_hex) else {
            return Err(WakeguardError::InvalidConfig(format!(
                "failed to decode wake leaf display name: {display_name_hex}"
            )));
        };

        leaves.push(WakeLeafObservation {
            instance_id: normalize_instance_id(instance_id),
            display_name,
            wake_enabled: enabled_raw.trim() == "1",
        });
    }
    Ok(leaves)
}

fn utf16_units_from_bytes(bytes: &[u8]) -> Vec<u16> {
    bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect()
}

fn wide_buffer_to_string(buffer: &[u16]) -> Option<String> {
    let end = buffer.iter().position(|value| *value == 0).unwrap_or(buffer.len());
    if end == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..end]))
}

fn decode_hex_utf8(hex: &str) -> Option<String> {
    let trimmed = hex.trim();
    if trimmed.is_empty() || trimmed.len() % 2 != 0 {
        return None;
    }

    let mut bytes = Vec::with_capacity(trimmed.len() / 2);
    let chars = trimmed.as_bytes();
    let mut idx = 0usize;
    while idx < chars.len() {
        let pair = std::str::from_utf8(&chars[idx..idx + 2]).ok()?;
        let value = u8::from_str_radix(pair, 16).ok()?;
        bytes.push(value);
        idx += 2;
    }
    String::from_utf8(bytes).ok()
}

fn normalize_instance_id(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

struct DeviceInfoSetGuard(HDEVINFO);

impl Drop for DeviceInfoSetGuard {
    fn drop(&mut self) {
        let _ = unsafe { SetupDiDestroyDeviceInfoList(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_wake_leaf_observations, DeviceNode, DeviceTopology};

    #[test]
    fn parse_wake_leaf_observations_decodes_utf8_hex() {
        let raw = "e7aca6e590882048494420e6a087e58786\tHID\\\\VID_1234&PID_5678\\\\ABC\t1";
        let leaves = parse_wake_leaf_observations(raw).expect("parse should succeed");
        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0].display_name, "符合 HID 标准");
        assert!(leaves[0].wake_enabled);
    }

    #[test]
    fn resolve_family_prefers_vid_pid_before_parent_system_id() {
        let topology = DeviceTopology::from_nodes(vec![
            DeviceNode {
                instance_id: "HID\\LEAF".to_string(),
                parent_instance_id: Some("USB\\PARENT".to_string()),
                container_id: None,
                display_name: Some("HID Keyboard Device".to_string()),
                hardware_ids: vec![],
                hardware_id: Some("VID_304E&PID_000A".to_string()),
                system_id: None,
            },
            DeviceNode {
                instance_id: "USB\\PARENT".to_string(),
                parent_instance_id: Some("PCI\\ROOT".to_string()),
                container_id: None,
                display_name: Some("USB Device".to_string()),
                hardware_ids: vec![],
                hardware_id: None,
                system_id: Some("VEN_8086&DEV_1539".to_string()),
            },
        ]);

        let resolved = topology
            .resolve_family("hid\\leaf")
            .expect("leaf should resolve");
        assert_eq!(resolved.hardware_id.as_deref(), Some("VID_304E&PID_000A"));
        assert_eq!(resolved.representative_name.as_deref(), Some("HID Keyboard Device"));
    }

    #[test]
    fn resolve_family_falls_back_to_parent_system_id() {
        let topology = DeviceTopology::from_nodes(vec![
            DeviceNode {
                instance_id: "ACPI\\LEAF".to_string(),
                parent_instance_id: Some("PCI\\ROOT".to_string()),
                container_id: None,
                display_name: Some("Wake Adapter".to_string()),
                hardware_ids: vec![],
                hardware_id: None,
                system_id: None,
            },
            DeviceNode {
                instance_id: "PCI\\ROOT".to_string(),
                parent_instance_id: None,
                container_id: None,
                display_name: Some("Intel(R) I211 Gigabit Network Connection".to_string()),
                hardware_ids: vec![],
                hardware_id: None,
                system_id: Some("VEN_8086&DEV_1539".to_string()),
            },
        ]);

        let resolved = topology
            .resolve_family("ACPI\\LEAF")
            .expect("parent system id should resolve");
        assert_eq!(resolved.system_id.as_deref(), Some("VEN_8086&DEV_1539"));
        assert_eq!(
            resolved.representative_name.as_deref(),
            Some("Intel(R) I211 Gigabit Network Connection")
        );
    }

    #[test]
    fn resolve_family_reports_missing_identity_when_chain_has_no_candidate() {
        let topology = DeviceTopology::from_nodes(vec![DeviceNode {
            instance_id: "HID\\LEAF".to_string(),
            parent_instance_id: None,
            container_id: None,
            display_name: Some("Generic HID".to_string()),
            hardware_ids: vec![],
            hardware_id: None,
            system_id: None,
        }]);

        let err = topology
            .resolve_family("HID\\LEAF")
            .expect_err("should reject unmanaged chain");
        assert_eq!(err.stage, "no_managed_identity");
    }

    #[test]
    fn wake_leaf_observation_normalizes_instance_id() {
        let leaves = parse_wake_leaf_observations(
            "686964\tHid\\\\Vid_1234&pid_5678\\\\abc\t0",
        )
        .expect("parse should succeed");
        assert_eq!(leaves[0].instance_id, "HID\\\\VID_1234&PID_5678\\\\ABC");
    }
}
