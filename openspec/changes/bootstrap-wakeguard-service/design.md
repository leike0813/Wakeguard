## Context

项目目标是一个 Windows 常驻服务，自动管理设备唤醒能力。当前阶段只做基础设施搭建，不进入业务逻辑实现。

## Goals / Non-Goals

**Goals**

- 建立可编译 Rust 工程骨架。
- 固化模块边界，避免后续实现阶段出现职责混乱。
- 起草 OpenSpec 文档，明确首里程碑范围和验收口径。

**Non-Goals**

- 不实现完整的设备识别链路。
- 不实现真实的 `powercfg` 调用流程。
- 不实现交互提示或 UI。

## Architecture Skeleton

- `main`:
  - 参数分发（`run/install/uninstall/once`）
  - 初始化日志
- `service`:
  - 服务生命周期入口
  - Worker 任务调度入口
- `device`:
  - 设备数据模型
  - 设备标识归一化接口
  - `powercfg` 访问适配层
- `config`:
  - 注册表配置读取接口
- `policy`:
  - 白名单策略计算（禁用计划）
- `logging`:
  - 统一日志初始化

## Key Decisions

### Decision 1: Service-first 架构

采用纯 Rust Windows Service 作为最终运行形态，命令行子命令仅用于安装/调试入口。

### Decision 2: Registry 作为白名单存储

白名单存储于 `HKLM\Software\Wakeguard`，减少额外配置文件分发成本，适配服务进程部署场景。

### Decision 3: 先骨架后业务

先落地模块和变更文档，再进入业务实现，可降低首轮返工概率。

## Risks and Mitigations

- 风险: 过早实现业务逻辑导致设计漂移。  
  缓解: 当前 change 明确禁止进入业务实现。
- 风险: 模块边界不清导致重复逻辑。  
  缓解: 通过独立模块职责定义和 tasks 清单约束。
