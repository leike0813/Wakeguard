## 1. Policy Design to Implementation Plan

- [x] 1.1 定义 `WakeDeviceRaw`、`WakeDevice`、`PolicyDecision`、`DisableAction` 数据结构
- [x] 1.2 在 `device::identity` 设计并实现稳定标识优先级策略
- [x] 1.3 在 `policy` 实现白名单匹配与动作计划生成

## 2. Observation Design to Implementation Plan

- [x] 2.1 在 `service::worker` 增加周期扫描和结果差分
- [x] 2.2 设计并实现新增设备结构化事件输出
- [x] 2.3 为异常路径增加不中断循环的错误处理

## 3. Registry Contract Implementation Plan

- [x] 3.1 在 `config::registry` 实现 `HKLM\Software\Wakeguard\Whitelist` 读取
- [x] 3.2 增加注册表值类型校验与错误降级策略
- [x] 3.3 为空白名单、非法类型、缺失键编写测试用例

## 4. Verification

- [x] 4.1 通过单元测试覆盖策略判定场景（白名单/非白名单/低置信度）
- [x] 4.2 通过集成测试覆盖扫描差分与事件输出场景
- [x] 4.3 运行 `cargo check` 与 `openspec validate --changes`
