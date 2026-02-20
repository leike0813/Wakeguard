use crate::device::{
    identity::{build_device, build_device_strict},
    DeviceSource, WakeDevice, WakeDeviceRaw,
};
use crate::error::WakeguardError;
use std::collections::HashMap;
use std::process::Command;
use std::time::SystemTime;

const DEVICE_QUERY_WAKE_ARMED: &str = "wake_armed";
const DEVICE_QUERY_WAKE_PROGRAMMABLE: &str = "wake_programmable";
const DEVICE_DISABLE_WAKE_ARG: &str = "-devicedisablewake";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum StrictIdentityCandidate {
    VidPid(String),
    PciVenDev(String),
}

impl StrictIdentityCandidate {
    fn as_sort_key(&self) -> String {
        match self {
            Self::VidPid(value) => format!("vidpid:{value}"),
            Self::PciVenDev(value) => format!("pci:{value}"),
        }
    }
}

pub fn list_wake_enabled_devices() -> Result<Vec<WakeDevice>, WakeguardError> {
    query_powercfg_devices(DEVICE_QUERY_WAKE_ARMED, "powercfg -devicequery wake_armed")
}

pub fn list_wake_programmable_devices() -> Result<Vec<WakeDevice>, WakeguardError> {
    query_powercfg_devices(
        DEVICE_QUERY_WAKE_PROGRAMMABLE,
        "powercfg -devicequery wake_programmable",
    )
}

fn query_powercfg_devices(
    query_target: &str,
    command_display: &str,
) -> Result<Vec<WakeDevice>, WakeguardError> {
    let stdout = run_powercfg_query_utf8(query_target, command_display)?;
    let candidates = match load_identity_candidates_from_wmi() {
        Ok(map) => map,
        Err(err) => {
            tracing::warn!(
                error = %err,
                "failed to load strict identity candidates, fallback will be limited"
            );
            HashMap::new()
        }
    };

    let raws = parse_wake_output_with_identity_candidates(&stdout, &candidates);
    let devices = build_strict_devices(raws);
    Ok(deduplicate_devices(devices))
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
    parse_wake_output_with_identity_candidates(raw, &HashMap::new())
}

fn parse_wake_output_with_identity_candidates(
    raw: &str,
    identity_candidates: &HashMap<String, Vec<StrictIdentityCandidate>>,
) -> Vec<WakeDeviceRaw> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !is_no_device_marker(line))
        .map(|line| {
            let (system_id, hardware_id, serial_number) =
                infer_identity_signals(line, identity_candidates);
            WakeDeviceRaw {
                name: line.to_string(),
                source: DeviceSource::PowerCfg,
                observed_at: SystemTime::now(),
                system_id,
                hardware_id,
                serial_number,
            }
        })
        .collect()
}

fn is_no_device_marker(line: &str) -> bool {
    let normalized = line.trim().to_lowercase();
    matches!(
        normalized.as_str(),
        "none" | "no" | "n/a" | "无" | "沒有" | "没有"
    )
}

fn infer_identity_signals(
    line: &str,
    identity_candidates: &HashMap<String, Vec<StrictIdentityCandidate>>,
) -> (Option<String>, Option<String>, Option<String>) {
    let trimmed = line.trim();
    let normalized_name = normalize_device_name_for_match(trimmed);
    let duplicate_index = parse_duplicate_index(trimmed);
    let candidate = pick_identity_candidate(identity_candidates.get(&normalized_name), duplicate_index);

    let direct_vid_pid = normalize_vid_pid_fragment(trimmed);
    let direct_ven_dev = normalize_ven_dev_fragment(trimmed);

    let hardware_id = direct_vid_pid.or_else(|| match candidate.as_ref() {
        Some(StrictIdentityCandidate::VidPid(value)) => Some(value.clone()),
        _ => None,
    });
    let system_id = direct_ven_dev.or_else(|| match candidate {
        Some(StrictIdentityCandidate::PciVenDev(value)) => Some(value),
        _ => None,
    });

    (system_id, hardware_id, None)
}

fn pick_identity_candidate(
    candidates: Option<&Vec<StrictIdentityCandidate>>,
    duplicate_index: usize,
) -> Option<StrictIdentityCandidate> {
    let list = candidates?;
    if list.is_empty() {
        return None;
    }
    let idx = duplicate_index.saturating_sub(1).min(list.len() - 1);
    list.get(idx).cloned()
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

fn load_identity_candidates_from_wmi() -> Result<HashMap<String, Vec<StrictIdentityCandidate>>, WakeguardError> {
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
        $norm = $name.Trim();
        if ($norm -match '^(.*) \((\d+)\)$') { $norm = $Matches[1].TrimEnd() }
        $norm = $norm.ToLowerInvariant();
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($norm);
        $hex = [System.BitConverter]::ToString($bytes).Replace('-','');
        "{0}`t{1}" -f $hex, $pnp
    }
}"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()?;

    if !output.status.success() {
        return Err(WakeguardError::CommandFailed {
            command: "powershell MSPower_DeviceWakeEnable lookup".to_string(),
            details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_identity_candidates_from_wmi_output(&stdout))
}

fn parse_identity_candidates_from_wmi_output(
    raw: &str,
) -> HashMap<String, Vec<StrictIdentityCandidate>> {
    let mut grouped: HashMap<String, Vec<StrictIdentityCandidate>> = HashMap::new();

    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let mut parts = line.splitn(2, '\t');
        let Some(name_key_hex) = parts.next() else {
            continue;
        };
        let Some(pnp_device_id) = parts.next() else {
            continue;
        };
        let Some(name_key) = decode_hex_utf8(name_key_hex) else {
            continue;
        };

        let Some(candidate) = parse_strict_identity_candidate(pnp_device_id) else {
            continue;
        };

        grouped.entry(name_key).or_default().push(candidate);
    }

    for values in grouped.values_mut() {
        values.sort_by_key(StrictIdentityCandidate::as_sort_key);
        values.dedup();
    }

    grouped
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

fn parse_strict_identity_candidate(raw: &str) -> Option<StrictIdentityCandidate> {
    if let Some(vid_pid) = normalize_vid_pid_fragment(raw) {
        return Some(StrictIdentityCandidate::VidPid(vid_pid));
    }
    if let Some(ven_dev) = normalize_ven_dev_fragment(raw) {
        return Some(StrictIdentityCandidate::PciVenDev(ven_dev));
    }
    None
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

fn parse_duplicate_index(name: &str) -> usize {
    let trimmed = name.trim();
    let Some(open_idx) = trimmed.rfind(" (") else {
        return 1;
    };
    if !trimmed.ends_with(')') {
        return 1;
    }
    let digits = &trimmed[(open_idx + 2)..(trimmed.len() - 1)];
    digits.parse::<usize>().unwrap_or(1)
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

fn deduplicate_devices(devices: Vec<WakeDevice>) -> Vec<WakeDevice> {
    let mut seen = std::collections::HashSet::new();
    let mut unique = Vec::with_capacity(devices.len());
    for device in devices {
        if seen.insert(device.stable_id.clone()) {
            unique.push(device);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::{
        decode_hex_utf8,
        normalize_device_name_for_match, normalize_ven_dev_fragment, normalize_vid_pid_fragment,
        parse_identity_candidates_from_wmi_output, parse_wake_armed_output,
        parse_wake_output_with_identity_candidates,
    };
    use crate::device::identity::build_device_strict;
    use std::collections::HashMap;

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
    fn parse_identity_candidates_groups_by_normalized_name() {
        let raw = "\
686964206b6579626f61726420646576696365\tHID\\VID_046D&PID_C52B&MI_00\\A\r\n\
686964206b6579626f61726420646576696365\tHID\\VID_046D&PID_C537&MI_00\\B\r\n\
686964206b6579626f61726420646576696365\tHID\\VID_046D&PID_C52B&MI_00\\A\r\n";

        let map = parse_identity_candidates_from_wmi_output(raw);
        let values = map
            .get("hid keyboard device")
            .expect("normalized key should exist");
        assert!(values.len() >= 2);
    }

    #[test]
    fn parse_output_matches_candidates_after_numeric_suffix_normalization() {
        let candidates = parse_identity_candidates_from_wmi_output(
            "686964206b6579626f61726420646576696365\tHID\\VID_046D&PID_C52B&MI_00\\A\r\n",
        );
        let devices = parse_wake_output_with_identity_candidates(
            "HID Keyboard Device (003)\r\n",
            &candidates,
        );
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].hardware_id.as_deref(), Some("VID_046D&PID_C52B"));
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
    fn strict_builder_accepts_pci_and_vidpid_candidates() {
        let mut candidates = HashMap::new();
        candidates.insert(
            "intel(r) i211 gigabit network connection".to_string(),
            vec![super::StrictIdentityCandidate::PciVenDev(
                "VEN_8086&DEV_1539".to_string(),
            )],
        );
        candidates.insert(
            "hid keyboard device".to_string(),
            vec![super::StrictIdentityCandidate::VidPid(
                "VID_046D&PID_C52B".to_string(),
            )],
        );

        let raws = parse_wake_output_with_identity_candidates(
            "Intel(R) I211 Gigabit Network Connection\r\nHID Keyboard Device (003)\r\n",
            &candidates,
        );
        let accepted = raws
            .into_iter()
            .filter_map(build_device_strict)
            .collect::<Vec<_>>();

        assert_eq!(accepted.len(), 2);
    }

    #[test]
    fn decode_hex_utf8_handles_unicode() {
        let decoded = decode_hex_utf8("e7aca6e590882048494420e6a087e58786")
            .expect("hex should decode");
        assert_eq!(decoded, "符合 HID 标准");
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
