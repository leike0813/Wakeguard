## ADDED Requirements

### Requirement: Uninstall MUST remove Wakeguard service
The system MUST remove the `Wakeguard` service when uninstall command is executed.

#### Scenario: Run uninstall command
- **WHEN** user executes `wakeguard uninstall`
- **THEN** system stops `Wakeguard` service if running
- **AND** system removes `Wakeguard` service registration

### Requirement: Uninstall MUST preserve registry data
The system MUST keep Wakeguard registry configuration after uninstall.

#### Scenario: Uninstall completes
- **WHEN** service uninstall finishes
- **THEN** existing Wakeguard registry keys remain intact

### Requirement: Uninstall MUST remove global PATH exposure
The system MUST remove `C:\Program Files\Wakeguard\bin` from system PATH during uninstall.

#### Scenario: PATH cleanup
- **WHEN** uninstall completes
- **THEN** system PATH no longer contains Wakeguard global bin directory

### Requirement: Uninstall MUST remove copied executable
The system MUST remove copied Wakeguard executable from global install directory after uninstall.

#### Scenario: Executable in-use during uninstall
- **WHEN** uninstall runs while copied executable is still in-use
- **THEN** system schedules delayed cleanup after process exit
- **AND** final state removes copied executable from `C:\Program Files\Wakeguard\bin`

### Requirement: Uninstalled state MUST not keep no-path CLI availability
The system MUST ensure `wakeguard` cannot be invoked from shell without explicit path once uninstall is complete.

#### Scenario: Shell invocation after uninstall
- **WHEN** uninstall is completed and user runs `wakeguard`
- **THEN** shell cannot resolve command from global PATH
