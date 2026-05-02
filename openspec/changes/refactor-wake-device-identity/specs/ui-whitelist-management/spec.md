## MODIFIED Requirements

### Requirement: UI MUST show all managed devices

The system MUST provide a whitelist management page that lists all managed device families currently under governance scope.

#### Scenario: Open whitelist page

- **WHEN** user runs `wakeguard ui` and opens the main page
- **THEN** system shows a family list with each row's `stable_id` and representative display name
- **AND** that representative display name is allowed to come from the resolved parent-chain family rather than a raw HID child name

### Requirement: UI MUST show whitelist membership status

The system MUST indicate whether each listed family is currently in whitelist.

#### Scenario: Render whitelist status

- **WHEN** family list is rendered
- **THEN** each row shows a boolean whitelist status for that family

### Requirement: UI MUST show wake-disable status

The system MUST display whether each managed family currently has wake capability disabled at family level.

#### Scenario: Render family disabled status

- **WHEN** family list is displayed
- **THEN** each row includes a wake-disabled status indicator for that family
- **AND** the indicator reflects whether any wake-enabled member remains in that family
