## Why

项目已完成工程骨架，但核心业务逻辑仍未定义，尤其是“设备稳定标识、白名单匹配、禁用策略、增量监控”之间的协作规则。  
在进入实现前，需要先形成一致的业务设计基线，降低后续返工和行为歧义。

## What Changes

- 设计唤醒治理主流程：设备发现 → 标识解析 → 白名单判定 → 策略执行 → 结果记录。
- 定义设备稳定标识规则，确保同一物理设备在不同端口下可被同一身份识别。
- 定义注册表白名单结构与读取约束（`HKLM\Software\Wakeguard`）。
- 定义新增可唤醒设备的检测与告警语义（首阶段仅记录事件，不做交互弹窗）。
- 形成业务逻辑实现任务清单，但不进入代码实现。

## Capabilities

### New Capabilities

- `wake-enforcement-policy`: 对可唤醒设备执行白名单策略并生成禁用动作。
- `wake-device-observation`: 持续检测可唤醒设备变化并产生日志事件。

### Modified Capabilities

- 无

## Non-goals

- 不实现具体业务代码。
- 不实现 Windows 弹窗或托盘交互。
- 不覆盖跨平台行为（仅面向 Windows）。

## Impact

- `src/device/*`: 后续将承载设备发现与标识解析逻辑。
- `src/policy/*`: 后续将承载策略决策与动作计划逻辑。
- `src/config/registry.rs`: 后续将承载白名单读取逻辑。
- `src/service/worker.rs`: 后续将承载扫描调度与执行编排逻辑。
- `openspec/changes/design-wake-enforcement-logic/*`: 本次产出业务设计与规格基线。
