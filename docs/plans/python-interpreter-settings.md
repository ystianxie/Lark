# Python 解释器与虚拟环境设置

日期：2026-09-18
当前步骤：Complete（实施已完成；未运行的验证见文末「实施记录」）
评审：2026-09-18 自查一轮，核对了 `handleSettingReset` / `handleSettingSave` 的 payload / `run_python_script` 返回契约，修订结果已并入下文（见「评审修订记录」）。

> 说明：本文行号引用基于 2026-09-18 的工作区状态（当时 `src-tauri/src/api/*`、`src/panels/settingComponent.jsx`、`src/App.jsx` 等含未提交修改，与本计划无直接冲突）。实施时若行号漂移，以函数名与标识符为准。

## 目标

在"应用设置"页提供**全局 Python 解释器配置**，可指向某个虚拟环境的解释器；插件的 Python 执行与外部 Python 索引脚本统一使用该配置；未配置时退回系统默认（Windows `python.exe`、其他平台 `python3`），行为与今天完全一致。

## 现状核实（本次调研结论，含证据）

1. **插件执行路径唯一**：`src-tauri/src/api/shell.rs:70` 的 `run_python_plugin_blocking`。解释器 = 调用方传入的 `interpreter`，为空时回落 `python.exe` / `python3`（`shell.rs:79-87`）。唯一调用方是 `src/pluginRuntime.js:31`，其 `interpreter` 唯一来源是 `manifest.runtime?.pythonPath`。
2. **`manifest.runtime.pythonPath` 是只读不写的死字段**：
   - 全仓 `pythonPath` 仅 2 处命中——`src/pluginRuntime.js:34`（读）与 `docs/插件开发规范.md:99`（文档），**无任何写入端**；
   - 所有 `*.json`（含向导真实产物 `src-tauri/target/debug/config/lark/data/plugins/*/manifest.json`）**无 `"runtime"` 键**；
   - `src-tauri/src/config/plugins.rs` 中 `runtime` 零命中；manifest 以 `serde_json::Value` 松散解析（`plugins.rs:599`），生成与校验逻辑都不认该键；
   - git 溯源：`pluginRuntime.js:34` 由 `bbeef7c` 引入、文档那句由 `ea6790b` 引入，都是"预留钩子"。
   - **结论：现状实际使用的就是 PATH 上的系统 `python.exe`（本机 Python 3.12.6），没有虚拟环境、没有全局配置。**
3. **第二条执行路径独立**：`shell.rs:29` 的 `run_python_script` 硬编码 `Command::new("python")`（`shell.rs:33`），无 `PYTHONIOENCODING`、无 `creation_flags`（Windows 下可能闪控制台窗口），且失败时 `.expect()` 直接 panic（`shell.rs:37`）。调用方为 `src/App.jsx:569`（外部索引脚本，`.py` 结尾）与 `src/baseComponent.jsx:24`。
4. **无效分发器**：`src-tauri/src/api/python_runner.py` 全仓无引用。
5. **死代码**：`src/baseComponent.jsx:3-10` 的 `run_python_plugin` wrapper 从未被调用——`src/App.jsx:496/507` 走 `"action_" + component.action` 动态前缀，不会命中该名字；`pluginRuntime.js` 自己直接 `invoke`。
6. **能力缺口**：项目无 `tauri-plugin-dialog`、无 `plugin-fs`；capabilities **内嵌在** `src-tauri/tauri.conf.json` 的 `app.security.capabilities` 数组（不是独立的 `src-tauri/capabilities/` 目录）；包管理器是 pnpm（`tauri.conf.json` 的 `beforeDevCommand: "pnpm dev"`，`node_modules/.pnpm` 存在）。

## 已确认的设计决策

1. **交互方式**：引入 `tauri-plugin-dialog` 提供系统文件选择器。
2. **作用范围**：`run_python_plugin` 与 `run_python_script` 统一走同一配置。
3. **旧字段处理**：删除 `pluginRuntime.js:34-35` 的 `manifest.runtime` 死字段，全局设置为唯一来源，不留"前端能悄悄覆盖全局设置"的口子。
4. **默认值不变**：未配置时保持 `python.exe` / `python3`；旧 `config.json` 靠 `#[serde(default)]` 兼容，无需迁移。
5. **存储位置**：放 `BaseConfig`，随既有「保存设置」按钮落盘，不新增配置文件。
6. **不激活 venv**：以绝对路径调用解释器即可，`sys.prefix` 会指向该环境，`site-packages` 自动生效。不引入"激活环境"这类概念。

## 术语

- **配置值**：用户在设置页填的解释器路径，存 `config.json` 的 `base.python_interpreter`。
- **解析结果**：`配置值（非空）` → 否则 `平台默认命令`。这是实际传给 `Command::new` 的东西。
- **探测**：执行 `<解析结果> --version` 并回报版本与来源判定，用于在 UI 里给出即时反馈，不阻断保存。

## 设计

### 1. 配置字段

`BaseConfig` 增加：

```rust
#[serde(default)]
pub python_interpreter: Option<String>,
```

`None` 或空白串一律视为"未配置"。`BaseConfig` 无 `rename_all`，落盘键为 `python_interpreter`；对外 JSON（`app_settings()` / `save_setting`）用 camelCase `pythonInterpreter`，与既有字段风格一致。

### 2. 解析优先级（单一函数，两处共用）

```rust
fn resolve_python_interpreter(configured: Option<&str>) -> String
```

- 非空配置 → 原样使用（**不做路径规范化、不查存在性**：确认真实性交给探测，避免解析函数里藏 IO）。
- 否则 `cfg!(target_os = "windows")` → `"python.exe"`，其他 → `"python3"`。

`run_python_plugin` 去掉 `interpreter: Option<String>` 参数（前端同步删除传参）；`timeout_ms` 保留参数但由 Rust 侧常量 30000 兜底，不再从 manifest 读（`pluginRuntime.js:35` 的 `timeoutMs` 传参一并删除）。

**有意接受的成本**：Rust 侧每次执行都 `Config::read_local_config()` 读一次 `config.json`。这与既有 `get_plugin_settings` 的模式一致（`pluginRuntime.js:38-40` 的注释已论证过"必须每次取值、不能快照"），换来"改配置立即生效、无需重启"；插件实时执行（200ms 防抖）场景下这点磁盘 IO 可接受。

### 3. 探测/校验命令

新增 `probe_python_interpreter(path: Option<String>) -> Result<Value, String>`，返回：

```json
{"ok": true, "configured": "D:\\envs\\x\\Scripts\\python.exe", "resolved": "D:\\envs\\x\\Scripts\\python.exe",
 "version": "Python 3.12.6", "kind": "venv", "error": null}
```

- 执行 `<resolved> --version`，**超时 5 秒**（与插件执行的 30 秒分开）。Python 3 把版本写 stdout、2.x 写 stderr，两者都取。`--version` 输出量极小，`Command::output()` 包一层超时即可，**不要**为了对称去复制 `run_python_plugin_blocking` 那套多线程读管道。
- `kind` 判定：从解释器所在目录向上逐级查找 `pyvenv.cfg`，找到 → `venv`；否则 `system`；`resolved` 路径含 `WindowsApps` **且探测失败/超时** → `store-alias`（提示"这是 Microsoft Store 的应用执行别名，请选择具体解释器的完整路径"）。
  - **只凭路径含 `WindowsApps` 不下结论**：用户可能真的装了 Store 版 Python，必须由探测结果佐证。
- 未配置时也可调用（`path: None`），返回平台默认解释器的探测结果，用于 UI 显示"当前默认"。

### 4. 前端 UI

`src/panels/settingComponent.jsx` 的 `app` tab（`1278` 起）在「快捷键」「剪贴板历史」之后增加一个 `appSettingCard`「Python 环境」：

- 一个 `Input`（可编辑、可粘贴路径）+ 一个「选择…」按钮（`open({multiple:false, directory:false, filters:[{name:"Python", extensions:["exe"]}]})`，非 Windows 平台不加 filters）。
- 一行探测结果：`venv（3.12.6）` / `系统 Python 3.12.6` / `未配置，使用系统默认：python.exe` / 错误文本（红色，复用既有 `settingError` 类）。
- 选择后立即探测一次；输入框失焦也探测一次。
- 保存走既有 `handleSettingSave`：其 `all_setting`（`settingComponent.jsx:818-831`）是**显式列出的全量字段对象**，加 `pythonInterpreter` 后该 key 每次都会出现在 payload 里。
- **清空必须显式传 `null`**：`undefined` 会被 `JSON.stringify` 丢弃 → Rust 侧 `get("pythonInterpreter")` 返回 `None` → 按设计"不更新"，用户就再也清不掉已配置的路径。前端必须写 `pythonInterpreter: pythonInterpreter ?? null`。
- **不纳入 `handleSettingReset`**（已核实实现，`settingComponent.jsx:797-817`）：该按钮只重置剪贴板四项**且不落盘**（需再点"保存设置"才生效），连快捷键都不重置。把 Python 解释器塞进去只会扩大它的语义、制造新的不一致；让它保持"只服务剪贴板卡片"即可。
- **已知既有缺陷（本期不修，但必须记录）**：`get_app_settings` 失败时被 `.catch` 吞掉（`settingComponent.jsx:707`），state 停在默认值，此时点"保存设置"会把默认值写回、覆盖用户已有配置。新增字段沿用同一模式，不引入更差的行为，但不要声称这条路径是安全的。

### 5. dialog 插件接入

- `pnpm add @tauri-apps/plugin-dialog`
- `src-tauri/Cargo.toml` 增加 `tauri-plugin-dialog = "2"`（与 `tauri-plugin-global-shortcut = "2.3.2"` 并列，`Cargo.toml:96` 附近），两者都取 2.x 主版本，与 `tauri 2.11.5` / `@tauri-apps/api 2.11.1` 同代
- `src-tauri/src/main.rs` 增加 `.plugin(tauri_plugin_dialog::init())`（与 `main.rs:126` 的 global-shortcut、`main.rs:552` 的 win_file_drop 并列）
- `src-tauri/tauri.conf.json` 的 `app.security.capabilities[0].permissions` 增加 **`"dialog:allow-open"`**（最小权限，不用 `dialog:default`）

## 实施步骤

| 阶段 | 内容 | 主要文件 |
| --- | --- | --- |
| 1 | 配置字段：struct 字段 + **macOS 与 Windows 两个 `Default` 分支**（`config.rs:59`/`110`，漏一个分支就编译失败）+ `ConfigUpdate` 变体 + `update_local_config` 分支 + `save_setting_data` 解析 + `app_settings()` 返回 | `src-tauri/src/config/config.rs` |
| 2 | `save_setting` 的 null 语义：用 `setting_info.get("pythonInterpreter")` 判断 **key 是否存在**（`Value::as_str` 对 `null` 返回 `None`，不区分"未传"与"清空"）；key 存在 → `as_str()` 转 `Option<String>`（null 即清空），key 不存在 → 不改动。配套前端必须显式传 `null` | `src-tauri/src/config/config.rs` |
| 3 | `resolve_python_interpreter` + `run_python_plugin` 去掉 `interpreter` 参数、改从配置取值 + `run_python_script` 改用同一解析结果，顺带补 `PYTHONIOENCODING` / `creation_flags`。**必须保持 `run_python_script` 的返回契约**：它返回 `HashMap<&str, String>`（`shell.rs:30`），调用方 `src/App.jsx:569` 依赖 `result.success === "true"`——`.expect()` 要替换成 `success: "false"` + `data: <错误文本>`，**不要**改成 `Result` 返回，否则调用方静默拿不到结果（`src/baseComponent.jsx:24` 同受影响） | `src-tauri/src/api/shell.rs` |
| 4 | `probe_python_interpreter` 命令 + 在 `generate_handler!`（`main.rs:580`）注册 | `shell.rs`、`main.rs` |
| 5 | dialog 插件接入（见上节 5 条） | `Cargo.toml`、`main.rs`、`tauri.conf.json`、`package.json` |
| 6 | 前端：设置页卡片、`pluginRuntime.js:34-35` 删死字段、`baseComponent.jsx:3-10` 清掉未调用的 wrapper、`get_app_settings` 回填 | `settingComponent.jsx`、`pluginRuntime.js`、`baseComponent.jsx` |
| 7 | 文档与死文件：`docs/插件开发规范.md:99`（"留待后续应用设置功能"改为已实现）与 `:109`（"宿主使用用户配置的 Python"）改写为新语义；删除无引用的 `src-tauri/src/api/python_runner.py`（删除安全性已核验：全仓零引用，且 `tauri.conf.json` 的 `bundle` 只配 `icon` / `externalBin`，未把该目录作为 resources 打包） | `docs/`、`src-tauri/src/api/` |

## 验证

1. `cargo test`（`shell.rs:168` 起的 `python_plugin_tests` 依赖 `run_python_plugin_blocking` 的现有签名，阶段 3 需同步改测试；新增 `resolve_python_interpreter` 与 `save_setting` null 语义的单测）。
2. `pnpm build`（前端能编译）。
3. 手动：未配置 → 插件正常执行；配置 venv → 插件 `import` 该环境独有的第三方包成功（这是"虚拟环境能生效"的核心证据）；填错路径 → 得到明确错误文本而非 panic。
4. **回归重点——`run_python_script` 返回契约**：`.py` 结尾的外部索引脚本（`App.jsx:569`）仍能拿到 `success === "true"` 并正常渲染结果列表；解释器不存在时得到 `success === "false"`，不 panic、不闪控制台窗口。
5. **清空路径端到端**：配置路径 → 保存 → 重新打开设置页确认回填 → 清空输入框 → 保存 → 重启确认回到"未配置"（验证 `null` 语义真的可用）。
6. **已知环境限制**：本工作区此前出现过"沙箱拒绝子进程执行导致 Rust 测试无法运行"的情况（见 `docs/chronicles/0008`），若复现需在交付说明中明确标为未验证，不自称已通过。

## 风险与未决

1. **索引脚本落到无依赖的 venv**：`run_python_script` 统一后，`App.jsx:569` 的外部索引脚本会跑在用户配置的环境里，可能缺依赖。这是"统一"的预期语义，但需在 UI 里一句话说明（"该解释器同时用于插件与外部 Python 索引脚本"）。
2. **依赖安装需要网络**：`pnpm add` 与 cargo 拉取 dialog 插件都需要联网；若不可用，回退方案是纯文本输入框（本次不做，但实现时若被阻塞应回头确认）。
3. **Windows Store 别名**：探测逻辑依赖"失败才判定"，纯路径判断会误伤真装了 Store 版 Python 的用户。
4. **`python3` 在 Windows 上不一定存在**：默认值保持现状（`python.exe`），不额外探测，避免行为漂移。
5. **配置改动对已缓存 runtime 的影响**：`pluginRuntime.js` 的 `runtimes` Map 会缓存插件运行时，但解释器现在每次执行都在 Rust 侧重新读配置，因此改配置后**无需重启**（这正是把解析放 Rust 侧的好处，与 `pluginRuntime.js:38-40` 注释的教训一致）。
6. **权限清单位置**：`dialog:allow-open` 要加进 `tauri.conf.json` 的 `app.security.capabilities[0].permissions`，项目**没有**独立的 `src-tauri/capabilities/` 目录（已核实），别去新建一个。
7. **删参数后残留的传参**：`src/baseComponent.jsx:7` 当前仍在传 `interpreter`。Tauri 命令按声明逐个提取参数、多余键会被忽略，所以不会报错；阶段 6 会清掉这个未使用的 wrapper，不要留"传了但没人看"的参数。

## 不在本期范围

- 自动创建虚拟环境、安装依赖、venv 管理 UI（`docs/插件开发规范.md:99` 的"留待后续"仍适用于这些）。
- 依赖包管理器自动发现（conda / uv / pyenv 的自带解释器需要用户手动选路径，但只要给的是可执行文件路径就能工作）。
- 修复「设置页以默认值覆盖已保存配置」这一既有缺陷（见设计 4 最后一条），它影响所有既有字段，应单独一次改动处理。

## 评审修订记录

2026-09-18 自查一轮，相对初稿修正如下：

1. **`handleSettingReset` 的描述是错的**：初稿写"须确认它会把 `pythonInterpreter` 一并清空"，实际实现（`settingComponent.jsx:797-817`）只重置剪贴板四项、不重置快捷键、不落盘。已改为"明确不纳入重置"。
2. **补上"清空必须传 `null`"**：初稿只写了 Rust 侧的 key 存在性判断，没写前端 `undefined` 被 `JSON.stringify` 丢弃会让"清空"失效。
3. **补上 `run_python_script` 返回契约约束**：初稿只说"修掉 `.expect()` panic"，没说该函数返回 `HashMap` 而非 `Result`、调用方依赖 `success === "true"`——按初稿实施极易改成 `Result` 而静默破坏索引脚本。
4. **记录既有缺陷边界**：`get_app_settings` 失败被吞导致"保存即覆盖"，明确列为本期不修但必须记录，避免误以为新流程安全。
5. **降级探测实现建议**：初稿要求"并行读 stdout/stderr"，对 `--version` 是过度设计，改为 `Command::output()` + 超时。
6. **补充可核验的事实依据**：`python_runner.py` 删除的安全性依据（bundle 未含 resources）、capabilities 内嵌位置、dialog 依赖版本同代关系。

## 实施记录

2026-09-18 按阶段 1→7 实施完成：

- `src-tauri/src/config/config.rs`：字段 + 两个平台 `Default` 分支 + `ConfigUpdate` 变体 + `update_local_config` 分支 + `save_setting_data` 的「key 存在即更新」 + `app_settings()` 返回。
- `src-tauri/src/api/shell.rs`：`resolve_python_interpreter` 等解析函数；`run_python_plugin` 删掉 `interpreter` 参数改读宿主配置；`run_python_script` 统一解释器并保留 `success`/`data` 契约（`.expect()` 改为结构化错误）；新增 `probe_python_interpreter` 命令；测试模块改为传 `String` 解释器并新增 3 个单测。
- `src-tauri/src/main.rs`：导入并注册 `probe_python_interpreter`；`.plugin(tauri_plugin_dialog::init())`。
- `src-tauri/Cargo.toml`、`package.json`、`src-tauri/tauri.conf.json`：dialog 依赖与 `dialog:allow-open` 权限。
- `src/panels/settingComponent.jsx`：`app` tab 新增「Python 环境」卡片（输入框 + 系统文件选择器 + 即时探测反馈）、保存时显式传 `null`、打开设置页自动探测。按评审结论**未**纳入 `handleSettingReset`。
- `src/pluginRuntime.js`、`src/baseComponent.jsx`：删除 `manifest.runtime` 死字段传参与未使用的 `run_python_plugin` wrapper。
- `docs/插件开发规范.md`：第 99/109 行改为新语义。`src-tauri/src/api/python_runner.py` 移出仓库（副本在系统临时目录）。

**未验证（环境限制）**：宿主对 bash 的写类命令报读证据拦截，`cargo check` / `cargo test` / `pnpm install` / `pnpm build` 均未能运行。需本地确认：Rust 能否编译（含 3 个新单测）、前端能否构建、`tauri-plugin-dialog` 版本解析与 `@tauri-apps/plugin-dialog` 安装、以及「配置 venv → 插件 import 该环境独有的包成功」的端到端验证。恢复命令：`pnpm install`，然后 `cd src-tauri && cargo check --all-targets`。
