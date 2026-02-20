## ADDED Requirements

### Requirement: Disable Retries Must Respect Cooldown

The system MUST avoid retrying disable actions for the same `stable_id` until cooldown cycles pass.

#### Scenario: Device disabled in previous cycle

- **GIVEN** a disable attempt was made for a device recently
- **WHEN** cooldown threshold is not reached
- **THEN** disable retry is skipped for that cycle

### Requirement: Disable Failures Must Be Observable

The system MUST emit error logs with device context when disable execution fails.

#### Scenario: Disable command returns failure

- **WHEN** command execution fails
- **THEN** log includes device name, stable_id, and failure reason
