## ADDED Requirements

### Requirement: First install MUST prompt user to configure whitelist
The system MUST prompt user to manage whitelist after first successful installation.

#### Scenario: First install onboarding starts
- **WHEN** installation completes and UI is opened
- **THEN** user sees onboarding prompt to select whitelist devices

### Requirement: Onboarding MUST support per-device selection
The system MUST allow users to choose devices individually for whitelist inclusion.

#### Scenario: User selects specific devices
- **WHEN** user marks subset of detected devices
- **THEN** system adds selected devices to whitelist only

### Requirement: Onboarding MUST support one-click add all
The system MUST provide a one-click action to add all supported detected devices into whitelist.

#### Scenario: User chooses add-all
- **WHEN** user clicks "add all devices"
- **THEN** system adds all currently supported detected devices to whitelist
