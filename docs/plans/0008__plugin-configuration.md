# 插件配置声明与配置页

日期：2026-09-18
当前步骤：Complete（实施结果与验证限制见 `docs/chronicles/0008__plugin-configuration.md`）

> 说明：本文的行号引用基于 2026-09-18 的工作区状态编写（工作区当时包含快捷键 `file_jump` 相关的未提交修改，与本计划无关）。实施时若行号漂移，以函数名与标识符为准。

## 目标

让插件能够声明自己需要哪些配置项（API Key、接口地址、模型名等），由宿主在组件库内渲染统一的配置表单并持久化保存；插件在运行时按需读取这些值。密钥类字段以 `password` 类型抽象，为将来换成加密存储或宿主代理请求留出升级空间，本期不实现加密。

## 已确认的设计决策

1. **数据归宿主**：配置存宿主的 `config.json`，按 manifest 的 `id` 为键，不写插件目录、不写 `localStorage`。
2. **界面由宿主渲染**：插件在 manifest 里声明字段，宿主用 antd 渲染表单。插件自绘的配置页作为逃生通道保留，但本期不实现。
3. **入口在组件库**：卡片上加"配置"按钮，进入独立配置视图。"应用设置"不承载插件配置。

## 术语

为避免实现时混淆，本文区分三样东西：

- **配置声明**：`manifest.json` 里的 `config` 对象，描述"有哪些配置项"，属于插件发布物。
- **配置值**：用户在配置页填的内容，存 `config.json` 的 `plugins.<id>`，属于用户数据。
- **配置完整性**：声明的 `required` 字段是否都已填写，由宿主计算，只用于提示和拦截。

## 协议：manifest 的 config 声明

`config` 是可选字段，没有它的插件行为完全不变（现有 `timestamp`、`json-handle` 不受影响）。

```json
"config": {
  "schemaVersion": 1,
  "ui": "form",
  "fields": [
    {"key": "api_key", "label": "API Key", "type": "password", "required": true,
     "placeholder": "在服务商控制台获取"},
    {"key": "base_url", "label": "接口地址", "type": "text",
     "default": "https://api.deepseek.com/v1"},
    {"key": "model", "label": "模型", "type": "select",
     "options": ["deepseek-chat", "deepseek-reasoner"], "default": "deepseek-chat"},
    {"key": "timeout_seconds", "label": "超时（秒）", "type": "number", "default": 30, "min": 1, "max": 120}
  ]
}
```

字段属性：

| 属性 | 适用类型 | 说明 |
| --- | --- | --- |
| `key` | 全部 | 必填，唯一，蛇形命名 `^[a-z][a-z0-9_]{0,31}$`（**不复用** `valid_plugin_id`：配置项 key 只是 JSON 键，不是路径） |
| `label` | 全部 | 必填，表单标签 |
| `type` | 全部 | 必填，`text` / `password` / `number` / `boolean` / `select` / `textarea` |
| `required` | 全部 | 可选，默认 `false`，只影响校验与拦截，不影响存储 |
| `default` | 全部 | 可选，类型需与 `type` 匹配 |
| `placeholder` / `description` | 全部 | 可选，辅助显示 |
| `options` | `select` | 必填非空，支持 `["a"]` 或 `[{"value":"a","label":"A"}]` |
| `min` / `max` | `number` | 可选 |

`ui` 取值 `form`（默认）或 `custom`。`custom` 时额外要求 `settingsPage` 指向一个已存在的 workflow `id`，本期不实现该分支（见阶段 7）。

## 存储与命令

存储结构直接放值对象，宿主不解释字段语义：

```json
{
  "base": { },
  "plugins": {
    "translate": {"api_key": "sk-...", "model": "deepseek-chat"}
  }
}
```

`ConfigData.plugins: HashMap<String, Value>`（`config/config.rs:210`）与 `register_plugin_config`（`:384`）已经存在，但全仓库没有调用者、也没有读写命令——本期把它接线成正式能力。

| 命令 | 参数 | 行为 |
| --- | --- | --- |
| `get_plugin_settings` | `pluginId` | 返回该插件的配置值对象，无记录时返回 `{}` |
| `save_plugin_settings` | `pluginId`, `values` | 覆盖写入，只接受该插件声明中存在的 `key` |
| `clear_plugin_settings` | `pluginId` | 删除该插件的配置记录 |
| `get_plugin_settings_status` | 无 | 返回 `{pluginId: {configured: bool, missing: [key]}}`，只含布尔与字段名，不含任何值 |

约束：

- `pluginId` 必须通过 `valid_plugin_id`（`config/plugins.rs:21`）**且**出现在 `load_plugins()` 的结果中，否则 `config.json` 可被写入任意键。
- 写入前读取该插件 manifest 的 `config.fields`，未声明的 `key` 一律丢弃。
- 序列化后大小上限 64 KB。
- 命令需 `#[tauri::command(rename_all = "camelCase")]`，与 `save_snippet_settings` 一致。
- 插件被删除或卸载时配置**保留**，由配置页的"清除配置"显式删除。

## 实施步骤

### 阶段 0：修复配置反序列化隐患（前置）

- 文件：`src-tauri/src/config/config.rs:210`
- 改动：`plugins` 字段加 `#[serde(default)]`。
- 原因：`read_local_config()` 使用 `serde_json::from_reader(..).unwrap_or(ConfigData::default())`（`:271`），而 `plugins` 没有 `serde(default)`，缺键会让**整个配置文件静默回退默认值**，用户全部设置丢失。现在没暴露只是因为序列化总会写出 `"plugins": {}`（已确认 `config.json` 中确实存在该键）。本期开始往这个结构里写插件配置，必须先堵这个口子。
- 验收：先写断言为红——把 `ConfigData::default()` 序列化后删掉 `"plugins"` 键，反序列化应成功且 `plugins` 为空；加 `#[serde(default)]` 后转绿。测试只构造内存 JSON，**不得触碰真实 config.json**。

### 阶段 1：宿主侧配置读写

- `src-tauri/src/config/config.rs`：新增 `plugin_settings(&self, id) -> Option<&Value>`、`set_plugin_settings(&mut self, id, Value)`、`clear_plugin_settings(&mut self, id)`；新增模块级函数 `plugin_settings(id)` / `save_plugin_settings_data(id, values)` / `clear_plugin_settings_data(id)`，落盘复用 `save_local_config()`（`:353`，已有 temp + backup + rename 的原子替换）。改造或删除已死的 `register_plugin_config`。
- `src-tauri/src/config/mod.rs`：导出新函数。
- `src-tauri/src/main.rs`：新增四个 `#[tauri::command]`（含只返回完整性摘要的 `get_plugin_settings_status`）并注册进 `invoke_handler` 列表（约 `:480-505`）。
- 验收：Rust 单元测试覆盖——未发现的 `pluginId` 被拒绝、未声明的 `key` 被丢弃、超过 64 KB 被拒绝、写入后 `get` 能读回、`clear` 后返回 `{}`。全部用临时目录，不碰真实配置。

### 阶段 2：config 声明校验

- `src-tauri/src/config/plugins.rs`：在 `validate_plugin_files`（`:37`）中增加 `config` 校验；`validate_scaffold`（`:150`）同步，保证向导编辑时声明一致。
- 校验规则：`schemaVersion == 1`；`fields` 非空且 ≤ 24 项；`key` 走独立的 `valid_config_key`（蛇形，不复用插件 ID 规则）且不重复；`type` 在枚举内；`select` 必须有非空 `options`；`default` 类型与 `type` 匹配；`ui` 为 `form`/`custom`；`ui == "custom"` 时 `settingsPage` 必须命中某个 workflow `id`。
- 注意边界：`validate_plugin_files` 只在**向导创建/更新**时执行，`load_plugins`（`:330`）直接透传 manifest。手动安装的插件可以带任意 `config`，因此前端渲染必须防御：未知 `type` 降级为 `text`，`fields` 超过上限就截断，非法 `options` 忽略。
- 验收：单元测试逐条覆盖上述规则，含边界（重复 key、空 options、错误 default 类型、不存在的 settingsPage）；`cargo test` 通过。

### 阶段 3：插件运行时读取

- `src/pluginRuntime.js:25` `createPluginContext`：新增 `api.getConfig` / `api.setConfig`，前者的实现是 `invoke("get_plugin_settings", {pluginId: manifest.id})`。
- 硬性约束：**必须是每次调用都 invoke 的函数，不能把配置快照进 context**。`loadPluginRuntime` 把 runtime 缓存在 `runtimes: Map`（`:3`、`:47`），`activate(context)` 只执行一次，快照会导致"改了配置不生效"这类难查的 bug。
- 不新增权限项：现有 `permissions` 管的是宿主能力与用户资源（`url.open`、`clipboard.*`、`python.execute`），配置是插件自己的数据，声明即授权。
- 验收：手工验证——插件执行一次后修改配置，再次执行无需重启即可读到新值。

### 阶段 4：组件库配置视图

- `src/template.jsx:465` `loadCustomComponent`：manifest 已通过 `{...manifest}`（`:476`）展开，`plugin.config` 直接可用，无需新增 `__config`。注意 `plugin.config` 是**声明**（来自 manifest），配置**值**只能通过 `get_plugin_settings` 读取，实现时不要混用这两个词。
- 新增 `src/panels/PluginSettings.jsx`：用 antd `Form` 按字段渲染；`password` → `Input.Password`，`number` → `InputNumber`（带 `min`/`max`），`boolean` → `Switch`，`select` → `Select`，`textarea` → `Input.TextArea`；必填校验；保存调 `save_plugin_settings`；提供"清除配置"。
- `src/panels/showComponent.jsx`：加 `configuring` 状态，沿用现有 `creating || editing` 的整页替换模式；卡片按钮仅在 `plugin.config?.fields?.length` 时出现，避免影响未声明配置的插件。
- 验收：新建一个带 `config` 的测试插件，走完"填写 → 保存 → 返回组件库 → 重新打开 → 值正确回显"；未声明 `config` 的现有插件（`timestamp`、`json-handle`）组件库表现不变。

### 阶段 5：触发时拦截

- `src/App.jsx` 的 `confirmComponentSelected` 守卫（约 `:349-350`）附近：选中 workflow 且该插件 `required` 字段不齐全时，不执行插件，返回一条提示项"XX 插件尚未配置 YY，点击打开配置"，选中后打开该插件的配置视图。
- 可选：搜索列表（`:719` 附近的插件过滤）给未配置插件加"未配置"角标。
- 完整性状态取自 `get_plugin_settings_status`（阶段 1），由 Rust 侧比对 `load_plugins()` 的声明与已存值计算，**只回传布尔与缺失字段名**，避免把其他插件的密钥读进前端内存；在 `refreshPlugins` 之后刷新一次即可，配置页保存后主动失效。
- 验收：新建未配置插件，搜索其关键词 → 出现提示项而非执行报错；配置完成后再搜索 → 正常执行。

### 阶段 6：向导与文档

- `src/panels/PluginCreator.jsx`：增加"配置项"编辑区，沿用"功能入口" `fieldset` 的动态增删模式，字段为 `key` / `label` / `description` / `type` / `required` / `default` / `options`。
- `src/pluginScaffold.js:49` `generatePluginFiles`：写出 `manifest.config`，并把同一份写进 `scaffold.json`（`scaffold.json` 的定位本就是"仅供开发参考的镜像配置"）。
- `docs/插件开发规范.md`：Manifest 示例（第 18-29 行）、组件库章节（第 39-45 行）、新增"插件配置"章节、`JavaScript API` 章节（第 65-69 行，补 `getConfig`/`setConfig`）、向导章节（第 47-63 行，补配置项编辑器）。
- 验收：向导创建带配置项的插件 → 生成文件预览中 `manifest.config` 与 `scaffold.json` 一致 → Rust 校验通过 → 组件库出现配置按钮。

### 阶段 7（延后，本期不做）：自定义配置页

`config.ui: "custom"` 的逃生通道，用于 OAuth 授权、"测试连接"、从 API 拉取模型列表这类静态表单渲染不出的交互。成本明显高于主路径，建议等出现真实需求再做，届时需要：

- 放宽 `validate_plugin_files` 对 `settingsPage` workflow 的 `keywords` 非空要求（`config/plugins.rs:102`）。
- 放宽该 workflow 的 `type` 允许 `panel`（`:141` 目前明确拒绝）。
- 向导支持生成 panel（当前明确"不生成 panel"，`docs/插件开发规范.md:49`）。
- `PluginView` 挂载时提供 settings 上下文与保存反馈。

补充结论：空 `keywords`（`[]`）天然不会被搜索命中（`App.jsx` 用 `keywords.some(...)` 匹配；前端 `template.jsx:469` 的 `validWorkflows` 对空数组判定为合法），所以**不需要新增 `hidden` 字段**，只需精确放宽 Rust 校验即可。

## 验证

- `cargo test`：阶段 0/1/2 的单元测试全部通过。
- `pnpm build`：前端构建通过。
- 手工闭环：向导创建带 `config` 的插件 → 组件库配置 → 搜索触发 → 未配置拦截 → 配置后正常执行。
- 回归：未声明 `config` 的现有插件（`timestamp`、`json-handle`、`curl-to-requests`）在组件库与搜索中的表现不变。
- 安全复核：`config.json` 中不存在未声明 `key`；配置命令拒绝未知 `pluginId`；日志不打印密钥。

## 风险与边界

- **密钥是明文**：`type: "password"` 只是抽象，不提供加密。它防的是静态泄露（翻配置文件、云同步、截图、日志），**不防恶意插件**——插件运行在宿主同源 WebView 主上下文（`tauri.conf.json` 中 `"csp": null`，入口经 blob module 加载），能读写 `localStorage`、hook `fetch`。要真正隔离必须走"密钥不出 Rust 的宿主代理请求"，那是另一档方案，本期明确不做，但 `password` 抽象保证升级时 manifest 声明与配置页都不用改。
- **config.json 是整文件重写**：多个写路径并发会互相覆盖（现有插件创建用的是 `CREATE_PLUGIN_LOCK`）。配置写入需要与现有写盘路径同样的串行化处理，否则"保存配置"可能覆盖同时发生的"保存设置"。
- **手动安装的插件绕过校验**：校验只覆盖向导路径，前端渲染必须自带防御（见阶段 2）。
- **插件 id 大小写**：`load_plugins` 的去重与 `valid_plugin_id` 都要求小写，配置键沿用同一 id，不做额外规范化。
- **未验证项**：本计划基于静态阅读，`ConfigData.plugins` 缺 `serde(default)` 会静默重置配置这一判断尚未用真实损坏配置复现，阶段 0 的测试就是它的复现与证明。

## 非目标

- 不做密钥加密（DPAPI / 凭据管理器）与宿主网络代理。
- 不做插件沙箱、权限隔离或插件间互不信任。
- 不做配置云同步、导入导出、多环境切换。
- 不把配置写进插件目录（`PLUGIN_FILES` 不变），不改 `scaffold.json` 的宿主定位。
- 不动 `pluginStatus` 现有的 `localStorage` 存储方式。
- 不实现阶段 7 的自定义配置页。
- 不覆盖工作区内已有的其他未提交修改（快捷键 `file_jump` 相关改动）。

## 完成后

按仓库惯例补 `docs/chronicles/0008__plugin-configuration.md`。
