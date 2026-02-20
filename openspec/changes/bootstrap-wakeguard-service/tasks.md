## 1. Project Scaffolding

- [x] 1.1 新增 `Cargo.toml`，定义项目基础依赖
- [x] 1.2 新增 `src/main.rs`，建立统一入口与命令分发
- [x] 1.3 新增 `src/service`、`src/device`、`src/config`、`src/policy`、`src/logging` 骨架模块
- [x] 1.4 保持所有模块为占位实现，不进入业务逻辑

## 2. OpenSpec Baseline

- [x] 2.1 创建 change: `bootstrap-wakeguard-service`
- [x] 2.2 起草 proposal（why/what/non-goals/impact）
- [x] 2.3 起草 design（模块边界/决策/风险）
- [x] 2.4 起草 spec（首里程碑可验证需求）

## 3. Verification

- [x] 3.1 运行 `cargo check` 通过类型检查
- [x] 3.2 运行 `openspec validate --changes` 验证 change 结构
