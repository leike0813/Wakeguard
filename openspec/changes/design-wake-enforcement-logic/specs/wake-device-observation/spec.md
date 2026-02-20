## ADDED Requirements

### Requirement: New Wake-Capable Devices Must Be Detected

The system MUST detect newly observed wake-capable devices between scan cycles.

#### Scenario: Detect device newly appeared in current scan

- **GIVEN** previous scan result is available
- **AND** current scan includes a `stable_id` absent from previous result
- **WHEN** observation logic runs
- **THEN** the device is classified as newly detected

### Requirement: New Device Detection Must Emit Structured Events

The system MUST emit structured log events for newly detected wake-capable devices.

#### Scenario: Emit event for new device

- **GIVEN** a new wake-capable device is detected
- **WHEN** observation event is generated
- **THEN** event includes `stable_id`, `display_name`, and whitelist status

### Requirement: Observation Must Not Break Service Loop

The system MUST keep service scanning active even if one observation step fails.

#### Scenario: Observation processing fails for one device

- **GIVEN** observation processing for a device returns an error
- **WHEN** the scan cycle completes
- **THEN** the loop continues to next cycle
- **AND** failure is recorded in logs
