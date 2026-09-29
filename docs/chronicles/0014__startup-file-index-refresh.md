# 启动时周期校准文件索引 Chronicle

日期：2026-09-29
计划：`docs/plans/0014__startup-file-index-refresh.md`
状态：完成

## 背景

文件 watcher 只能覆盖应用运行期间的变化。应用关闭期间发生的文件增删改不会被补获，但每次启动无条件全量重建又会让频繁重启产生无意义的重复扫描。

## 决策

保留现有 watcher 与 staging 原子替换机制，新增最近一次成功重建时间。首次索引保持立即补建；已有索引在启动后延迟 15 秒，再按 10 小时阈值或 `Stale` 状态决定是否后台校准。

## 实施结果

- `BaseConfig` 新增可选 Unix 秒时间戳 `last_file_index_rebuild_at`；旧配置缺字段时安全回退为 `None`，并在下一次启动安排一次校准。
- 手动重建、首次补建和启动周期校准统一经过 `rebuild_file_index_blocking`，继续复用现有运行互斥、staging 扫描和事务替换。
- `finish_rebuild_and_wait` 在发送 `FinishRebuild` 后等待索引服务处理到 `Flush`，确保重建期间积累的增量变更已处理，再检查是否进入 `Stale`。
- 只有全量扫描、增量补写和配置保存全部成功，才记录最近成功时间；失败保持旧时间，后续启动仍可重试。
- 已初始化索引在启动 15 秒后重新读取最新配置，避免启动期间手动重建成功后又使用旧快照重复扫描。
- 时间戳缺失、达到 10 小时边界或异常位于未来时均视为需要校准；未来时间按系统时钟异常处理，避免永久抑制重建。

## 验证

- `rustfmt --check`（本次直接修改的 config、config mod、file watcher）：通过。
- `git diff --check`（本次目标文件）：通过，仅有工作区既有 LF/CRLF 提示。
- `cargo metadata --no-deps --format-version 1`：通过。
- 新增 10 小时边界、缺失时间与未来时间的单元测试。
- `cargo test ... --no-run`：未完成。默认 target 被 `.cargo-build-lock` 拒绝访问；独立 target 又在依赖 build script 处以 `0xc0000022` 失败，未编译到项目代码。
- 未进行真实启动等待 15 秒、磁盘全量扫描、搜索并发和应用关闭重启的运行验证。

## Explain diff

本次时间戳记录的不是“开始扫描时间”，而是“正式索引已原子替换，重建期间的增量事件也已处理完成”的成功边界。这样应用在扫描失败或中途退出后不会错误等待 10 小时。扫描阶段继续使用旧的正式索引，最终替换仍由现有 SQLite 事务完成。

## Review

- 没有引入第二套索引器、USN、锁屏检测或前端调度。
- 自动、首次和手动入口共享相同成功时间语义。
- 启动调度重新读取配置，避免与启动后手动重建形成重复工作。
- 空扫描路径继续直接跳过，不改变用户明确禁用文件索引的语义。
- 编译和实际运行行为仍受当前 Windows 执行环境限制，静态检查不能替代运行证明。

## How the understanding evolved

- A successful rebuild timestamp must be written after both the atomic snapshot replacement and queued incremental updates, not when scanning starts.
- The delayed startup decision must reread persisted configuration so a manual rebuild during the delay cannot trigger an unnecessary second rebuild.

## 当前步骤

Review complete
