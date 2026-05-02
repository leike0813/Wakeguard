## Context

当前系统虽然已经使用 `vidpid:*` 和 `sys:*` 作为展示与白名单值，但它们并不是直接从 `powercfg` 输出稳定获得的。  
如果直接拿 generic HID 显示名做候选匹配，就会在同名设备很多时产生歧义；如果简单地“有歧义就跳过”，又会把大量真正需要治理的 HID 设备整体排除掉。  
本次变更的目标不是进一步强化实例级识别，而是直接改变治理对象：只治理设备族群，不治理具体实例；同时把族群来源改成“wake-capable 叶子 devnode 的父链解析”。

## Goals / Non-Goals

**Goals**

- 明确 `stable_id` 的业务语义为“治理族群 ID”。
- 停止把某个设备实例作为最小治理对象。
- 将白名单、禁用执行、UI 展示和新设备检测统一到族群级语义。
- 通过收缩治理粒度来消除实例映射漂移导致的误禁。

**Non-Goals**

- 不承诺识别某一物理设备实例。
- 不通过序列号、Container ID 等方案建立新的实例级模型。
- 不重做安装/卸载流程。
- 不实现 GUI 与弹窗交互。

## Business Flow

1. `Powercfg Names`：扫描当前 `wake_programmable` 名称集合，作为当前执行名与 UI 成员名来源。
2. `Topology Snapshot`：枚举 present devnodes，收集 `InstanceId`、`Parent`、`ContainerId`、显示名与硬件 ID。
3. `Wake Leaf Snapshot`：读取 `MSPower_DeviceWakeEnable`，获得 wake-capable 叶子 devnode 与其 `Enable` 状态。
4. `Family Resolution`：将每个 wake-capable 叶子沿父链解析为治理族群 ID（`vidpid:*` 优先，其次 `sys:*`）。
5. `Family Aggregation`：把属于同一族群的多个当前显示名归并为一个治理对象。
6. `Whitelist Load`：读取族群级白名单。
7. `Policy Evaluation`：对每个族群生成 `Allow` / `Disable` / `Observe` 决策。
8. `Action Execution`：对每个需禁用族群的所有 wake-enabled 成员逐个执行禁用。
9. `Observation Emit`：输出族群级新增事件、父链解析日志、跳过事件和决策日志。

## Module Boundaries

- `device::powercfg`
  - 负责扫描 `powercfg` 当前名称集合。
  - 负责把当前名称与 wake-capable 叶子绑定，并归并为族群级对象。
  - 负责对某个族群当前命中的所有名称执行禁用。
- `device::topology`
  - 负责枚举 present devnodes 与设备属性。
  - 负责读取 wake-capable 叶子快照。
  - 负责沿父链解析 `vidpid:*` / `sys:*` 族群来源。
- `device::identity`
  - 负责解析族群 ID。
  - 不再提供实例级启发式绑定给治理层使用。
- `config::registry`
  - 继续存储 `Whitelist`，但其值语义明确为族群 ID。
- `policy`
  - 负责族群级白名单判定和禁用计划生成。
- `service::worker`
  - 负责编排族群级扫描、判定、执行和日志。
- `ui`
  - 展示族群级状态，不再声称某一行对应某个实例。

## Data Model

- `WakeDeviceRaw`
  - `name` 表示族群代表名称。
  - `member_name` 表示当前 `powercfg` 观测到的执行名。
- `WakeDevice`
  - `stable_id` 语义改为治理族群 ID。
  - `display_name` 视为代表名称，而不是实例唯一名称。
- `DisableAction`
  - 应扩展为族群级动作，包含：
    - `stable_id`
    - `device_names`
    - `reason`
- `NewDeviceEvent`
  - `stable_id` 语义改为族群 ID。

## Family Identity Strategy

允许的受管族群 ID 仅有两类：

1. `vidpid:vid_xxxx&pid_xxxx`
2. `sys:ven_xxxx&dev_xxxx`

规则：

- `vidpid:*` 和 `sys:*` 都表示“设备族群”，不是“单个物理实例”。
- `name:*` 仅用于诊断，不进入自动治理、白名单、禁用或新增判定。
- 同一 `VID/PID` 的多个设备视为同一业务对象。
- 同一 `VEN/DEV` 的多个设备视为同一业务对象。
- 族群来源优先取 wake-capable 叶子自身的 `VID/PID`；若叶子无 `VID/PID`，则沿父链上溯，直到找到 `VID/PID` 或首个可接受的 `VEN/DEV`。

## Aggregation Strategy

- 扫描阶段先将 `powercfg` 当前名称与 wake-capable 叶子按“规范化名称 + 当前观测顺序”配对，再按族群 ID 归并。
- 同一族群若对应多个显示名：
  - UI 只显示一行。
  - 禁用执行对该组所有当前 wake-enabled 名称逐个执行。
  - 新设备检测按族群 ID 做集合差分。
- 不能稳定归并到 `vidpid:*` 或 `sys:*` 的记录：
  - 不进入自动治理。
  - 输出告警日志，附带显示名、叶子实例 ID、父链摘要和失败阶段。

## Policy Semantics

- 白名单表示“从自动禁用中豁免”。
- 白名单不表示“强制启用唤醒”。
- 若某个族群在加入白名单前已经被禁用，系统不会自动恢复其唤醒能力。
- 对未白名单族群：
  - 若能稳定解析为受管族群，则对该族群的 wake-enabled 成员执行禁用。
  - 若只能回退为 `name:*`，则只观察不治理。

## UI Semantics

- 管理页每一行代表一个族群。
- `whitelist=true` 表示整个族群被豁免。
- `wake_disabled=true` 表示该族群当前没有任何 wake-enabled 成员。
- `wake_disabled=false` 表示该族群当前至少存在一个 wake-enabled 成员。
- 若一个族群当前对应多个名称，UI 可显示代表名称，并允许后续补充成员数量或附加说明。

## Logging Strategy

- 输出族群级 heartbeat 和决策日志。
- 对每个需禁用族群记录：
  - `stable_id`
  - 观测到的名称列表
  - 实际执行禁用的目标名称列表
  - `reason`
- 对每个已解析族群记录：
  - `powercfg_name`
  - `leaf_instance_id`
  - `lineage`
  - `family_vidpid` / `family_sys`
- 对跳过治理的记录输出结构化日志：
  - `display_name`
  - `leaf_instance_id`
  - 父链摘要
  - 跳过原因

## Risks and Mitigations

- 风险：同一 `VID/PID` 的多台设备会被整组处理。  
  缓解：这是本次变更有意选择的业务语义，用于换取稳定性和可预测性。
- 风险：UI 从实例级改为族群级后，用户可能误解某一行对应单个设备。  
  缓解：在 UI 文案和 spec 中明确“该行代表一个受管族群”。
- 风险：部分只能回退到 `name:*` 的设备不再参与治理。  
  缓解：优先避免误禁，允许覆盖率下降，并通过日志暴露缺口。
