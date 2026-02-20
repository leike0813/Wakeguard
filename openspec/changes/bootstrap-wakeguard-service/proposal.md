## Why

Wakeguard 目前只有目标描述，尚未形成可编译、可扩展的 Rust 工程结构，也没有可执行的 OpenSpec 变更基线。  
在进入业务实现前，需要先完成工程骨架和变更文档，降低后续实现的不确定性。

## What Changes

- 初始化 Rust 项目骨架，明确模块边界（service/device/config/policy/logging）。
- 约定 Windows Service 作为唯一服务形态入口。
- 明确白名单采用注册表 `HKLM\Software\Wakeguard`。
- 建立首个 OpenSpec change 的 proposal/design/spec/tasks 文档。

## Capabilities

### New Capabilities

- **Project Scaffolding Baseline**: 提供可编译的项目骨架和统一入口命令。
- **Specification Baseline**: 提供首个可执行的 OpenSpec 变更文档，作为后续业务实现依据。

### Non-goals

- 不实现具体设备枚举与禁用逻辑。
- 不实现注册表真实读写逻辑。
- 不实现服务安装/卸载的真实系统调用。

## Impact

- `Cargo.toml`: 新增项目与依赖定义。
- `src/main.rs`: 新增程序入口和命令分发。
- `src/service/*`: 新增服务层骨架。
- `src/device/*`: 新增设备领域骨架。
- `src/config/*`: 新增注册表配置骨架。
- `src/policy/*`: 新增策略层骨架。
- `src/logging.rs`: 新增日志初始化。
- `openspec/config.yaml`: 补充项目上下文和制品规则。
