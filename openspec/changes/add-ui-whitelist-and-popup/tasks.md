## 1. UI Entry and Window Scaffold

- [x] 1.1 在 `src/main.rs` 增加 `ui` 子命令并接入 UI 启动入口
- [x] 1.2 新增 `src/ui` 模块骨架与主窗口启动流程
- [x] 1.3 在主窗口中渲染设备列表基础布局（含 stable_id 与设备名列）

## 2. Whitelist Management Page

- [ ] 2.1 在注册表层新增白名单读写接口（支持增删单项）
- [x] 2.2 在 UI 中显示每个设备是否在白名单内
- [ ] 2.3 实现勾选增删白名单并持久化
- [x] 2.4 在 UI 中显示设备唤醒禁用状态

## 3. New Device Prompt and Dedup

- [ ] 3.1 在注册表层新增 `KnownDevices` 与 `PendingPrompts` 存取接口
- [ ] 3.2 在 `service::worker` 中对首次未记录设备写入弹窗事件
- [ ] 3.3 实现“已记录设备不重复弹窗”的去重规则
- [ ] 3.4 在 UI/Agent 侧消费弹窗事件并处理确认/拒绝结果

## 4. Integration and Verification

- [ ] 4.1 增加单元测试覆盖白名单勾选增删与记录设备去重逻辑
- [ ] 4.2 增加集成测试覆盖“新设备首次弹窗、重复插入不弹窗”
- [ ] 4.3 运行 `cargo check`
- [ ] 4.4 运行 `cargo test`
- [ ] 4.5 运行 `openspec validate --changes`
