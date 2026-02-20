## ADDED Requirements

### Requirement: Installed state MUST provide no-path CLI command availability
The system MUST make `wakeguard` executable available from shell without full path while service is installed.

#### Scenario: Invoke command in installed state
- **WHEN** service is installed and user executes `wakeguard ui` or `wakeguard uninstall`
- **THEN** command is resolved via global PATH entry

### Requirement: Uninstalled state MUST revoke no-path CLI availability
The system MUST revoke global shell command availability after uninstall.

#### Scenario: Invoke command in uninstalled state
- **WHEN** uninstall completed and user executes `wakeguard`
- **THEN** shell cannot resolve command from global PATH
