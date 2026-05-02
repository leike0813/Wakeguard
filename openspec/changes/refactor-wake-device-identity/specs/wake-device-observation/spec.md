## MODIFIED Requirements

### Requirement: New Wake-Capable Devices Must Be Detected

The system MUST detect newly observed managed wake-capable device families between scan cycles.

#### Scenario: Detect family newly appeared in current scan

- **GIVEN** previous scan result is available
- **AND** current scan includes a family `stable_id` absent from previous result
- **WHEN** observation logic runs
- **THEN** the family is classified as newly detected

### Requirement: New Device Detection Must Emit Structured Events

The system MUST emit structured log events for newly detected managed wake-capable device families.

#### Scenario: Emit event for new family

- **GIVEN** a new managed wake-capable family is detected
- **WHEN** observation event is generated
- **THEN** event includes family `stable_id`, representative display name, and whitelist status

### Requirement: Unmanaged fallback devices MUST be observable

The system MUST log unmanaged fallback devices that were skipped from family-level enforcement.

#### Scenario: Skip name-only fallback device

- **GIVEN** a wake-capable device only resolves to `name:*`
- **WHEN** scanning completes
- **THEN** the device is not included in managed family results
- **AND** the skip reason is recorded in logs

#### Scenario: Skip device when parent-chain family resolution fails

- **GIVEN** a wake-capable leaf cannot be resolved to `vidpid:*` or `sys:*` through its parent chain
- **WHEN** scanning completes
- **THEN** the device is not included in managed family results
- **AND** the log includes its current display name, leaf instance ID, failure stage, and parent-chain summary
