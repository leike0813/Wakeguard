use crate::config::registry;
use crate::device::{powercfg, WakeDevice};
use crate::error::WakeguardError;
use crate::policy;
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::Duration;

const DEFAULT_SCAN_INTERVAL: Duration = Duration::from_secs(30);
const DISABLE_RETRY_COOLDOWN_CYCLES: u64 = 3;

#[derive(Debug, Default)]
pub struct WorkerState {
    previous_ids: HashSet<String>,
    disable_last_attempt_cycle: HashMap<String, u64>,
    cycle: u64,
    metrics: RuntimeMetrics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDeviceEvent {
    pub event: &'static str,
    pub stable_id: String,
    pub display_name: String,
    pub is_whitelisted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanReport {
    pub scanned: usize,
    pub disabled: usize,
    pub observed: usize,
    pub new_devices: usize,
    pub disable_failures: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeMetrics {
    pub cycles: u64,
    pub total_scanned: u64,
    pub total_disabled: u64,
    pub total_observed: u64,
    pub total_new_devices: u64,
    pub scan_failures: u64,
    pub disable_failures: u64,
}

pub fn run_once() -> Result<(), WakeguardError> {
    let mut state = WorkerState::default();
    let report = execute_scan_cycle(&mut state)?;
    tracing::info!(
        scanned = report.scanned,
        disabled = report.disabled,
        observed = report.observed,
        new_devices = report.new_devices,
        disable_failures = report.disable_failures,
        "one-shot scan completed"
    );
    Ok(())
}

pub fn run_periodic_loop(
    interval: Option<Duration>,
    max_cycles: Option<usize>,
) -> Result<(), WakeguardError> {
    run_periodic_loop_with_shutdown(interval, max_cycles, || false)
}

pub fn run_periodic_loop_with_shutdown<F>(
    interval: Option<Duration>,
    max_cycles: Option<usize>,
    mut should_stop: F,
) -> Result<(), WakeguardError>
where
    F: FnMut() -> bool,
{
    let interval = interval.unwrap_or(DEFAULT_SCAN_INTERVAL);
    let mut state = WorkerState::default();
    let mut cycles = 0usize;

    loop {
        if should_stop() {
            tracing::info!("worker stop signal received");
            break;
        }

        if let Err(err) = execute_scan_cycle(&mut state) {
            state.metrics.scan_failures += 1;
            tracing::error!(error = %err, "scan cycle failed; loop will continue");
        }

        cycles += 1;
        if let Some(limit) = max_cycles {
            if cycles >= limit {
                tracing::info!(cycles, "max cycles reached; stopping worker loop");
                break;
            }
        }

        sleep_until_next_cycle_or_stop(interval, &mut should_stop);
    }

    Ok(())
}

fn sleep_until_next_cycle_or_stop<F>(interval: Duration, should_stop: &mut F)
where
    F: FnMut() -> bool,
{
    let mut remaining_ms = interval.as_millis() as u64;
    const SLICE_MS: u64 = 250;
    while remaining_ms > 0 {
        if should_stop() {
            break;
        }
        let step = remaining_ms.min(SLICE_MS);
        thread::sleep(Duration::from_millis(step));
        remaining_ms -= step;
    }
}

fn execute_scan_cycle(state: &mut WorkerState) -> Result<ScanReport, WakeguardError> {
    state.cycle += 1;

    let whitelist = match registry::load_whitelist() {
        Ok(whitelist) => whitelist,
        Err(err) => {
            tracing::error!(error = %err, "failed to load whitelist; using empty whitelist");
            HashSet::new()
        }
    };

    let devices = powercfg::list_wake_programmable_devices()?;
    let wake_enabled_devices = powercfg::list_wake_enabled_devices()?;
    let wake_enabled_names = wake_enabled_devices
        .iter()
        .map(|d| (d.stable_id.clone(), d.display_name.clone()))
        .collect::<HashMap<_, _>>();
    let wake_enabled_ids = wake_enabled_names.keys().cloned().collect::<HashSet<_>>();
    let plan = policy::build_policy_plan(&devices, &whitelist);
    let mut disable_failures = 0usize;

    for action in &plan.disable_actions {
        if !wake_enabled_ids.contains(&action.stable_id) {
            tracing::debug!(
                stable_id = action.stable_id,
                "disable action skipped because wake is already disabled"
            );
            continue;
        }

        if !should_attempt_disable(
            &action.stable_id,
            state.cycle,
            &state.disable_last_attempt_cycle,
            DISABLE_RETRY_COOLDOWN_CYCLES,
        ) {
            tracing::debug!(
                stable_id = action.stable_id,
                cycle = state.cycle,
                cooldown_cycles = DISABLE_RETRY_COOLDOWN_CYCLES,
                "disable action skipped due to cooldown"
            );
            continue;
        }

        state
            .disable_last_attempt_cycle
            .insert(action.stable_id.clone(), state.cycle);

        let disable_target = wake_enabled_names
            .get(&action.stable_id)
            .map(String::as_str)
            .unwrap_or(action.device_name.as_str());

        if let Err(err) = powercfg::disable_wake_for_device(disable_target) {
            disable_failures += 1;
            tracing::error!(
                device = disable_target,
                plan_device = action.device_name,
                stable_id = action.stable_id,
                reason = action.reason,
                error = %err,
                "disable wake action failed"
            );
        }
    }

    for observed in &plan.observed_devices {
        tracing::info!(
            event = "low_confidence_identity",
            stable_id = observed.stable_id,
            display_name = observed.display_name,
            "device requires observation"
        );
    }

    let new_devices = detect_new_devices(&state.previous_ids, &devices);
    for event in build_new_device_events(&new_devices, &whitelist) {
        tracing::info!(
            event = event.event,
            stable_id = event.stable_id,
            display_name = event.display_name,
            is_whitelisted = event.is_whitelisted,
            "new wake-capable device detected"
        );
    }

    state.previous_ids = devices.iter().map(|d| d.stable_id.clone()).collect();
    state.metrics.cycles += 1;
    state.metrics.total_scanned += devices.len() as u64;
    state.metrics.total_disabled += plan.disable_actions.len() as u64;
    state.metrics.total_observed += plan.observed_devices.len() as u64;
    state.metrics.total_new_devices += new_devices.len() as u64;
    state.metrics.disable_failures += disable_failures as u64;

    tracing::info!(
        event = "monitor_heartbeat",
        cycle = state.metrics.cycles,
        scanned = devices.len(),
        disabled = plan.disable_actions.len(),
        observed = plan.observed_devices.len(),
        new_devices = new_devices.len(),
        scan_failures = state.metrics.scan_failures,
        disable_failures = state.metrics.disable_failures,
        "scan cycle heartbeat"
    );

    Ok(ScanReport {
        scanned: devices.len(),
        disabled: plan.disable_actions.len(),
        observed: plan.observed_devices.len(),
        new_devices: new_devices.len(),
        disable_failures,
    })
}

pub fn detect_new_devices(previous_ids: &HashSet<String>, current: &[WakeDevice]) -> Vec<WakeDevice> {
    current
        .iter()
        .filter(|device| !previous_ids.contains(&device.stable_id))
        .cloned()
        .collect()
}

pub fn build_new_device_events(
    new_devices: &[WakeDevice],
    whitelist: &HashSet<String>,
) -> Vec<NewDeviceEvent> {
    new_devices
        .iter()
        .map(|device| NewDeviceEvent {
            event: "new_wake_device_detected",
            stable_id: device.stable_id.clone(),
            display_name: device.display_name.clone(),
            is_whitelisted: whitelist.contains(&device.stable_id),
        })
        .collect()
}

pub fn should_attempt_disable(
    stable_id: &str,
    current_cycle: u64,
    history: &HashMap<String, u64>,
    cooldown_cycles: u64,
) -> bool {
    match history.get(stable_id) {
        Some(last_cycle) => current_cycle.saturating_sub(*last_cycle) >= cooldown_cycles,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_new_device_events, detect_new_devices, run_periodic_loop_with_shutdown,
        should_attempt_disable,
    };
    use crate::device::{DeviceClass, IdentityConfidence, WakeDevice};
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    fn test_device(id: &str, name: &str) -> WakeDevice {
        WakeDevice {
            display_name: name.to_string(),
            stable_id: id.to_string(),
            class: DeviceClass::Unknown,
            identity_confidence: IdentityConfidence::High,
        }
    }

    #[test]
    fn detect_new_devices_returns_only_delta() {
        let previous = HashSet::from(["id-a".to_string(), "id-b".to_string()]);
        let current = vec![
            test_device("id-a", "A"),
            test_device("id-b", "B"),
            test_device("id-c", "C"),
        ];

        let delta = detect_new_devices(&previous, &current);
        assert_eq!(delta.len(), 1);
        assert_eq!(delta[0].stable_id, "id-c");
    }

    #[test]
    fn build_new_device_events_includes_whitelist_flag() {
        let devices = vec![test_device("id-a", "A"), test_device("id-b", "B")];
        let whitelist = HashSet::from(["id-b".to_string()]);

        let events = build_new_device_events(&devices, &whitelist);
        assert_eq!(events.len(), 2);
        assert!(!events[0].is_whitelisted);
        assert!(events[1].is_whitelisted);
        assert_eq!(events[0].event, "new_wake_device_detected");
    }

    #[test]
    fn disable_attempt_respects_cooldown_window() {
        let mut history = HashMap::new();
        history.insert("id-a".to_string(), 10);

        assert!(!should_attempt_disable("id-a", 11, &history, 3));
        assert!(!should_attempt_disable("id-a", 12, &history, 3));
        assert!(should_attempt_disable("id-a", 13, &history, 3));
        assert!(should_attempt_disable("id-b", 11, &history, 3));
    }

    #[test]
    fn worker_loop_honors_shutdown_signal() {
        let stop = Arc::new(AtomicBool::new(true));
        let flag = stop.clone();
        let result = run_periodic_loop_with_shutdown(
            Some(Duration::from_millis(10)),
            None,
            move || flag.load(Ordering::Relaxed),
        );
        assert!(result.is_ok());
    }
}
