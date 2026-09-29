# Windows 应用索引来源分级过滤

日期：2026-09-22
当前步骤：Complete

## 目标

避免可信 Windows 用户入口因启动目标文件名包含 `launcher` 等辅助词被误过滤，同时确保只有成功收录的 AppsFolder 应用才能抑制其所在目录的便携应用扫描。

## 已确认行为

- AppsFolder、开始菜单和桌面属于可信用户入口，辅助项判断以展示名称为准。
- 配置目录发现的裸 `.exe` 继续同时检查展示名称和文件名，保持严格辅助程序过滤。
- `registered_app_dirs` 只从成功加入自动索引的 AppsFolder 记录产生。
- 成功收录的 AppsFolder 入口仍抑制其父目录和子目录，避免 helper、updater 等重复进入索引。
- 被过滤或去重失败的 AppsFolder 条目不得抑制目录，便携扫描可继续发现其中的有效主程序。

## 修改范围

- `src-tauri/src/api/explorer.rs`
- `docs/Windows应用索引扫描方案.md`
- Windows 应用扫描相关 Rust 单元测试

## 验证

- 新增可信入口与便携入口分级过滤测试。
- 新增只有成功收录项才能产生目录抑制依据的结构性验证。
- `rustfmt --check`、相关 Rust 测试、`cargo check`、`git diff --check`。
- 若现有 Windows 构建权限问题阻止 Cargo 验证，明确记录限制。

## 非目标

- 不增加产品白名单。
- 不取消已注册应用父目录抑制。
- 不调整应用扫描设置或数据库结构。
- 不修改工作区内无关的插件和构建文件。
