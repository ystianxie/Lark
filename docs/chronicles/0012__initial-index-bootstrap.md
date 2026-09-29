# 首次启动自动建立应用与文件索引 Chronicle

日期：2026-09-21
计划：`docs/plans/0012__initial-index-bootstrap.md`
状态：完成

## 背景

启动流程已经能够创建索引数据库、为首次运行生成默认文件扫描路径并启动增量 watcher，但不会主动扫描启动前已存在的应用和文件。仅启动 watcher 无法补齐历史数据。

## 决策

使用配置布尔值分别记录应用索引和文件索引是否成功完成过全量扫描。启动时先兼容迁移旧配置，再在后台串行补建未初始化的索引。空文件路径代表用户明确禁用文件扫描，不能恢复默认值或反复尝试扫描。

## 实施结果

- `BaseConfig` 新增 `app_index_initialized` 与 `file_index_initialized`，新配置默认均为 `false`，旧配置缺字段时由 serde 安全回退。
- 启动时先检查旧配置是否真实包含字段：对应表有数据则补写 `true`，表为空则补写 `false`；数据库状态查询失败时不把失败误判为空表，本次启动跳过自动扫描并在下次启动重试迁移。
- `local_file_search_paths = Some([])` 时将文件初始化状态补写为 `true`，不恢复默认目录、不启动全量扫描。
- Tauri `setup` 后台任务按应用、文件顺序执行首次扫描，不阻塞窗口和快捷键初始化；应用失败不会阻止文件扫描。
- 应用和文件扫描入口在成功提交数据库后自行持久化初始化标志，因此首次扫描、手动重建和 watcher 触发的完整应用重建共享同一成功边界。
- 文件全量扫描改为返回 `Result`；开始代次、批量写入、提交代次或配置路径全部不可用时不会误记成功。
- 应用扫描增加进程内互斥锁；文件首次扫描与手动重建共享运行标志，避免启动阶段重复全量扫描。
- 新增旧配置缺少初始化字段时回退为 `false` 的 serde 测试。

## 验证

- `rustfmt --edition 2021 --config skip_children=true`：通过。
- `cargo fmt --check`：通过。
- `cargo metadata --no-deps --format-version 1`：通过。
- 目标文件 `git diff --check`：通过，仅有工作区既有的 LF/CRLF 提示。
- `cargo test config_without_index_flags_defaults_to_not_initialized --no-run`：未完成；独立 target 在 `windows_x86_64_msvc` build script 处以 `0xc0000022` 失败，尚未编译到项目代码。
- 未进行首次安装、旧配置升级或真实磁盘全量扫描的运行验证。

## Explain diff

两个布尔值记录的是“成功完成过全量扫描”，而不是“表当前非空”。表为空可能是合法扫描结果，表有数据则可用于兼容旧版本。首次扫描和手动扫描都在同一个扫描函数成功提交后写标志，避免不同入口对“完成”的定义不一致。

旧配置迁移必须先判断字段是否缺失，不能只读取 `#[serde(default)]` 后的布尔值；否则无法区分“旧版本没有这个字段”和“新版本明确保存为 false”。因此启动流程先读取原始 JSON 的字段存在性，再结合数据库行数迁移。

## Review

- 配置缺失、明确 `false`、明确 `true` 三种状态在迁移前后可以区分。
- `Some([])` 保留用户明确关闭文件扫描的含义。
- 数据库查询失败、扫描失败和配置保存失败均不会被声明为初始化成功。
- 应用和文件扫描按顺序执行，并对可达的手动/监听并发入口进行了串行化。
- 没有增加数据库元数据表、前端设置项或新的扫描实现。
- Rust 编译与运行行为仍受 `0xc0000022` 限制，当前结论属于格式、元数据和静态结构验证，不是运行成功证明。

## How the understanding evolved

- The initialization flag represents a successfully committed full scan, not whether the index currently contains rows.
- Legacy configurations require raw field-presence detection so an absent field is not confused with an explicit false value.

## 当前步骤

Complete
