use crate::device::WakeDevice;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisableAction {
    pub stable_id: String,
    pub device_names: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Disable { reason: String },
    Observe { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceDecision {
    pub device: WakeDevice,
    pub decision: PolicyDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyPlan {
    pub decisions: Vec<DeviceDecision>,
    pub disable_actions: Vec<DisableAction>,
    pub observed_devices: Vec<WakeDevice>,
}

pub fn build_disable_plan(
    devices: &[WakeDevice],
    whitelist: &HashSet<String>,
) -> Vec<DisableAction> {
    build_policy_plan(devices, whitelist).disable_actions
}

pub fn build_policy_plan(devices: &[WakeDevice], whitelist: &HashSet<String>) -> PolicyPlan {
    let mut decisions = Vec::with_capacity(devices.len());
    let mut disable_actions = Vec::new();
    let mut observed_devices = Vec::new();

    for device in devices {
        let decision = evaluate_device(device, whitelist);
        match &decision {
            PolicyDecision::Disable { reason } => disable_actions.push(DisableAction {
                stable_id: device.stable_id.clone(),
                device_names: if device.member_names.is_empty() {
                    vec![device.display_name.clone()]
                } else {
                    device.member_names.clone()
                },
                reason: reason.clone(),
            }),
            PolicyDecision::Observe { .. } => observed_devices.push(device.clone()),
            PolicyDecision::Allow => {}
        }
        decisions.push(DeviceDecision {
            device: device.clone(),
            decision,
        });
    }

    PolicyPlan {
        decisions,
        disable_actions,
        observed_devices,
    }
}

pub fn evaluate_device(device: &WakeDevice, whitelist: &HashSet<String>) -> PolicyDecision {
    if whitelist.contains(&device.stable_id) {
        return PolicyDecision::Allow;
    }
    if device.is_low_confidence() {
        return PolicyDecision::Observe {
            reason: "low_confidence_identity".to_string(),
        };
    }
    PolicyDecision::Disable {
        reason: "non_whitelisted_device".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{build_policy_plan, evaluate_device, PolicyDecision};
    use crate::device::{DeviceClass, IdentityConfidence, WakeDevice};
    use std::collections::HashSet;

    fn make_device(stable_id: &str, confidence: IdentityConfidence) -> WakeDevice {
        WakeDevice {
            display_name: stable_id.to_string(),
            stable_id: stable_id.to_string(),
            member_names: vec![stable_id.to_string()],
            class: DeviceClass::Unknown,
            identity_confidence: confidence,
        }
    }

    #[test]
    fn whitelisted_device_is_allowed() {
        let device = make_device("sys:mouse-1", IdentityConfidence::High);
        let mut whitelist = HashSet::new();
        whitelist.insert("sys:mouse-1".to_string());

        let decision = evaluate_device(&device, &whitelist);
        assert!(matches!(decision, PolicyDecision::Allow));
    }

    #[test]
    fn non_whitelisted_high_confidence_device_is_disabled() {
        let device = make_device("sys:kbd-1", IdentityConfidence::High);
        let whitelist = HashSet::new();

        let decision = evaluate_device(&device, &whitelist);
        assert!(matches!(decision, PolicyDecision::Disable { .. }));
    }

    #[test]
    fn non_whitelisted_low_confidence_device_is_observed() {
        let device = make_device("name:unknown-device", IdentityConfidence::Low);
        let whitelist = HashSet::new();

        let decision = evaluate_device(&device, &whitelist);
        assert!(matches!(decision, PolicyDecision::Observe { .. }));
    }

    #[test]
    fn policy_plan_collects_disable_and_observe_actions() {
        let whitelist = HashSet::from(["sys:allowed".to_string()]);
        let devices = vec![
            make_device("sys:allowed", IdentityConfidence::High),
            make_device("sys:disable", IdentityConfidence::Medium),
            make_device("name:observe", IdentityConfidence::Low),
        ];

        let plan = build_policy_plan(&devices, &whitelist);
        assert_eq!(plan.disable_actions.len(), 1);
        assert_eq!(plan.observed_devices.len(), 1);
        assert_eq!(plan.decisions.len(), 3);
    }

    #[test]
    fn policy_plan_uses_family_member_names_for_disable_action() {
        let whitelist = HashSet::new();
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

        let plan = build_policy_plan(&[device], &whitelist);
        assert_eq!(plan.disable_actions.len(), 1);
        assert_eq!(plan.disable_actions[0].device_names.len(), 2);
    }
}
