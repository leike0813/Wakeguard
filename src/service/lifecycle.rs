use crate::config::registry;
use crate::error::WakeguardError;
use crate::install_path;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

const SERVICE_NAME: &str = "Wakeguard";
const SERVICE_STOP_TIMEOUT: Duration = Duration::from_secs(15);
const SERVICE_STATE_STOPPED: u32 = 1;

pub fn install(binary_source: Option<PathBuf>, launch_ui: bool) -> Result<(), WakeguardError> {
    let service_installed = query_service_exists()?;
    let global_executable = install_path::global_executable_path();
    let default_source = std::env::current_exe()?;
    let resolved_source = resolve_install_source_path(binary_source.as_deref(), &default_source)?;

    if !service_installed {
        registry::sanitize_registry_schema()?;
        registry::ensure_default_whitelist_empty()?;
    }

    if service_installed {
        stop_service_if_running()?;
    }
    install_path::copy_binary_to_global(&resolved_source)?;

    if !global_executable.exists() {
        return Err(WakeguardError::InvalidConfig(format!(
            "global executable missing: {}",
            global_executable.display()
        )));
    }

    install_path::ensure_global_bin_in_system_path()?;

    if !service_installed {
        create_service(&global_executable)?;
    } else {
        configure_service(&global_executable)?;
    }
    start_service()?;

    if launch_ui {
        if let Err(err) = launch_ui_process(true) {
            tracing::warn!(error = %err, "failed to auto-launch UI after install");
        }
    }

    Ok(())
}

pub fn uninstall() -> Result<(), WakeguardError> {
    let service_installed = query_service_exists()?;
    if service_installed {
        stop_service_if_running()?;
        delete_service()?;
    }

    install_path::remove_global_bin_from_system_path()?;
    install_path::remove_global_binary_with_fallback()?;

    Ok(())
}

fn create_service(executable: &Path) -> Result<(), WakeguardError> {
    let bin_path = format!("\"{}\" run", executable.display());
    run_sc_checked(&[
        "create",
        SERVICE_NAME,
        "binPath=",
        &bin_path,
        "start=",
        "auto",
        "DisplayName=",
        SERVICE_NAME,
    ])
}

fn configure_service(executable: &Path) -> Result<(), WakeguardError> {
    let bin_path = format!("\"{}\" run", executable.display());
    run_sc_checked(&[
        "config",
        SERVICE_NAME,
        "binPath=",
        &bin_path,
        "start=",
        "auto",
        "DisplayName=",
        SERVICE_NAME,
    ])
}

fn start_service() -> Result<(), WakeguardError> {
    let output = run_sc(["start", SERVICE_NAME])?;
    if output.status.success() {
        return Ok(());
    }
    let text = output_text(&output);
    if is_service_already_running(&text) {
        return Ok(());
    }
    Err(command_failure("sc.exe start", &text))
}

fn stop_service_if_running() -> Result<(), WakeguardError> {
    let output = run_sc(["stop", SERVICE_NAME])?;
    if output.status.success() {
        return wait_for_service_stopped(SERVICE_STOP_TIMEOUT);
    }
    let text = output_text(&output);
    if is_service_not_active(&text) {
        return Ok(());
    }
    Err(command_failure("sc.exe stop", &text))
}

fn delete_service() -> Result<(), WakeguardError> {
    let output = run_sc(["delete", SERVICE_NAME])?;
    if output.status.success() {
        return Ok(());
    }
    let text = output_text(&output);
    if is_service_not_found(&text) {
        return Ok(());
    }
    Err(command_failure("sc.exe delete", &text))
}

fn query_service_exists() -> Result<bool, WakeguardError> {
    let output = run_sc(["query", SERVICE_NAME])?;
    if output.status.success() {
        return Ok(true);
    }
    let text = output_text(&output);
    if is_service_not_found(&text) {
        return Ok(false);
    }
    Err(command_failure("sc.exe query", &text))
}

fn wait_for_service_stopped(timeout: Duration) -> Result<(), WakeguardError> {
    let start = Instant::now();
    loop {
        let output = run_sc(["query", SERVICE_NAME])?;
        let text = output_text(&output);

        if is_service_not_found(&text) || is_service_not_active(&text) {
            return Ok(());
        }

        if let Some(state) = parse_service_state_code(&text) {
            if state == SERVICE_STATE_STOPPED {
                return Ok(());
            }
        }

        if start.elapsed() >= timeout {
            return Err(command_failure(
                "sc.exe query",
                "service stop timeout reached before STOPPED state",
            ));
        }

        thread::sleep(Duration::from_millis(300));
    }
}

pub(crate) fn resolve_install_source_path(
    binary_source: Option<&Path>,
    default_source: &Path,
) -> Result<PathBuf, WakeguardError> {
    let selected = binary_source.unwrap_or(default_source);
    if !selected.exists() {
        return Err(WakeguardError::InvalidConfig(format!(
            "install source binary path does not exist: {}",
            selected.display()
        )));
    }
    Ok(selected.to_path_buf())
}

fn launch_ui_process(onboarding: bool) -> Result<(), WakeguardError> {
    let current_exe = std::env::current_exe()?;
    let executable = if install_path::global_executable_path().exists() {
        install_path::global_executable_path()
    } else {
        current_exe
    };

    let mut command = Command::new(executable);
    command.arg("ui");
    if onboarding {
        command.arg("--onboarding");
    }
    let status = command.status().map_err(|err| WakeguardError::CommandFailed {
        command: "wakeguard ui".to_string(),
        details: err.to_string(),
    })?;
    if !status.success() {
        return Err(WakeguardError::CommandFailed {
            command: "wakeguard ui".to_string(),
            details: format!("ui process exited with status: {status}"),
        });
    }
    Ok(())
}

fn run_sc_checked(args: &[&str]) -> Result<(), WakeguardError> {
    let output = run_sc(args)?;
    if output.status.success() {
        return Ok(());
    }
    Err(command_failure("sc.exe", &output_text(&output)))
}

fn run_sc(args: impl IntoIterator<Item = impl AsRef<str>>) -> Result<std::process::Output, WakeguardError> {
    let args_vec: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    let output = Command::new("sc.exe").args(&args_vec).output()?;
    Ok(output)
}

fn output_text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn command_failure(command: &str, details: &str) -> WakeguardError {
    WakeguardError::CommandFailed {
        command: command.to_string(),
        details: details.trim().to_string(),
    }
}

pub(crate) fn is_service_not_found(text: &str) -> bool {
    let t = text.to_ascii_lowercase();
    t.contains("1060") || t.contains("does not exist")
}

pub(crate) fn is_service_already_running(text: &str) -> bool {
    text.to_ascii_lowercase().contains("1056")
}

pub(crate) fn is_service_not_active(text: &str) -> bool {
    text.to_ascii_lowercase().contains("1062")
}

pub(crate) fn parse_service_state_code(text: &str) -> Option<u32> {
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.to_ascii_uppercase().starts_with("STATE") {
            continue;
        }
        let mut parts = trimmed.split(':');
        let _left = parts.next();
        let right = parts.next()?.trim();
        let code = right.split_whitespace().next()?;
        if let Ok(parsed) = code.parse::<u32>() {
            return Some(parsed);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        is_service_already_running, is_service_not_active, is_service_not_found,
        parse_service_state_code, resolve_install_source_path,
    };
    use std::path::Path;

    #[test]
    fn resolve_install_source_defaults_to_current_exe_when_not_provided() {
        let current = std::env::current_exe().expect("current exe should resolve");
        let resolved = resolve_install_source_path(None, &current).expect("should default to current exe");
        assert_eq!(
            resolved.to_string_lossy().to_ascii_lowercase(),
            current.to_string_lossy().to_ascii_lowercase()
        );
    }

    #[test]
    fn resolve_install_source_prefers_explicit_binary_path() {
        let fallback = Path::new(r"C:\fallback\wakeguard.exe");
        let explicit = std::env::current_exe().expect("current exe should resolve");
        let source = resolve_install_source_path(Some(&explicit), fallback).expect("should accept source");
        assert_eq!(
            source.to_string_lossy().to_ascii_lowercase(),
            explicit.to_string_lossy().to_ascii_lowercase()
        );
    }

    #[test]
    fn resolve_install_source_rejects_missing_path() {
        let missing = Path::new(r"C:\definitely-missing\wakeguard.exe");
        let err = resolve_install_source_path(Some(missing), Path::new(r"C:\fallback.exe"))
            .expect_err("missing source should fail");
        assert!(format!("{err}").contains("does not exist"));
    }

    #[test]
    fn parse_service_not_found_code() {
        assert!(is_service_not_found("OpenService FAILED 1060"));
    }

    #[test]
    fn parse_service_running_code() {
        assert!(is_service_already_running("StartService FAILED 1056"));
    }

    #[test]
    fn parse_service_not_active_code() {
        assert!(is_service_not_active("ControlService FAILED 1062"));
    }

    #[test]
    fn parse_service_state_value_from_query_output() {
        let text = r#"
SERVICE_NAME: Wakeguard
        TYPE               : 10  WIN32_OWN_PROCESS
        STATE              : 3  STOP_PENDING
"#;
        assert_eq!(parse_service_state_code(text), Some(3));
    }
}
