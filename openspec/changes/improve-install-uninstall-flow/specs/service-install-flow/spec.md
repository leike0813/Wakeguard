## ADDED Requirements

### Requirement: Service MUST support command-based installation
The system MUST allow users to install Wakeguard service via command invocation.

#### Scenario: Run install command
- **WHEN** user executes `wakeguard install`
- **THEN** system installs the `Wakeguard` Windows service
- **AND** service is started after successful installation

### Requirement: Install MUST support default source binary path
The system MUST use current executing `wakeguard.exe` as install source when explicit binary path is not provided.

#### Scenario: Fresh install without binary path
- **WHEN** user executes install command in non-installed state without binary path argument
- **THEN** system selects current executing `wakeguard.exe` as installation source
- **AND** installation continues if selected executable is valid

#### Scenario: Install with explicit binary path
- **WHEN** user executes install command with explicit binary path argument
- **THEN** system validates explicit path points to a usable executable
- **AND** explicit path overrides default source

### Requirement: Install MUST copy executable into global non-system directory
The system MUST copy executable into `C:\Program Files\Wakeguard\bin\wakeguard.exe` during install.

#### Scenario: Install copy stage
- **WHEN** install command starts from valid source binary path
- **THEN** executable is copied into `C:\Program Files\Wakeguard\bin`
- **AND** executable is not copied into Windows system directories

### Requirement: Install MUST make wakeguard command globally reachable
The system MUST add `C:\Program Files\Wakeguard\bin` into system PATH during installation.

#### Scenario: Install PATH injection
- **WHEN** install succeeds
- **THEN** system PATH contains `C:\Program Files\Wakeguard\bin`
- **AND** user can invoke `wakeguard` from shell without full path

### Requirement: Installation MUST launch UI onboarding
The system MUST attempt to launch the main UI after successful installation.

#### Scenario: Install succeeds
- **WHEN** service installation and startup complete successfully
- **THEN** system invokes `wakeguard ui` for onboarding
- **AND** installation result remains successful even if UI launch fails
