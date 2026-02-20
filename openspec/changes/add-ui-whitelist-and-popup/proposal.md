## Why

当前 Wakeguard 已具备服务侧扫描、禁用和监控能力，但缺少用户可见交互入口，用户无法直观管理白名单，也无法在新设备接入时及时决策。  
需要补齐基础 UI 与弹窗机制，使服务能力可操作、可确认、可回溯。

## What Changes

- 新增白名单管理页面，展示可管理设备列表及白名单状态。
- 支持在页面中通过勾选直接添加/移除白名单项。
- 在页面中展示设备当前唤醒禁用状态。
- 新增新设备弹窗机制：仅针对“未记录设备”首次插入弹窗确认是否加入白名单。
- 新增 `wakeguard ui` 命令，用于在服务运行期间主动调出主界面。
- 新增“已记录设备列表”持久化，避免同设备拔插后重复弹窗。

## Capabilities

### New Capabilities

- `ui-whitelist-management`: 白名单管理页面与勾选增删逻辑。
- `new-device-dialog-prompt`: 新设备首次接入弹窗与去重规则。
- `ui-launch-command`: 通过 `wakeguard ui` 调起主界面。

### Modified Capabilities

- 无

## Non-goals

- 不实现复杂权限系统与多用户配置隔离。
- 不实现跨平台 UI（仅支持 Windows）。
- 不引入数据库或外部服务依赖。

## Impact

- `src/main.rs`: 增加 `ui` 子命令入口。
- `src/service/worker.rs`: 新设备事件入队与已记录设备去重逻辑。
- `src/config/registry.rs`: 持久化已记录设备列表与 UI 共享状态。
- `src/ui/*`: 新增 UI 页面与弹窗交互模块。
- `tests/*`: 新增白名单页面数据流与弹窗去重测试。
