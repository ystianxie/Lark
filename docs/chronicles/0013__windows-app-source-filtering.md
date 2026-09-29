# Windows 应用索引来源分级过滤 Chronicle

日期：2026-09-22
计划：`docs/plans/0013__windows-app-source-filtering.md`
状态：完成
当前步骤：Complete

## 背景

当前扫描先从全部 AppsFolder 记录建立父目录抑制列表，再用同一套标题加目标文件名规则过滤 AppsFolder、开始菜单、桌面和便携目录。像 Reasonix 这类用户入口虽然展示名称正常，但目标为 `reasonix-launcher.exe`，会先被 `launcher` 规则过滤；其父目录仍会抑制便携扫描，导致同目录的 `reasonix.exe` 也无法作为兜底结果进入索引。

## 决策

按来源可信度拆分过滤语义：Windows Shell 和快捷方式提供的用户可见入口只根据展示名称识别辅助项，配置目录直接发现的裸 `.exe` 继续检查标题与文件名。父目录抑制只由通过过滤、去重并成功加入结果集的 AppsFolder 记录建立。

## 实施结果

- AppsFolder、开始菜单和桌面入口改为仅按用户可见标题识别辅助项，目标文件名中的 `launcher` 不再单独导致删除。
- 配置目录发现的裸 `.exe` 继续同时检查标题和文件名，维持严格辅助程序过滤。
- `registered_app_dirs` 改为在 AppsFolder 条目通过过滤与启动标识去重后才加入，失败条目不再封锁整个父目录。
- 增加 Reasonix 标题与 `reasonix-launcher.exe` 目标的来源分级测试，以及注册目录规范化测试。
- 更新 Windows 应用索引扫描方案，记录来源分级和成功收录后抑制的不变量。

## 验证

- `rustfmt --edition 2021 --config skip_children=true --check src-tauri/src/api/explorer.rs`：通过。
- `cargo metadata --manifest-path src-tauri/Cargo.toml --no-deps --format-version 1`：通过。
- `git diff --check`：通过；仅输出工作区既有无关文件的 LF/CRLF 警告。
- `cargo test --manifest-path src-tauri/Cargo.toml app_scan_tests --no-fail-fast`：未完成，默认 target 的 `.cargo-build-lock` 被拒绝访问。
- 使用独立 `CARGO_TARGET_DIR` 重试：依赖 build script 以 `0xc0000022` 失败。因此没有把单元测试或完整编译标记为通过。
- 已确认本机 `shell:AppsFolder` 存在显示名称为 `Reasonix` 的条目，开始菜单快捷方式目标为 `D:\App\Reasonix\reasonix-launcher.exe`，且无启动参数。

## Explain diff

这次变化把两个概念分开：用户明确可见的 Shell/快捷方式入口使用展示名称表达产品语义；文件系统直接发现的裸可执行文件仍需从文件名推测其是否为内部组件。目录抑制则成为成功收录的结果，而不是仅仅“扫描到过”的副作用。这样 Reasonix 的 launcher 入口可以作为主入口保留，同时仍由该入口抑制同目录的重复主程序和辅助进程。

## Review

- 与已确认方案一致，没有产品白名单或数据库迁移。
- 被过滤、空标识或启动标识重复的 AppsFolder 条目均不会产生目录抑制。
- 成功收录且有实际可执行路径的 AppsFolder 条目仍会抑制父目录及子目录。
- 开始菜单和桌面不会建立注册目录抑制；它们仍通过启动标识与 AppsFolder、便携结果去重。这保持了现有“AppsFolder 注册目录才是高可信应用边界”的范围。
- 未发现需要阻止交付的静态代码问题；Cargo 编译和运行时重建仍需在允许执行 build script 的环境验证。

## How the understanding evolved

- 原先统一的辅助项规则无法表达入口来源的可信度；本次把可信用户入口和裸可执行文件分开处理。
- 原先“扫描到注册项即可抑制目录”会让被拒绝入口隐藏有效主程序；本次改为只有成功收录项才获得抑制资格。
