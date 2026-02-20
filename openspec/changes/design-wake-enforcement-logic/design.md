## Context

当前项目仅有可编译骨架，尚未落地任何真实业务行为。  
首个业务里程碑要求系统能够在服务模式下自动处理可唤醒设备，并对新增设备产生可追踪事件。

## Goals / Non-Goals

**Goals**

- 明确业务流程和模块职责，形成可实现的设计契约。
- 定义“稳定设备标识”与“白名单判定”规则，避免设备身份漂移。
- 定义新增设备监控与失败处理策略，保证服务可持续运行。

**Non-Goals**

- 不实现 UI 交互（是否加入白名单的用户决策流程暂缓）。
- 不做设备驱动层深度适配，仅定义当前可执行方案。
- 不实现安装器、升级器等部署链路。

## Business Flow

1. `Discovery`：获取当前“可唤醒系统”的设备列表。
2. `Identity Resolution`：将设备映射为稳定标识 `stable_id`。
3. `Whitelist Load`：读取注册表白名单集合。
4. `Policy Evaluation`：对每个设备生成 `Allow` / `Disable` / `Observe` 决策。
5. `Action Execution`：对 `Disable` 设备执行禁用唤醒动作（幂等）。
6. `Observation Emit`：对新增设备和异常结果产生日志事件。

## Module Boundaries

- `device::powercfg`
  - 提供设备发现与禁用动作执行接口。
  - 不负责白名单判定。
- `device::identity`
  - 负责将原始设备信息转换为稳定标识。
- `config::registry`
  - 负责白名单配置访问与格式校验。
- `policy`
  - 负责决策计算，输出动作计划。
- `service::worker`
  - 负责编排扫描周期、执行顺序、错误隔离。

## Data Model (Design-Level)

- `WakeDeviceRaw`
  - 字段：`name`、`source`、`observed_at`
- `WakeDevice`
  - 字段：`display_name`、`stable_id`、`class`
- `PolicyDecision`
  - 枚举：`Allow`、`Disable`、`Observe`
- `DisableAction`
  - 字段：`stable_id`、`device_name`、`reason`

## Stable Identity Strategy

优先级（从高到低）：

1. 可复现硬件标识（VID/PID）
2. 系统总线硬件标识（VEN/DEV）
3. 标准化后的设备名（仅诊断回退，不纳入受管设备集合）

要求：

- 相同物理设备在不同端口重插时应保持同一 `stable_id`（VID/PID 或 VEN/DEV 路径）。
- 无法获得严格标识时不进入禁用策略输入集合，并记录跳过告警日志。
- 名称回退标识仅用于诊断与排障，不用于自动禁用唤醒决策。

## Whitelist Registry Contract

- 根键：`HKLM\Software\Wakeguard`
- 值名：`Whitelist`
- 类型：`REG_MULTI_SZ`
- 值内容：每行一个 `stable_id`

读取约束：

- 空值视为“无白名单”。
- 非法值类型视为配置错误，记录错误并继续服务循环（不崩溃）。

## Monitoring Strategy

- 第一阶段采用固定间隔扫描（默认 30 秒）。
- 与上一轮扫描结果做集合差分，识别新增 `stable_id`。
- 对新增设备记录结构化事件：
  - `event = new_wake_device_detected`
  - `stable_id`
  - `display_name`
  - `is_whitelisted`

## Failure Handling

- 单设备动作失败不终止本轮扫描。
- 单轮扫描失败不终止服务主循环，进入下一周期重试。
- 记录失败上下文（命令、设备、错误类型）便于排障。

## Risks and Mitigations

- 风险：仅依赖设备名可能导致误判。  
  缓解：设备名仅作为最后回退，并要求 `Observe` 标记。
- 风险：系统权限不足导致禁用失败。  
  缓解：失败可观测，不中断主循环。
- 风险：短时抖动设备导致频繁事件。  
  缓解：后续可引入去抖窗口（本 change 只定义接口）。
