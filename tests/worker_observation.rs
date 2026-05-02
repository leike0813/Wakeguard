use std::collections::HashSet;
use wakeguard::device::{DeviceClass, IdentityConfidence, WakeDevice};
use wakeguard::service::worker::{build_new_device_events, detect_new_devices};

fn make_device(stable_id: &str, display_name: &str) -> WakeDevice {
    WakeDevice {
        display_name: display_name.to_string(),
        stable_id: stable_id.to_string(),
        member_names: vec![display_name.to_string()],
        class: DeviceClass::Unknown,
        identity_confidence: IdentityConfidence::High,
    }
}

#[test]
fn detect_new_devices_from_previous_snapshot() {
    let previous = HashSet::from(["sys:a".to_string(), "sys:b".to_string()]);
    let current = vec![
        make_device("sys:a", "A"),
        make_device("sys:b", "B"),
        make_device("sys:c", "C"),
    ];

    let new_devices = detect_new_devices(&previous, &current);
    assert_eq!(new_devices.len(), 1);
    assert_eq!(new_devices[0].stable_id, "sys:c");
}

#[test]
fn new_device_events_include_whitelist_status() {
    let new_devices = vec![make_device("sys:a", "A"), make_device("sys:b", "B")];
    let whitelist = HashSet::from(["sys:b".to_string()]);

    let events = build_new_device_events(&new_devices, &whitelist);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event, "new_wake_device_detected");
    assert!(!events[0].is_whitelisted);
    assert!(events[1].is_whitelisted);
}

#[test]
fn renumbered_family_is_not_treated_as_new_device() {
    let previous = HashSet::from(["vidpid:vid_304e&pid_000a".to_string()]);
    let current = vec![make_device(
        "vidpid:vid_304e&pid_000a",
        "HID Keyboard Device (003)",
    )];

    let new_devices = detect_new_devices(&previous, &current);
    assert!(new_devices.is_empty());
}
