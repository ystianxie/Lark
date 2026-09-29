# Windows AppsFolder 变化触发应用索引刷新

日期：2026-09-21
当前步骤：Complete

## 目标

监听 Windows `shell:AppsFolder` 的 Shell 变化通知，在正常安装、卸载或更新应用后，防抖并串行重建自动应用索引。

## 已确认行为

- Shell 通知只表示“应用目录可能变化”，不直接推导单条数据库增删。
- 连续通知合并后复用现有完整应用扫描与去重逻辑。
- 同一时刻只运行一个应用索引扫描；扫描期间的新通知会在完成后再触发一次。
- AppsFolder 枚举失败时保留旧自动索引，禁止把失败误判为空目录。
- 自定义应用继续由 `is_custom = 1` 保护。

## 修改范围

- `src-tauri/Cargo.toml`
- `src-tauri/src/api/windows_apps.rs`
- `src-tauri/src/api/windows_app_watcher.rs`
- `src-tauri/src/api/explorer.rs`
- `src-tauri/src/api/mod.rs`
- `src-tauri/src/main.rs`
- `docs/Windows应用索引扫描方案.md`
- 本计划与对应 chronicle

## 验证

- 检查协调器防抖、重复刷新合并和扫描串行化。
- Rust 格式检查和目标代码静态检查。
- `cargo check`。
- Windows 实机仍需验证传统 Win32 与 Store/MSIX 安装、卸载事件覆盖。

## 非目标

- 不监听 Program Files、注册表或便携应用目录。
- 不接入 PackageCatalog。
- 不增加应用索引差异写入或数据库 schema。
- 不修改普通文件 watcher。


