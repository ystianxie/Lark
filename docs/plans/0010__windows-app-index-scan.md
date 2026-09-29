# Windows 应用索引扫描优化

日期：2026-09-20
当前步骤：Implement

## 目标

以 Windows `shell:AppsFolder` 注册应用为主数据源，补充用户配置的便携应用目录，覆盖系统内置与 Microsoft Store 应用，同时减少辅助程序、卸载器和重复结果。

## 已确认行为

- Windows 注册应用优先于便携目录结果。
- 自定义应用继续由数据库保留，并优先于自动扫描结果。
- 不再单独扫描开始菜单和桌面。
- 便携目录仅收录可执行文件，并继续应用目录排除与辅助程序过滤。
- 同名但启动目标不同的应用保留；按规范化启动目标去重。
- `shell:AppsFolder\\...` 通过 Windows Shell 启动。
- macOS 扫描行为不变。

## 修改范围

- `src-tauri/src/api/file.rs`
- `src-tauri/src/api/explorer.rs`
- `src-tauri/src/api/shell.rs`
- 应用扫描、去重和数据库保护相关 Rust 测试

## 验证

- Rust 格式检查和相关单元测试。
- `cargo check` 与前端构建。
- Windows 手工验证计算器、记事本、至少一个 Store 应用、传统 Win32 和便携应用。
- 环境若阻止构建脚本或子进程执行，明确记录限制，不将静态检查视为运行成功。

## 非目标

- 不增加新的扫描模式设置。
- 不扫描 Program Files、Windows 目录或全盘。
- 不重构 macOS 应用发现。
- 不覆盖工作区内其他未提交修改。
