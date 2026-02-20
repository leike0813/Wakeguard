## ADDED Requirements

### Requirement: Worker Loop Must Emit Health Metrics

The worker loop MUST maintain and emit cycle-level and cumulative metrics for scan execution.

#### Scenario: Scan cycle completes

- **WHEN** one scan cycle finishes
- **THEN** metrics include scanned, disabled, observed, new_devices
- **AND** cumulative counters are updated

### Requirement: Scan Failures Must Be Counted

The worker loop MUST track scan and disable failures without stopping the loop.

#### Scenario: Disable action fails

- **WHEN** one disable action execution returns error
- **THEN** disable failure counter increments
- **AND** loop continues
