mod lifecycle;
mod windows_host;
pub mod worker;

use crate::error::WakeguardError;
use std::env;
use std::path::PathBuf;
use std::time::Duration;

pub fn run_service() -> Result<(), WakeguardError> {
    let interval = read_scan_interval();
    let max_cycles = read_max_cycles();
    tracing::info!(
        scan_interval_secs = interval.as_secs(),
        max_cycles = format!("{max_cycles:?}"),
        "service run requested"
    );
    windows_host::run_or_console(interval, max_cycles)
}

pub fn install_service(binary_path: Option<PathBuf>, launch_ui: bool) -> Result<(), WakeguardError> {
    lifecycle::install(binary_path, launch_ui)
}

pub fn uninstall_service() -> Result<(), WakeguardError> {
    lifecycle::uninstall()
}

fn read_scan_interval() -> Duration {
    env::var("WAKEGUARD_SCAN_INTERVAL_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(30))
}

fn read_max_cycles() -> Option<usize> {
    env::var("WAKEGUARD_MAX_CYCLES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
}
