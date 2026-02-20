## 1. Service Install Flow

- [x] 1.1 为 `wakeguard install` 增加默认安装源（当前执行文件）并保留二进制路径覆盖参数
- [x] 1.2 实现安装源二进制复制到 `C:\Program Files\Wakeguard\bin\wakeguard.exe`
- [x] 1.3 实现系统 PATH 注入 `C:\Program Files\Wakeguard\bin`
- [x] 1.4 在 `src/service` 中实现服务创建与启动编排
- [x] 1.5 为安装流程增加“服务已存在”幂等处理或明确错误分支
- [x] 1.6 在安装成功后调用 `wakeguard ui` 拉起主界面

## 2. First-Install Registry Sanitize

- [x] 2.1 在 `src/config/registry.rs` 定义注册表允许字段白名单
- [x] 2.2 实现首次安装注册表合法性检查与非法项清理
- [x] 2.3 实现首次安装默认空白名单初始化

## 3. First-Install Onboarding

- [x] 3.1 在 UI 中增加首次安装引导提示（空白名单说明）
- [x] 3.2 提供按设备选择加入白名单的入口
- [x] 3.3 提供“一键加入所有支持设备”入口

## 4. Service Uninstall Flow

- [x] 4.1 在 `src/service` 中实现 `wakeguard uninstall` 的停止与删除服务逻辑
- [x] 4.2 卸载时移除系统 PATH 中的 `C:\Program Files\Wakeguard\bin`
- [x] 4.3 卸载时删除全局复制的 `wakeguard.exe`（运行中采用延迟删除）
- [x] 4.4 确保卸载流程不删除 Wakeguard 注册表数据
- [x] 4.5 验证卸载后 Shell 无法无路径执行 `wakeguard`

## 5. Verification

- [x] 5.1 增加测试覆盖路径参数校验、二进制复制、PATH 注入/移除
- [x] 5.2 增加测试覆盖安装编排与卸载编排关键分支
- [x] 5.3 增加测试覆盖首次安装注册表清理规则
- [x] 5.4 增加测试覆盖卸载延迟删除分支
- [x] 5.5 运行 `cargo check`
- [x] 5.6 运行 `cargo test`
- [x] 5.7 运行 `openspec validate --changes`
