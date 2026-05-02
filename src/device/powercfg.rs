use crate::device::{
    identity::{build_device, build_device_strict},
    topology::{load_wake_leaf_observations, snapshot_present_devices, DeviceTopology, WakeLeafObservation},
    DeviceSource, WakeDevice, WakeDeviceRaw,
};
use crate::error::WakeguardError;
use std::collections::HashMap;
use std::process::Command;
use std::time::SystemTime;

const DEVICE_QUERY_WAKE_PROGRAMMABLE: &str = "wake_programmable";
const DEVICE_DISABLE_WAKE_ARG: &str = "-devicedisablewake";

#[derive(Debug, Clone, PartialEq, Eq)]
struct PowercfgEntry {
    display_name: String,
    normalized_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedPowercfgEntry {
    raw: WakeDeviceRaw,
    wake_enabled: bool,
}

pub fn list_wake_enabled_devices() -> Result<Vec<WakeDevice>, WakeguardError> {
    query_powercfg_devices(true)
}

pub fn list_wake_programmable_devices() -> Result<Vec<WakeDevice>, WakeguardError> {
    query_powercfg_devices(false)
}

fn query_powercfg_devices(only_enabled: bool) -> Result<Vec<WakeDevice>, WakeguardError> {
    let stdout = run_powercfg_query_utf8(
        DEVICE_QUERY_WAKE_PROGRAMMABLE,
        "powercfg -devicequery wake_programmable",
    )?;
    let powercfg_entries = parse_powercfg_entries(&stdout);
    let topology = snapshot_present_devices()?;
    let wake_leafs = load_wake_leaf_observations()?;

    let raws = resolve_powercfg_entries(powercfg_entries, &wake_leafs, &topology)
        .into_iter()
        .filter(|entry| !only_enabled || entry.wake_enabled)
        .map(|entry| entry.raw)
        .collect::<Vec<_>>();

    let devices = build_strict_devices(raws);
    Ok(aggregate_wake_devices(devices))
}

fn run_powercfg_query_utf8(query_target: &str, command_display: &str) -> Result<String, WakeguardError> {
    let script = format!(
        "$OutputEncoding = [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding;\r\npowercfg -devicequery {query_target}"
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let details = if output.stderr.is_empty() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        String::from_utf8_lossy(&output.stderr).trim().to_string()
    };
    Err(WakeguardError::CommandFailed {
        command: command_display.to_string(),
        details,
    })
}

pub fn disable_wake_for_device(device_name: &str) -> Result<(), WakeguardError> {
    let output = Command::new("powercfg")
        .arg(DEVICE_DISABLE_WAKE_ARG)
        .arg(device_name)
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    Err(WakeguardError::CommandFailed {
        command: format!("powercfg -devicedisablewake \"{device_name}\""),
        details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

#[cfg(test)]
pub(crate) fn parse_wake_armed_output(raw: &str) -> Vec<WakeDeviceRaw> {
    parse_powercfg_entries(raw)
        .into_iter()
        .map(|entry| WakeDeviceRaw {
            name: entry.display_name,
            member_name: None,
            source: DeviceSource::PowerCfg,
            observed_at: SystemTime::now(),
            system_id: None,
            hardware_id: None,
            serial_number: None,
        })
        .collect()
}

fn parse_powercfg_entries(raw: &str) -> Vec<PowercfgEntry> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !is_no_device_marker(line))
        .map(|line| PowercfgEntry {
            display_name: line.to_string(),
            normalized_name: normalize_device_name_for_match(line),
        })
        .collect()
}

fn resolve_powercfg_entries(
    entries: Vec<PowercfgEntry>,
    wake_leafs: &[WakeLeafObservation],
    topology: &DeviceTopology,
) -> Vec<ResolvedPowercfgEntry> {
    let mut grouped_entries: HashMap<String, Vec<PowercfgEntry>> = HashMap::new();
    for entry in entries {
        grouped_entries
            .entry(entry.normalized_name.clone())
            .or_default()
            .push(entry);
    }

    let mut grouped_leafs: HashMap<String, Vec<&WakeLeafObservation>> = HashMap::new();
    for leaf in wake_leafs {
        grouped_leafs
            .entry(normalize_device_name_for_match(&leaf.display_name))
            .or_default()
            .push(leaf);
    }

    let mut keys = grouped_entries.keys().cloned().collect::<Vec<_>>();
    keys.sort();

    let mut resolved = Vec::new();
    for key in keys {
        let Some(entry_group) = grouped_entries.remove(&key) else {
            continue;
        };
        let leaf_group = grouped_leafs.remove(&key).unwrap_or_default();

        if leaf_group.is_empty() {
            let missing_names = entry_group
                .iter()
                .map(|entry| entry.display_name.as_str())
                .collect::<Vec<_>>();
            tracing::warn!(
                normalized_name = key,
                powercfg_names = ?missing_names,
                "skipping wake devices because no matching wake leafs were found"
            );
            continue;
        }

        if entry_group.len() != leaf_group.len() {
            tracing::warn!(
                normalized_name = key,
                powercfg_count = entry_group.len(),
                wake_leaf_count = leaf_group.len(),
                powercfg_names = ?entry_group.iter().map(|entry| entry.display_name.as_str()).collect::<Vec<_>>(),
                wake_leaf_ids = ?leaf_group.iter().map(|leaf| leaf.instance_id.as_str()).collect::<Vec<_>>(),
                "powercfg and wake leaf counts differ; pairing by current observation order"
            );
        }

        for (entry, leaf) in entry_group.iter().zip(leaf_group.iter()) {
            if let Some(resolved_entry) = resolve_powercfg_entry(entry, leaf, topology) {
                resolved.push(resolved_entry);
            }
        }
    }

    for (normalized_name, leaf_group) in grouped_leafs {
        tracing::debug!(
            normalized_name,
            wake_leaf_ids = ?leaf_group.iter().map(|leaf| leaf.instance_id.as_str()).collect::<Vec<_>>(),
            "wake leaf group had no matching powercfg names"
        );
    }

    resolved
}

fn resolve_powercfg_entry(
    entry: &PowercfgEntry,
    leaf: &WakeLeafObservation,
    topology: &DeviceTopology,
) -> Option<ResolvedPowercfgEntry> {
    match topology.resolve_family(&leaf.instance_id) {
        Ok(family) => {
            tracing::debug!(
                powercfg_name = entry.display_name,
                leaf_instance_id = leaf.instance_id,
                lineage = ?family.lineage,
                family_vidpid = family.hardware_id,
                family_sys = family.system_id,
                representative_name = family.representative_name,
                "resolved wake leaf to managed family"
            );

            Some(ResolvedPowercfgEntry {
                raw: WakeDeviceRaw {
                    name: family
                        .representative_name
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| entry.display_name.clone()),
                    member_name: Some(entry.display_name.clone()),
                    source: DeviceSource::PowerCfg,
                    observed_at: SystemTime::now(),
                    system_id: family.system_id,
                    hardware_id: family.hardware_id,
                    serial_number: None,
                },
                wake_enabled: leaf.wake_enabled,
            })
        }
        Err(err) => {
            tracing::warn!(
                display_name = entry.display_name,
                leaf_instance_id = leaf.instance_id,
                stage = err.stage,
                lineage = ?err.lineage,
                "skipping wake device because parent-chain family resolution failed"
            );
            None
        }
    }
}

fn is_no_device_marker(line: &str) -> bool {
    let normalized = line.trim().to_lowercase();
    matches!(
        normalized.as_str(),
        "none" | "no" | "n/a" | "无" | "沒有" | "没有"
    )
}

fn build_strict_devices(raws: Vec<WakeDeviceRaw>) -> Vec<WakeDevice> {
    let mut devices = Vec::with_capacity(raws.len());
    for raw in raws {
        let display_name = raw.name.clone();
        if let Some(device) = build_device_strict(raw.clone()) {
            devices.push(device);
            continue;
        }

        let preview = build_device(raw);
        tracing::warn!(
            display_name = %display_name,
            candidate_stable_id = %preview.stable_id,
            "skipping wake device without strict identity"
        );
    }
    devices
}

fn aggregate_wake_devices(devices: Vec<WakeDevice>) -> Vec<WakeDevice> {
    let mut grouped = HashMap::<String, WakeDevice>::new();

    for device in devices {
        let stable_id = device.stable_id.clone();
        let member_names = if device.member_names.is_empty() {
            vec![device.display_name.clone()]
        } else {
            device.member_names.clone()
        };

        if let Some(existing) = grouped.get_mut(&stable_id) {
            for member_name in member_names {
                if !existing.member_names.iter().any(|known| known == &member_name) {
                    existing.member_names.push(member_name);
                }
            }
            continue;
        }

        let mut device = device;
        device.member_names = member_names;
        grouped.insert(stable_id, device);
    }

    let mut aggregated = grouped.into_values().collect::<Vec<_>>();
    aggregated.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
    aggregated
}

pub(crate) fn normalize_device_name_for_match(name: &str) -> String {
    let trimmed = name.trim();
    let without_suffix = strip_trailing_numeric_suffix(trimmed);
    let lowered = without_suffix.to_lowercase();
    canonicalize_device_name_alias(&lowered)
}

fn canonicalize_device_name_alias(name: &str) -> String {
    match name {
        "符合 hid 标准的用户控制设备" => "hid-compliant consumer control device".to_string(),
        "符合 hid 标准的系统控制器" => "hid-compliant system controller".to_string(),
        "符合 hid 标准的供应商定义设备" => "hid-compliant vendor-defined device".to_string(),
        _ => name.to_string(),
    }
}

fn strip_trailing_numeric_suffix(name: &str) -> &str {
    let Some(open_idx) = name.rfind(" (") else {
        return name;
    };
    if !name.ends_with(')') {
        return name;
    }
    let digits = &name[(open_idx + 2)..(name.len() - 1)];
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return name;
    }
    name[..open_idx].trim_end()
}

pub(crate) fn normalize_vid_pid_fragment(raw: &str) -> Option<String> {
    let upper = raw.to_ascii_uppercase();
    let vid = extract_hex_segment_any(&upper, &["VID_", "VID&"])?;
    let pid = extract_hex_segment_any(&upper, &["PID_", "PID&"])?;
    Some(format!("VID_{vid}&PID_{pid}"))
}

pub(crate) fn normalize_ven_dev_fragment(raw: &str) -> Option<String> {
    let upper = raw.to_ascii_uppercase();
    let ven = extract_hex_segment(&upper, "VEN_")?;
    let dev = extract_hex_segment(&upper, "DEV_")?;
    Some(format!("VEN_{ven}&DEV_{dev}"))
}

fn extract_hex_segment_any(input: &str, prefixes: &[&str]) -> Option<String> {
    for prefix in prefixes {
        if let Some(segment) = extract_hex_segment(input, prefix) {
            return Some(segment);
        }
    }
    None
}

fn extract_hex_segment(input: &str, prefix: &str) -> Option<String> {
    let start = input.find(prefix)? + prefix.len();
    let mut end = start;
    let bytes = input.as_bytes();
    while end < bytes.len() && bytes[end].is_ascii_hexdigit() {
        end += 1;
    }
    if end <= start {
        return None;
    }
    let segment = &input[start..end];
    if segment.len() < 4 {
        return None;
    }
    Some(segment[(segment.len() - 4)..].to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        aggregate_wake_devices, normalize_device_name_for_match, normalize_ven_dev_fragment,
        normalize_vid_pid_fragment, parse_powercfg_entries, parse_wake_armed_output,
        resolve_powercfg_entries,
    };
    use crate::device::topology::{DeviceNode, DeviceTopology, WakeLeafObservation};
    use crate::device::{identity::build_device_strict, DeviceClass, IdentityConfidence, WakeDevice};

    #[test]
    fn parse_powercfg_output_skips_empty_lines() {
        let output = "\nKeyboard Device\n\nMouse Device\n";
        let parsed = parse_wake_armed_output(output);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "Keyboard Device");
        assert_eq!(parsed[1].name, "Mouse Device");
    }

    #[test]
    fn parse_powercfg_output_skips_localized_none_marker() {
        let output = "无\r\n";
        let parsed = parse_wake_armed_output(output);
        assert!(parsed.is_empty());
    }

    #[test]
    fn normalize_vid_pid_extracts_from_usb_instance() {
        let id = r"USB\VID_046D&PID_C52B&MI_00\7&123456&0&0000";
        let normalized = normalize_vid_pid_fragment(id).expect("VID/PID should parse");
        assert_eq!(normalized, "VID_046D&PID_C52B");
    }

    #[test]
    fn normalize_vid_pid_extracts_from_bluetooth_instance() {
        let id = r"BTHENUM\{00001124-0000-1000-8000-00805F9B34FB}_VID&0002054C_PID&0CE6\8&1F76AF7C&0&D0";
        let normalized = normalize_vid_pid_fragment(id).expect("VID/PID should parse");
        assert_eq!(normalized, "VID_054C&PID_0CE6");
    }

    #[test]
    fn normalize_ven_dev_extracts_from_pci_instance() {
        let id = r"PCI\VEN_8086&DEV_1539&SUBSYS_15391849&REV_03\A8A159FFFF2D76A500";
        let normalized = normalize_ven_dev_fragment(id).expect("VEN/DEV should parse");
        assert_eq!(normalized, "VEN_8086&DEV_1539");
    }

    #[test]
    fn resolve_powercfg_entries_groups_hid_children_into_one_family() {
        let entries = parse_powercfg_entries("HID Keyboard Device (002)\r\nHID Keyboard Device (003)\r\n");
        let wake_leafs = vec![
            WakeLeafObservation {
                instance_id: "HID\\VID_304E&PID_000A&MI_00\\A".to_string(),
                display_name: "HID Keyboard Device".to_string(),
                wake_enabled: true,
            },
            WakeLeafObservation {
                instance_id: "HID\\VID_304E&PID_000A&MI_01\\B".to_string(),
                display_name: "HID Keyboard Device".to_string(),
                wake_enabled: false,
            },
        ];
        let topology = DeviceTopology::from_nodes(vec![
            DeviceNode {
                instance_id: "HID\\VID_304E&PID_000A&MI_00\\A".to_string(),
                parent_instance_id: Some("USB\\ROOT".to_string()),
                container_id: None,
                display_name: Some("Wireless Keyboard".to_string()),
                hardware_ids: vec![],
                hardware_id: Some("VID_304E&PID_000A".to_string()),
                system_id: None,
            },
            DeviceNode {
                instance_id: "HID\\VID_304E&PID_000A&MI_01\\B".to_string(),
                parent_instance_id: Some("USB\\ROOT".to_string()),
                container_id: None,
                display_name: Some("Wireless Keyboard".to_string()),
                hardware_ids: vec![],
                hardware_id: Some("VID_304E&PID_000A".to_string()),
                system_id: None,
            },
            DeviceNode {
                instance_id: "USB\\ROOT".to_string(),
                parent_instance_id: None,
                container_id: None,
                display_name: Some("Wireless Keyboard".to_string()),
                hardware_ids: vec![],
                hardware_id: None,
                system_id: None,
            },
        ]);

        let raws = resolve_powercfg_entries(entries, &wake_leafs, &topology)
            .into_iter()
            .map(|entry| entry.raw)
            .collect::<Vec<_>>();
        let accepted = raws
            .into_iter()
            .filter_map(build_device_strict)
            .collect::<Vec<_>>();
        let aggregated = aggregate_wake_devices(accepted);

        assert_eq!(aggregated.len(), 1);
        assert_eq!(aggregated[0].stable_id, "vidpid:vid_304e&pid_000a");
        assert_eq!(aggregated[0].member_names.len(), 2);
        assert_eq!(aggregated[0].display_name, "Wireless Keyboard");
    }

    #[test]
    fn resolve_powercfg_entries_skips_unmanaged_leaf() {
        let entries = parse_powercfg_entries("Generic Device\r\n");
        let wake_leafs = vec![WakeLeafObservation {
            instance_id: "ROOT\\UNKNOWN\\0001".to_string(),
            display_name: "Generic Device".to_string(),
            wake_enabled: true,
        }];
        let topology = DeviceTopology::from_nodes(vec![DeviceNode {
            instance_id: "ROOT\\UNKNOWN\\0001".to_string(),
            parent_instance_id: None,
            container_id: None,
            display_name: Some("Generic Device".to_string()),
            hardware_ids: vec![],
            hardware_id: None,
            system_id: None,
        }]);

        let resolved = resolve_powercfg_entries(entries, &wake_leafs, &topology);
        assert!(resolved.is_empty());
    }

    #[test]
    fn normalize_device_name_for_match_strips_numeric_suffix() {
        let normalized = normalize_device_name_for_match("HID Keyboard Device (003)");
        assert_eq!(normalized, "hid keyboard device");
    }

    #[test]
    fn normalize_device_name_for_match_maps_chinese_hid_aliases() {
        let consumer = normalize_device_name_for_match("符合 HID 标准的用户控制设备 (004)");
        let system = normalize_device_name_for_match("符合 HID 标准的系统控制器 (007)");
        let vendor = normalize_device_name_for_match("符合 HID 标准的供应商定义设备 (019)");

        assert_eq!(consumer, "hid-compliant consumer control device");
        assert_eq!(system, "hid-compliant system controller");
        assert_eq!(vendor, "hid-compliant vendor-defined device");
    }

    #[test]
    fn aggregate_wake_devices_groups_same_family_members() {
        let devices = vec![
            WakeDevice {
                display_name: "Wireless Keyboard".to_string(),
                stable_id: "vidpid:vid_304e&pid_000a".to_string(),
                member_names: vec!["HID Keyboard Device".to_string()],
                class: DeviceClass::Keyboard,
                identity_confidence: IdentityConfidence::High,
            },
            WakeDevice {
                display_name: "Wireless Keyboard".to_string(),
                stable_id: "vidpid:vid_304e&pid_000a".to_string(),
                member_names: vec!["HID Keyboard Device (003)".to_string()],
                class: DeviceClass::Keyboard,
                identity_confidence: IdentityConfidence::High,
            },
        ];

        let aggregated = aggregate_wake_devices(devices);
        assert_eq!(aggregated.len(), 1);
        assert_eq!(aggregated[0].member_names.len(), 2);
    }

    #[test]
    fn strict_builder_filters_unknown_identity_device() {
        let raws = parse_wake_armed_output("Generic Device Name\r\n");
        let accepted = raws
            .into_iter()
            .filter_map(build_device_strict)
            .collect::<Vec<_>>();
        assert!(accepted.is_empty());
    }
}
