## ADDED Requirements

### Requirement: Command MUST launch main UI
The system MUST provide a `wakeguard ui` command to launch the whitelist management main interface.

#### Scenario: Execute ui command
- **WHEN** user runs `wakeguard ui`
- **THEN** system launches the main UI window

### Requirement: UI launch MUST work while service is running
The system MUST allow opening UI independently from the service runtime loop.

#### Scenario: Service already running
- **WHEN** service process is active and user runs `wakeguard ui`
- **THEN** UI opens and reads current managed-device state
