use crate::config::registry;
use crate::device::powercfg;
use crate::error::WakeguardError;
use std::collections::HashSet;
use std::io::{self, Write};

pub fn launch_ui(onboarding: bool) -> Result<(), WakeguardError> {
    let devices = powercfg::list_wake_programmable_devices()?;
    let wake_enabled_ids = powercfg::list_wake_enabled_devices()?
        .into_iter()
        .map(|d| d.stable_id)
        .collect::<HashSet<_>>();
    let whitelist = registry::load_whitelist()?;

    println!("Wakeguard UI");
    println!("Managed devices:");
    for (idx, device) in devices.iter().enumerate() {
        let in_whitelist = whitelist.contains(&device.stable_id);
        let wake_disabled = !wake_enabled_ids.contains(&device.stable_id);
        println!(
            "  {}. {} [{}] whitelist={} wake_disabled={}",
            idx + 1,
            format_device_label(device),
            device.stable_id,
            in_whitelist,
            wake_disabled
        );
    }

    if onboarding {
        run_onboarding(&devices, &whitelist)?;
    } else {
        println!("Tip: run `wakeguard ui --onboarding` to launch onboarding actions.");
    }

    Ok(())
}

fn format_device_label(device: &crate::device::WakeDevice) -> String {
    let member_count = device.member_names.len();
    if member_count <= 1 {
        return device.display_name.clone();
    }
    format!("{} (+{} family members)", device.display_name, member_count - 1)
}

fn run_onboarding(
    devices: &[crate::device::WakeDevice],
    whitelist: &HashSet<String>,
) -> Result<(), WakeguardError> {
    println!();
    println!("First-install onboarding");
    println!("1) Keep whitelist empty");
    println!("2) Select devices to add");
    println!("3) Add all supported devices");
    print!("Choose option [1/2/3]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    match input.trim() {
        "1" => {
            println!("Whitelist kept empty.");
            Ok(())
        }
        "3" => {
            let all_ids = devices.iter().map(|d| d.stable_id.clone()).collect::<Vec<_>>();
            registry::add_to_whitelist(all_ids)?;
            println!("Added all detected devices to whitelist.");
            Ok(())
        }
        "2" => {
            print!("Enter device indices separated by commas (e.g. 1,3): ");
            io::stdout().flush()?;
            let mut selected = String::new();
            io::stdin().read_line(&mut selected)?;
            let indices = parse_selection_indices(selected.trim(), devices.len())?;
            let selected_ids = indices
                .into_iter()
                .map(|idx| devices[idx].stable_id.clone())
                .collect::<Vec<_>>();
            let mut merged = whitelist.clone();
            merged.extend(selected_ids);
            registry::save_whitelist(&merged)?;
            println!("Selected devices added to whitelist.");
            Ok(())
        }
        _ => Err(WakeguardError::InvalidConfig(
            "invalid onboarding option, expected one of 1/2/3".to_string(),
        )),
    }
}

fn parse_selection_indices(raw: &str, len: usize) -> Result<Vec<usize>, WakeguardError> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    for token in raw.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        let value: usize = token.parse().map_err(|_| {
            WakeguardError::InvalidConfig(format!("invalid index token: '{token}'"))
        })?;
        if value == 0 || value > len {
            return Err(WakeguardError::InvalidConfig(format!(
                "index out of range: {value}, valid range is 1..={len}"
            )));
        }
        result.push(value - 1);
    }
    result.sort_unstable();
    result.dedup();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{format_device_label, parse_selection_indices};
    use crate::device::{DeviceClass, IdentityConfidence, WakeDevice};

    #[test]
    fn parse_selection_indices_handles_basic_case() {
        let parsed = parse_selection_indices("1,3,2", 3).expect("parse should succeed");
        assert_eq!(parsed, vec![0, 1, 2]);
    }

    #[test]
    fn parse_selection_indices_rejects_out_of_range() {
        let err = parse_selection_indices("4", 3).expect_err("should reject out of range");
        assert!(format!("{err}").contains("index out of range"));
    }

    #[test]
    fn format_device_label_shows_family_members() {
        let device = WakeDevice {
            display_name: "HID Keyboard Device".to_string(),
            stable_id: "vidpid:vid_304e&pid_000a".to_string(),
            member_names: vec![
                "HID Keyboard Device".to_string(),
                "HID Keyboard Device (003)".to_string(),
            ],
            class: DeviceClass::Keyboard,
            identity_confidence: IdentityConfidence::High,
        };

        assert_eq!(
            format_device_label(&device),
            "HID Keyboard Device (+1 family members)"
        );
    }
}
