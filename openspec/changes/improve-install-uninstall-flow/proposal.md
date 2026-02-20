## Why

Wakeguard 目前安装与卸载仍是占位流程，无法形成可交付的服务生命周期闭环。  
需要补齐安装初始化、首次引导、注册表清理与卸载流程，确保用户可从命令完成部署和移除。

## What Changes

- 实现 `wakeguard install` 命令安装 `Wakeguard` 服务并启动。
- `wakeguard install` 默认使用当前执行的 `wakeguard.exe` 作为安装源，同时保留显式路径覆盖参数。
- 安装时将二进制复制到 `C:\Program Files\Wakeguard\bin\wakeguard.exe`。
- 安装时写入系统级 PATH（`C:\Program Files\Wakeguard\bin`），且不写入 Windows 系统目录。
- 安装完成后自动拉起 `wakeguard ui`，引导用户管理白名单。
- 首次安装默认白名单为空，并提供“按设备选择”与“一键加入全部设备”两种入口。
- 首次安装检测注册表键是否匹配 Wakeguard 规则，不匹配则清理非法项。
- 实现 `wakeguard uninstall` 命令，卸载 `Wakeguard` 服务（保留注册表数据）。
- 卸载时自动移除全局 PATH 项与复制的二进制文件（运行中场景采用延迟清理策略）。
- 服务处于安装状态时，用户可直接在 Shell 中执行 `wakeguard ui` 与 `wakeguard uninstall`，无需附加路径。
- 卸载完成后，Shell 中不应继续无路径调用 `wakeguard`。

## Capabilities

### New Capabilities

- `service-install-flow`: 服务安装、启动与安装后 UI 引导。
- `service-uninstall-flow`: 服务停止与卸载（保留注册表）。
- `first-install-registry-sanitize`: 首次安装注册表合法性校验与非法项清理。
- `first-install-onboarding`: 首次安装空白名单引导与设备选择入口。
- `global-cli-distribution`: 安装后全局命令可达性与卸载后命令回收。

### Modified Capabilities

- 无

## Non-goals

- 不卸载或清理注册表数据。
- 不实现多服务协同卸载，仅处理 `Wakeguard` 主服务。
- 不引入安装器 GUI 程序（MSI/EXE wizard）。

## Impact

- `src/main.rs`: 安装卸载命令行为强化。
- `src/service/*`: 安装/卸载真实编排逻辑。
- `src/config/registry.rs`: 首次安装注册表白名单规则校验与清理。
- `src/install_path/*`（或等价模块）: 全局路径复制、PATH 注入/移除、延迟删除执行。
- `src/ui/*`: 首次安装引导入口（空白名单提示 + 一键加入全部）。
- `tests/*`: 安装卸载、PATH 分发与注册表清理策略测试。
