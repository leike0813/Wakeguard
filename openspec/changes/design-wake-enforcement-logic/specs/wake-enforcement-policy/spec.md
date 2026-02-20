## ADDED Requirements

### Requirement: Non-Whitelisted Wake Devices Must Be Disabled

The system MUST evaluate all wake-capable devices and disable wake capability for devices not present in the whitelist.

#### Scenario: Disable device not in whitelist

- **GIVEN** wake-capable devices are discovered
- **AND** a device's `stable_id` is not in whitelist
- **WHEN** policy evaluation runs
- **THEN** a disable action is generated for that device

### Requirement: Whitelisted Devices Must Remain Allowed

The system MUST preserve wake capability for devices explicitly present in the whitelist.

#### Scenario: Keep whitelisted device enabled

- **GIVEN** a device `stable_id` exists in whitelist
- **WHEN** policy evaluation runs
- **THEN** no disable action is generated for that device

### Requirement: Low-Confidence Identity Must Be Observed

The system MUST avoid aggressive disabling when identity confidence is low.

#### Scenario: Device identity falls back to low-confidence path

- **GIVEN** a device cannot be resolved by high-confidence identity sources
- **WHEN** policy evaluation runs
- **THEN** the device is marked for observation
- **AND** an observation event is emitted
