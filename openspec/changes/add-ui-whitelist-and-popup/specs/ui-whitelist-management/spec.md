## ADDED Requirements

### Requirement: UI MUST show all managed devices
The system MUST provide a whitelist management page that lists all devices currently under management scope.

#### Scenario: Open whitelist page
- **WHEN** user runs `wakeguard ui` and opens the main page
- **THEN** system shows a device list with each device's `stable_id` and display name

### Requirement: UI MUST show whitelist membership status
The system MUST indicate whether each listed device is currently in whitelist.

#### Scenario: Render whitelist status
- **WHEN** device list is rendered
- **THEN** each row shows a boolean whitelist status for that device

### Requirement: UI MUST allow checkbox-based whitelist update
The system MUST allow users to add or remove whitelist entries via checkbox interaction.

#### Scenario: Check device to add whitelist
- **WHEN** user checks a non-whitelisted device row
- **THEN** system persists that device `stable_id` into whitelist storage

#### Scenario: Uncheck device to remove whitelist
- **WHEN** user unchecks a whitelisted device row
- **THEN** system removes that device `stable_id` from whitelist storage

### Requirement: UI MUST show wake-disable status
The system MUST display whether each device wake capability is currently disabled.

#### Scenario: Render disabled status column
- **WHEN** device list is displayed
- **THEN** each row includes a wake-disabled status indicator
