## ADDED Requirements

### Requirement: System MUST prompt for first-seen unrecorded device
The system MUST show a dialog prompt when a newly inserted device is detected and not found in recorded device history.

#### Scenario: First insertion of unknown device
- **WHEN** service detects a device `stable_id` not in recorded device list
- **THEN** system creates one pending prompt event for that device
- **AND** UI displays a dialog asking whether to add it to whitelist

### Requirement: System MUST NOT reprompt recorded device
The system MUST avoid repeated dialog prompts for devices already recorded previously.

#### Scenario: Reinsert previously recorded device
- **WHEN** a previously recorded device is unplugged and inserted again
- **THEN** system does not create a new prompt event for that `stable_id`

### Requirement: Prompt decision MUST be persisted
The system MUST persist user choice from dialog interaction.

#### Scenario: User accepts device into whitelist
- **WHEN** user clicks confirm in prompt dialog
- **THEN** system adds the device `stable_id` to whitelist

#### Scenario: User declines prompt
- **WHEN** user dismisses or declines prompt dialog
- **THEN** system keeps device out of whitelist
- **AND** system still marks device as recorded to avoid repeat prompt
