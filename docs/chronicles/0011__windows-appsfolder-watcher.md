# Windows AppsFolder 变化触发应用索引刷新 Chronicle

日期：2026-09-21
计划：`docs/plans/0011__windows-appsfolder-watcher.md`
状态：完成

## 背景

Windows 应用索引已经以 `shell:AppsFolder` 为主数据源，但安装或卸载后只能依赖手动重建。目录文件事件不能可靠代表 Windows 的用户可启动应用集合。

## 决策

使用 Windows Shell 变化通知作为刷新信号。通知经过 2 秒防抖和单线程串行协调后重新枚举完整应用集合；只有 AppsFolder 枚举和数据库事务成功时才替换自动记录。

## 实施结果

- 新增独立 `WindowsAppWatcher`，在专用 COM/Windows 消息线程中注册 `shell:AppsFolder` 的 Shell 变化通知。
- Shell 通知进入容量为 1 的刷新队列，连续通知合并；扫描期间最多保留一个待处理刷新，避免安装器事件风暴堆积任务。
- 手动“重建应用索引”复用同一个刷新线程；监听启动失败时保留原有手动重建路径。
- `windows_apps::get_all_app` 改为返回 `Result`；COM 初始化或 AppsFolder 枚举失败时不再返回假空集合。
- `create_app_index_to_sql` 只在完整发现成功后执行事务替换，失败时保留旧自动应用和自定义应用。
- 应用退出时注销 Shell 通知、销毁消息窗口并回收监听与刷新线程。
- 更新 Windows 应用索引方案文档，明确通知、枚举和落库的职责边界。

## 验证

- `cargo fmt --check`：通过。
- `cargo metadata --no-deps`：通过，新增 Windows feature 名称有效。
- 目标文件 `git diff --check`：通过。
- `cargo check`：未完成。默认 target 被 `.cargo-build-lock` 的访问拒绝阻塞；两个独立 target 均在 `windows_x86_64_msvc` build script 处以 `0xc0000022` 失败，尚未编译到项目代码。
- Windows Shell 通知覆盖范围尚需实机验证：传统 Win32、当前用户安装、Store/MSIX 的安装、卸载和原地升级。

## Explain diff

核心边界是“通知不是事实”。Shell 通知只触发刷新，最终状态仍由重新枚举 AppsFolder 决定；枚举失败与成功的空集合被明确区分，因此临时 Shell/COM 故障不会清空数据库。

## Review

- 刷新线程串行执行，不会并发调用自动应用全量替换。
- 容量为 1 的队列会合并重复 Shell 通知，不会无界积压。
- 监听启动失败不会阻止 Lark 启动，也不会移除手动重建能力。
- 本阶段没有加入 Program Files、注册表、PackageCatalog、便携目录 watcher 或数据库 schema。
- 由于编译和实机验证受限，Windows API 签名和真实通知覆盖不能声明为运行验证成功。

## 当前步骤

Complete
