## 1. Runtime Monitoring

- [x] 1.1 在 `service::worker` 增加累计运行指标结构
- [x] 1.2 在每轮扫描结束输出结构化 heartbeat 日志
- [x] 1.3 统计扫描失败与禁用失败次数

## 2. Device Disable

- [x] 2.1 增加同一设备禁用重试冷却策略
- [x] 2.2 为禁用冷却判定补单元测试
- [x] 2.3 保留禁用失败上下文字段日志

## 3. Device Scan

- [x] 3.1 在 `device::powercfg` 扫描结果中按 `stable_id` 去重
- [x] 3.2 补充扫描去重与空行解析测试

## 4. Verification

- [x] 4.1 运行 `cargo test`
- [x] 4.2 运行 `cargo check`
- [x] 4.3 运行 `openspec validate --changes`
