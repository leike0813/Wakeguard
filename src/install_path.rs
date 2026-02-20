use crate::error::WakeguardError;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};
use winreg::RegKey;

const SYSTEM_ENV_KEY_PATH: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
const PATH_VALUE_NAME: &str = "Path";
const GLOBAL_BIN_SUBDIR: &str = r"Wakeguard\bin";
const EXECUTABLE_NAME: &str = "wakeguard.exe";
const REMOVE_RETRY_ATTEMPTS: usize = 10;
const REMOVE_RETRY_DELAY: Duration = Duration::from_millis(400);

pub fn global_bin_dir() -> PathBuf {
    let program_files = std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
    program_files.join(GLOBAL_BIN_SUBDIR)
}

pub fn global_executable_path() -> PathBuf {
    global_bin_dir().join(EXECUTABLE_NAME)
}

pub fn copy_binary_to_global(source_binary: &Path) -> Result<PathBuf, WakeguardError> {
    if !source_binary.exists() {
        return Err(WakeguardError::InvalidConfig(format!(
            "source binary path does not exist: {}",
            source_binary.display()
        )));
    }

    let target = global_executable_path();
    if is_same_path_case_insensitive(source_binary, &target) {
        return Ok(target);
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source_binary, &target)?;
    Ok(target)
}

pub fn ensure_global_bin_in_system_path() -> Result<(), WakeguardError> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    update_path_value(&hklm, &global_bin_dir(), true)
}

pub fn remove_global_bin_from_system_path() -> Result<(), WakeguardError> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    update_path_value(&hklm, &global_bin_dir(), false)
}

pub fn remove_global_binary_with_fallback() -> Result<(), WakeguardError> {
    let executable = global_executable_path();
    if !executable.exists() {
        return Ok(());
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if is_same_path_case_insensitive(&current_exe, &executable) {
            tracing::warn!(
                path = %executable.display(),
                "global executable is current running process; scheduling delayed cleanup"
            );
            return schedule_delayed_cleanup(&executable);
        }
    }

    for attempt in 1..=REMOVE_RETRY_ATTEMPTS {
        match fs::remove_file(&executable) {
            Ok(()) => return Ok(()),
            Err(err)
                if matches!(err.kind(), ErrorKind::PermissionDenied | ErrorKind::WouldBlock)
                    && attempt < REMOVE_RETRY_ATTEMPTS =>
            {
                thread::sleep(REMOVE_RETRY_DELAY);
                continue;
            }
            Err(err) => {
                tracing::warn!(
                    path = %executable.display(),
                    error = %err,
                    "immediate executable removal failed; scheduling delayed cleanup"
                );
                return schedule_delayed_cleanup(&executable);
            }
        }
    }

    schedule_delayed_cleanup(&executable)
}

pub(crate) fn update_path_value(
    hive: &RegKey,
    target_dir: &Path,
    add: bool,
) -> Result<(), WakeguardError> {
    let env_key = hive
        .open_subkey_with_flags(SYSTEM_ENV_KEY_PATH, KEY_READ | KEY_WRITE)
        .map_err(|err| {
            WakeguardError::Registry(format!(
                "failed to open system environment key '{}': {err}",
                SYSTEM_ENV_KEY_PATH
            ))
        })?;

    let current_path: String = env_key.get_value(PATH_VALUE_NAME).unwrap_or_default();
    let updated = mutate_path_entries(&current_path, target_dir, add);

    if updated == current_path {
        return Ok(());
    }

    env_key
        .set_value(PATH_VALUE_NAME, &updated)
        .map_err(|err| WakeguardError::Registry(format!("failed to write PATH value: {err}")))?;

    Ok(())
}

pub(crate) fn mutate_path_entries(current: &str, target_dir: &Path, add: bool) -> String {
    let target = normalize_path_str(target_dir.to_string_lossy().as_ref());
    let mut entries: Vec<String> = current
        .split(';')
        .map(normalize_path_str)
        .filter(|entry| !entry.is_empty())
        .collect();

    entries.retain(|entry| !eq_path_case_insensitive(entry, &target));
    if add {
        entries.push(target);
    }
    entries.join(";")
}

fn normalize_path_str(value: &str) -> String {
    value.trim().trim_end_matches('\\').to_string()
}

fn eq_path_case_insensitive(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

pub(crate) fn is_same_path_case_insensitive(a: &Path, b: &Path) -> bool {
    let left = normalize_path_str(a.to_string_lossy().as_ref());
    let right = normalize_path_str(b.to_string_lossy().as_ref());
    eq_path_case_insensitive(&left, &right)
}

fn schedule_delayed_cleanup(executable_path: &Path) -> Result<(), WakeguardError> {
    let cleanup_script = build_cleanup_script(executable_path);

    Command::new("cmd.exe")
        .args(["/C", &cleanup_script])
        .spawn()
        .map_err(|err| {
            WakeguardError::CommandFailed {
                command: "cmd.exe /C <cleanup-script>".to_string(),
                details: err.to_string(),
            }
        })?;

    Ok(())
}

pub(crate) fn build_cleanup_script(executable_path: &Path) -> String {
    let exe: OsString = executable_path.as_os_str().to_os_string();
    let parent = executable_path
        .parent()
        .map(|p| p.as_os_str().to_os_string())
        .unwrap_or_else(|| OsString::from(""));

    format!(
        "ping 127.0.0.1 -n 3 > nul & del /f /q \"{}\" > nul 2>&1 & rmdir \"{}\" > nul 2>&1",
        PathBuf::from(&exe).display(),
        PathBuf::from(&parent).display()
    )
}

#[cfg(test)]
mod tests {
    use super::{build_cleanup_script, mutate_path_entries};
    use crate::install_path::is_same_path_case_insensitive;
    use std::path::Path;

    #[test]
    fn mutate_path_adds_missing_entry() {
        let updated = mutate_path_entries(r"C:\A;C:\B", Path::new(r"C:\Wakeguard\bin"), true);
        assert!(updated.contains(r"C:\Wakeguard\bin"));
    }

    #[test]
    fn mutate_path_deduplicates_case_insensitive() {
        let updated = mutate_path_entries(
            r"C:\Wakeguard\bin;C:\Other",
            Path::new(r"c:\wakeguard\bin"),
            true,
        );
        let parts: Vec<_> = updated.split(';').collect();
        assert_eq!(parts.iter().filter(|p| p.eq_ignore_ascii_case("C:\\Wakeguard\\bin")).count(), 1);
    }

    #[test]
    fn mutate_path_removes_entry() {
        let updated =
            mutate_path_entries(r"C:\Wakeguard\bin;C:\Other", Path::new(r"C:\Wakeguard\bin"), false);
        assert!(!updated.to_ascii_lowercase().contains("wakeguard\\bin"));
        assert!(updated.contains(r"C:\Other"));
    }

    #[test]
    fn cleanup_script_contains_del_and_rmdir() {
        let script = build_cleanup_script(Path::new(r"C:\Program Files\Wakeguard\bin\wakeguard.exe"));
        assert!(script.contains("del /f /q"));
        assert!(script.contains("rmdir"));
    }

    #[test]
    fn same_path_compare_is_case_insensitive() {
        assert!(is_same_path_case_insensitive(
            Path::new(r"C:\Program Files\Wakeguard\bin\wakeguard.exe"),
            Path::new(r"c:\program files\wakeguard\bin\wakeguard.exe")
        ));
    }
}
