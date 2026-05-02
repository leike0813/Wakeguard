## ADDED Requirements

### Requirement: Managed wake devices MUST be identified by family ID

The system MUST use a family-level `stable_id` as the managed identity for wake-capable devices.

#### Scenario: Resolve device into managed family identity

- **GIVEN** a wake-capable device can be resolved to `VID/PID` or `VEN/DEV`
- **WHEN** identity resolution runs
- **THEN** the resulting `stable_id` is a family ID
- **AND** that family ID is used as the managed identity

### Requirement: HID wake leaves MUST resolve through device topology

The system MUST bind wake-capable HID leaves to real PnP leaf devnodes and resolve their family by walking the device parent chain.

#### Scenario: Resolve HID child through parent chain

- **GIVEN** a wake-capable HID child node is observed
- **AND** the child itself or one of its ancestors exposes `VID/PID`
- **WHEN** family resolution runs
- **THEN** the system resolves the child into one `vidpid:*` family
- **AND** that family becomes the managed identity

#### Scenario: Fall back to parent `VEN/DEV` when no `VID/PID` exists

- **GIVEN** a wake-capable leaf has no usable `VID/PID`
- **AND** one of its ancestors exposes `VEN/DEV`
- **WHEN** family resolution runs
- **THEN** the system resolves the leaf into one `sys:*` family

#### Scenario: Skip unmanaged parent chain

- **GIVEN** a wake-capable leaf and its entire parent chain expose neither `VID/PID` nor `VEN/DEV`
- **WHEN** family resolution runs
- **THEN** the leaf is excluded from managed enforcement
- **AND** the skip log includes the failure stage and parent-chain summary

### Requirement: Name-only fallback MUST NOT enter managed enforcement

The system MUST treat `name:*` fallback identities as diagnostic-only and exclude them from managed enforcement.

#### Scenario: Device only has name-based fallback

- **GIVEN** a wake-capable device cannot be resolved to `vidpid:*` or `sys:*`
- **WHEN** identity resolution completes
- **THEN** the device is excluded from managed enforcement
- **AND** a diagnostic warning is emitted

### Requirement: Devices in the same family MUST be aggregated

The system MUST aggregate multiple observed devices into one managed object when they share the same family ID.

#### Scenario: Multiple names resolve to one family

- **GIVEN** multiple wake-capable device names resolve to the same family ID
- **WHEN** aggregation runs
- **THEN** the system keeps one managed family object for that ID
- **AND** retains the member names as family members
