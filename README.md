# Wakeguard

Wakeguard 是一个基于 Rust 的 Windows Service 项目骨架，用于后续实现设备唤醒管理能力。

## Current Stage

当前仓库仅完成工程初始化与 OpenSpec change 起草，尚未进入业务逻辑实现。

## Commands

```powershell
cargo check
cargo run -- once
cargo run -- run
cargo run -- install
cargo run -- uninstall
```

## OpenSpec

当前 change:

- `openspec/changes/bootstrap-wakeguard-service/`
