## Why

当前版本已具备基础扫描、禁用、监控流程，但在高频循环下的稳定性和可观测性仍偏弱，尚不能满足长期后台运行要求。  
需要针对运行监控、设备禁用、设备扫描做一轮增强，以提高鲁棒性与可维护性。

## What Changes

- 增强运行监控：引入周期级运行指标，输出结构化 heartbeat。
- 增强设备禁用：加入禁用重试冷却，降低重复执行与噪音日志。
- 增强设备扫描：对扫描结果做稳定去重与标准化，降低重复设备干扰。
- 补充对应单元/集成测试，覆盖上述增强逻辑。

## Capabilities

### New Capabilities

- `runtime-monitoring-hardening`: 监控扫描循环中的吞吐、失败和告警指标。
- `device-disable-hardening`: 对禁用动作增加重试节流与失败统计。
- `device-scan-hardening`: 对扫描结果做去重和可预测输出。

### Modified Capabilities

- `wake-enforcement-policy`: 增加禁用执行策略约束（冷却与重试）。
- `wake-device-observation`: 增加监控事件中的统计信息输出。

## Non-goals

- 不引入 UI 交互。
- 不实现 SCM 服务安装/卸载的深度能力。
- 不引入数据库或额外外部存储。

## Impact

- `src/service/worker.rs`: 运行监控、禁用执行节流、监控事件增强。
- `src/device/powercfg.rs`: 扫描结果去重与解析增强。
- `tests/*`: 增加监控/禁用/扫描相关覆盖。
