## ADDED Requirements

### Requirement: Wake Scan Results Must Be Deduplicated

The system MUST deduplicate scanned wake-capable devices by `stable_id`.

#### Scenario: Same device appears multiple times

- **GIVEN** scan output contains duplicate lines for one logical device
- **WHEN** parsing completes
- **THEN** only one device entry is retained

### Requirement: Scan Parsing Must Ignore Empty Lines

The parser MUST ignore blank lines in command output.

#### Scenario: Scan output contains empty rows

- **WHEN** parser processes output text
- **THEN** blank rows do not produce device entries

### Requirement: Scan Parsing MUST ignore localized no-device markers

The parser MUST ignore localized no-device marker lines emitted by `powercfg` (for example `none/no/n/a/无/沒有/没有`).

#### Scenario: Scan output contains localized no-device marker

- **WHEN** parser reads a line that represents no wake-capable device marker
- **THEN** parser does not emit any device entry for that line

### Requirement: Managed scan results MUST enforce strict identity

The scanner MUST only keep devices that can be resolved to strict identity (`VID/PID` or `VEN/DEV`) for managed enforcement input.

#### Scenario: Device lacks strict identity

- **GIVEN** a scanned device can only be resolved to name-based fallback identity
- **WHEN** strict identity filter is applied
- **THEN** that device is excluded from managed scan results
- **AND** a skip warning is logged with device context
