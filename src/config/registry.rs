use crate::error::WakeguardError;
use std::collections::HashSet;
use std::io::ErrorKind;
use winreg::enums::{HKEY_LOCAL_MACHINE, REG_MULTI_SZ};
use winreg::types::FromRegValue;
use winreg::{RegKey, RegValue};

pub const ROOT_KEY_PATH: &str = r"Software\Wakeguard";
pub const WHITELIST_VALUE_NAME: &str = "Whitelist";
pub const KNOWN_DEVICES_VALUE_NAME: &str = "KnownDevices";
pub const PENDING_PROMPTS_VALUE_NAME: &str = "PendingPrompts";
pub const DEVICE_SNAPSHOT_VALUE_NAME: &str = "DeviceSnapshot";

const ALLOWED_VALUE_NAMES: [&str; 4] = [
    WHITELIST_VALUE_NAME,
    KNOWN_DEVICES_VALUE_NAME,
    PENDING_PROMPTS_VALUE_NAME,
    DEVICE_SNAPSHOT_VALUE_NAME,
];
const ALLOWED_SUBKEYS: [&str; 0] = [];

pub fn load_whitelist() -> Result<HashSet<String>, WakeguardError> {
    let hive = RegKey::predef(HKEY_LOCAL_MACHINE);
    load_whitelist_from_hive(&hive, ROOT_KEY_PATH, WHITELIST_VALUE_NAME)
}

pub fn save_whitelist(whitelist: &HashSet<String>) -> Result<(), WakeguardError> {
    let hive = RegKey::predef(HKEY_LOCAL_MACHINE);
    save_whitelist_to_hive(&hive, ROOT_KEY_PATH, WHITELIST_VALUE_NAME, whitelist)
}

pub fn add_to_whitelist(entries: impl IntoIterator<Item = String>) -> Result<(), WakeguardError> {
    let mut whitelist = load_whitelist()?;
    whitelist.extend(
        entries
            .into_iter()
            .map(|entry| entry.trim().to_ascii_lowercase())
            .filter(|entry| !entry.is_empty()),
    );
    save_whitelist(&whitelist)
}

pub fn ensure_default_whitelist_empty() -> Result<(), WakeguardError> {
    let hive = RegKey::predef(HKEY_LOCAL_MACHINE);
    ensure_default_whitelist_empty_in_hive(&hive, ROOT_KEY_PATH, WHITELIST_VALUE_NAME)
}

pub fn sanitize_registry_schema() -> Result<(), WakeguardError> {
    let hive = RegKey::predef(HKEY_LOCAL_MACHINE);
    sanitize_registry_schema_in_hive(&hive, ROOT_KEY_PATH)
}

pub(crate) fn load_whitelist_from_hive(
    hive: &RegKey,
    key_path: &str,
    value_name: &str,
) -> Result<HashSet<String>, WakeguardError> {
    let key = match hive.open_subkey(key_path) {
        Ok(key) => key,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            tracing::info!(key = key_path, "registry key missing; treating whitelist as empty");
            return Ok(HashSet::new());
        }
        Err(err) => {
            return Err(WakeguardError::Registry(format!(
                "failed to open registry key '{key_path}': {err}"
            )));
        }
    };

    let raw = match key.get_raw_value(value_name) {
        Ok(raw) => raw,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            tracing::info!(
                key = key_path,
                value = value_name,
                "registry whitelist value missing; treating as empty"
            );
            return Ok(HashSet::new());
        }
        Err(err) => {
            return Err(WakeguardError::Registry(format!(
                "failed to read registry value '{value_name}': {err}"
            )));
        }
    };

    parse_whitelist_reg_value(&raw)
}

pub(crate) fn parse_whitelist_reg_value(raw: &RegValue) -> Result<HashSet<String>, WakeguardError> {
    if raw.vtype != REG_MULTI_SZ {
        return Err(WakeguardError::InvalidConfig(format!(
            "registry value type mismatch: expected REG_MULTI_SZ, got {:?}",
            raw.vtype
        )));
    }

    let values: Vec<String> = FromRegValue::from_reg_value(raw).map_err(|err| {
        WakeguardError::InvalidConfig(format!("failed to parse whitelist entries: {err}"))
    })?;

    Ok(values
        .into_iter()
        .map(|entry| entry.trim().to_ascii_lowercase())
        .filter(|entry| !entry.is_empty())
        .collect())
}

pub(crate) fn save_whitelist_to_hive(
    hive: &RegKey,
    key_path: &str,
    value_name: &str,
    whitelist: &HashSet<String>,
) -> Result<(), WakeguardError> {
    let (key, _) = hive.create_subkey(key_path).map_err(|err| {
        WakeguardError::Registry(format!("failed to create/open registry key '{key_path}': {err}"))
    })?;

    let entries: Vec<String> = whitelist.iter().cloned().collect();
    key.set_value(value_name, &entries)
        .map_err(|err| WakeguardError::Registry(format!("failed to save whitelist value: {err}")))?;
    Ok(())
}

pub(crate) fn ensure_default_whitelist_empty_in_hive(
    hive: &RegKey,
    key_path: &str,
    value_name: &str,
) -> Result<(), WakeguardError> {
    let (key, _) = hive.create_subkey(key_path).map_err(|err| {
        WakeguardError::Registry(format!("failed to create/open registry key '{key_path}': {err}"))
    })?;

    if key.get_raw_value(value_name).is_ok() {
        return Ok(());
    }

    let empty: Vec<String> = Vec::new();
    key.set_value(value_name, &empty)
        .map_err(|err| WakeguardError::Registry(format!("failed to initialize empty whitelist: {err}")))?;
    Ok(())
}

pub(crate) fn sanitize_registry_schema_in_hive(
    hive: &RegKey,
    key_path: &str,
) -> Result<(), WakeguardError> {
    let (key, _) = hive.create_subkey(key_path).map_err(|err| {
        WakeguardError::Registry(format!("failed to create/open registry key '{key_path}': {err}"))
    })?;

    let to_remove_values: Vec<String> = key
        .enum_values()
        .filter_map(|item| item.ok().map(|(name, _)| name))
        .filter(|name| !ALLOWED_VALUE_NAMES.iter().any(|allowed| allowed.eq_ignore_ascii_case(name)))
        .collect();

    for value_name in to_remove_values {
        key.delete_value(&value_name).map_err(|err| {
            WakeguardError::Registry(format!("failed to delete illegal value '{value_name}': {err}"))
        })?;
    }

    let to_remove_subkeys: Vec<String> = key
        .enum_keys()
        .filter_map(Result::ok)
        .filter(|name| !ALLOWED_SUBKEYS.iter().any(|allowed| allowed.eq_ignore_ascii_case(name)))
        .collect();

    for subkey_name in to_remove_subkeys {
        key.delete_subkey_all(&subkey_name).map_err(|err| {
            WakeguardError::Registry(format!(
                "failed to delete illegal subkey '{subkey_name}': {err}"
            ))
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_default_whitelist_empty_in_hive, load_whitelist_from_hive, parse_whitelist_reg_value,
        sanitize_registry_schema_in_hive, save_whitelist_to_hive, WHITELIST_VALUE_NAME,
    };
    use crate::error::WakeguardError;
    use std::collections::HashSet;
    use std::time::{SystemTime, UNIX_EPOCH};
    use winreg::enums::{HKEY_CURRENT_USER, REG_SZ};
    use winreg::types::ToRegValue;
    use winreg::{RegKey, RegValue};

    fn unique_test_key_path(test_name: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time must be monotonic")
            .as_nanos();
        format!(r"Software\Wakeguard\Tests\{test_name}-{nanos}")
    }

    #[test]
    fn parse_empty_whitelist_returns_empty_set() {
        let raw = Vec::<String>::new().to_reg_value();
        let whitelist = parse_whitelist_reg_value(&raw).expect("parse should succeed");
        assert!(whitelist.is_empty());
    }

    #[test]
    fn parse_invalid_type_returns_error() {
        let raw = RegValue {
            bytes: "not-multi".to_string().to_reg_value().bytes,
            vtype: REG_SZ,
        };

        let err = parse_whitelist_reg_value(&raw).expect_err("must reject non REG_MULTI_SZ");
        assert!(matches!(err, WakeguardError::InvalidConfig(_)));
    }

    #[test]
    fn missing_key_returns_empty_set() {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = unique_test_key_path("missing-key");

        let whitelist = load_whitelist_from_hive(&hkcu, &path, WHITELIST_VALUE_NAME)
            .expect("missing key should be downgraded to empty whitelist");
        assert!(whitelist.is_empty());
    }

    #[test]
    fn missing_value_returns_empty_set() {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = unique_test_key_path("missing-value");
        let (key, _) = hkcu
            .create_subkey(&path)
            .expect("test key should be created");

        let whitelist = load_whitelist_from_hive(&hkcu, &path, WHITELIST_VALUE_NAME)
            .expect("missing value should be downgraded to empty whitelist");
        assert!(whitelist.is_empty());

        drop(key);
        let _ = hkcu.delete_subkey_all(&path);
    }

    #[test]
    fn load_multi_sz_whitelist_normalizes_entries() {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = unique_test_key_path("valid-whitelist");
        let (key, _) = hkcu
            .create_subkey(&path)
            .expect("test key should be created");

        let entries = vec![" SYS:Keyboard-1 ".to_string(), "name:Mouse".to_string()];
        key.set_value(WHITELIST_VALUE_NAME, &entries)
            .expect("test whitelist should be stored");

        let whitelist = load_whitelist_from_hive(&hkcu, &path, WHITELIST_VALUE_NAME)
            .expect("whitelist should be read");
        assert!(whitelist.contains("sys:keyboard-1"));
        assert!(whitelist.contains("name:mouse"));

        drop(key);
        let _ = hkcu.delete_subkey_all(&path);
    }

    #[test]
    fn ensure_default_whitelist_initializes_empty_value() {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = unique_test_key_path("init-empty-whitelist");
        ensure_default_whitelist_empty_in_hive(&hkcu, &path, WHITELIST_VALUE_NAME)
            .expect("should initialize empty whitelist");

        let loaded = load_whitelist_from_hive(&hkcu, &path, WHITELIST_VALUE_NAME)
            .expect("whitelist should load");
        assert!(loaded.is_empty());
        let _ = hkcu.delete_subkey_all(&path);
    }

    #[test]
    fn save_whitelist_and_load_roundtrip() {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = unique_test_key_path("save-roundtrip");
        let whitelist = HashSet::from(["sys:keyboard".to_string(), "sys:mouse".to_string()]);

        save_whitelist_to_hive(&hkcu, &path, WHITELIST_VALUE_NAME, &whitelist)
            .expect("save whitelist should succeed");
        let loaded = load_whitelist_from_hive(&hkcu, &path, WHITELIST_VALUE_NAME)
            .expect("load whitelist should succeed");
        assert_eq!(loaded, whitelist);
        let _ = hkcu.delete_subkey_all(&path);
    }

    #[test]
    fn sanitize_registry_removes_illegal_values_and_subkeys() {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = unique_test_key_path("sanitize");
        let (key, _) = hkcu.create_subkey(&path).expect("create key");
        key.set_value("IllegalValue", &"x").expect("set illegal value");
        key.set_value(WHITELIST_VALUE_NAME, &Vec::<String>::new())
            .expect("set whitelist");
        key.create_subkey("UnexpectedSubkey")
            .expect("create illegal subkey");

        sanitize_registry_schema_in_hive(&hkcu, &path).expect("sanitize should succeed");

        assert!(key.get_raw_value(WHITELIST_VALUE_NAME).is_ok());
        assert!(key.get_raw_value("IllegalValue").is_err());
        assert!(key.open_subkey("UnexpectedSubkey").is_err());

        drop(key);
        let _ = hkcu.delete_subkey_all(&path);
    }
}
