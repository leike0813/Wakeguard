## ADDED Requirements

### Requirement: Skeleton Project Must Compile

The project MUST provide a Rust skeleton that compiles without implementing device wake business behavior.

#### Scenario: Compile skeleton successfully

- **WHEN** a developer runs `cargo check`
- **THEN** the project compiles successfully
- **AND** no business wake-control side effects are triggered

### Requirement: OpenSpec Baseline Must Exist

The change MUST include proposal, design, tasks, and at least one capability spec for the first milestone.

#### Scenario: Validate change artifacts

- **WHEN** a developer runs `openspec validate --changes`
- **THEN** the change artifacts are structurally valid
- **AND** the milestone scope clearly excludes business implementation
