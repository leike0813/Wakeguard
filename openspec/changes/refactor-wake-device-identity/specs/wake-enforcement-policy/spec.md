## MODIFIED Requirements

### Requirement: Non-Whitelisted Wake Devices Must Be Disabled

The system MUST evaluate all managed wake-capable device families and disable wake capability for families not present in the whitelist.

#### Scenario: Disable non-whitelisted family

- **GIVEN** managed wake-capable device families are discovered
- **AND** a family `stable_id` is not in whitelist
- **WHEN** policy evaluation runs
- **THEN** a disable action is generated for that family

#### Scenario: Disable all wake-enabled members of one family

- **GIVEN** a non-whitelisted family currently has multiple wake-enabled member names
- **WHEN** disable execution runs
- **THEN** the system attempts disable for each wake-enabled member name in that family
- **AND** those member names correspond to the current `powercfg` execution names bound to that family in the same scan

### Requirement: Whitelisted Devices Must Remain Allowed

The system MUST treat whitelist membership as a family-level exemption from automatic disable actions.

#### Scenario: Keep whitelisted family exempt from disable

- **GIVEN** a family `stable_id` exists in whitelist
- **WHEN** policy evaluation runs
- **THEN** no disable action is generated for that family

#### Scenario: Whitelist does not auto-restore wake

- **GIVEN** a family is already in whitelist
- **AND** its wake capability was disabled before whitelist membership
- **WHEN** the worker loop runs
- **THEN** the system does not issue a wake-enable action automatically
