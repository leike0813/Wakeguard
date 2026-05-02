## Why

当前 Wakeguard 会把 `powercfg` 输出中的 generic HID 显示名与 WMI/PnP 候选做实例级启发式绑定。  
这类绑定依赖显示名编号和枚举顺序，在系统重启或设备重枚举后可能漂移，导致某个设备被错误映射到别的 `vidpid/sys`，进一步误伤白名单设备。  
上一版保守修正虽然阻止了误映射，但也把大量 HID 设备整体排除在治理范围外。

## What Changes

- 将最小业务对象从“设备实例”调整为“设备族群”。
- 将 wake-capable HID 子节点先绑定到真实 PnP 叶子 devnode，再沿父链解析其上游设备族群。
- 将 `stable_id` 的业务语义明确为治理族群 ID，仅允许：
  - `vidpid:vid_xxxx&pid_xxxx`
  - `sys:ven_xxxx&dev_xxxx`
- 白名单按族群生效，自动禁用也按族群生效。
- 名称回退 `name:*` 仅用于诊断，不进入自动治理、白名单匹配或新设备判定。
- 调整 UI 和日志语义，使其展示和记录族群级状态，而不是实例级状态。

## Capabilities

### New Capabilities

- `wake-device-family-identity`: 定义受管设备族群 ID 的生成、过滤与归并规则。

### Modified Capabilities

- `wake-enforcement-policy`: 白名单与禁用动作切换为族群级治理。
- `wake-device-observation`: 新设备检测与日志事件切换为族群级语义。
- `ui-whitelist-management`: 管理页展示和状态解释切换为族群级语义。

## Non-goals

- 不引入物理设备实例级唯一标识方案。
- 不修改安装/卸载流程。
- 不实现 GUI 或新设备弹窗。
- 不自动恢复已被禁用设备的唤醒能力。

## Impact

- `src/device/*`: 后续将承载设备拓扑快照、父链族群解析、扫描归并与过滤逻辑。
- `src/policy/*`: 后续将承载族群级白名单判定与动作计划。
- `src/service/worker.rs`: 后续将承载族群级禁用执行与族群级日志。
- `src/ui/*`: 后续将展示族群级白名单与唤醒状态。
- `openspec/changes/refactor-wake-device-identity/*`: 本次产出重构设计与任务基线。
