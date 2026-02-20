## ADDED Requirements

### Requirement: First install MUST validate registry structure
The system MUST validate existing Wakeguard registry entries against an allowed key/value whitelist on first install.

#### Scenario: First install detects existing registry keys
- **WHEN** installation identifies existing Wakeguard registry root
- **THEN** system checks all keys and values against allowed schema

### Requirement: First install MUST remove illegal registry entries
The system MUST remove registry entries that do not match allowed Wakeguard schema on first install.

#### Scenario: Illegal entry is found
- **WHEN** validation finds an unknown key or value
- **THEN** system removes that illegal registry entry
- **AND** keeps allowed entries unchanged

### Requirement: Default whitelist MUST be empty on first install
The system MUST initialize whitelist as empty by default during first install.

#### Scenario: Fresh first install
- **WHEN** installation runs on first-use environment
- **THEN** whitelist is initialized with zero devices
