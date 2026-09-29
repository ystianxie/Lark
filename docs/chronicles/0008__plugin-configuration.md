# 插件配置声明与配置页 Chronicle

日期：2026-09-18
计划：`docs/plans/0008__plugin-configuration.md`
状态：实现完成；编译与前端构建未能在本环境验证

## 背景

翻译一类插件需要 API Key、接口地址等用户配置，但此前宿主既没有存放插件配置的地方，也没有让插件声明配置项的机制：manifest 属于发布物、会被向导的创建/编辑整体重写；`localStorage.pluginStatus` 只适合无所谓的开关，且插件与宿主同源、任何插件都能读到它；`ConfigData.plugins` 与 `register_plugin_config` 虽然已经存在，但全仓库没有调用者，也没有读写命令。

## 决策

- **配置归宿主**：按 manifest 的 `id` 存在宿主 `config.json` 的 `plugins` 字段里，不写插件目录、不写 `localStorage`。
- **界面由宿主渲染**：manifest 用可选的 `config.fields` 声明配置项，宿主生成表单，插件不写 UI 代码。
- **入口在组件库**：卡片上的「配置」按钮；「应用设置」不承载动态的插件配置。
- **密钥先抽象、后加固**：只引入 `type: "password"`，存储仍是明文；加密存储与宿主代理请求明确列为非目标，文档中如实说明其局限。
- **按需读取**：配置每次调用都向宿主取值，不做快照注入——runtime 按插件 id 缓存且 `activate` 只执行一次，快照会导致改完配置不生效。

## 实施结果

宿主侧：

- `ConfigData.plugins` 补上 `#[serde(default)]`。此前 `read_local_config` 用 `unwrap_or(ConfigData::default())` 兜底，配置里缺 `plugins` 键会让**整份配置静默回退默认值**；本期开始往该结构写数据，先堵住这个口子。
- 移除死代码 `register_plugin_config`，改为 `impl ConfigData` 上的 `plugin_settings` / `set_plugin_settings` / `remove_plugin_settings`，并新增自由函数 `plugin_settings`、`plugin_settings_map`、`save_plugin_settings_data`、`clear_plugin_settings_data`，落盘复用 `save_local_config` 的原子替换。
- 新增 4 个 Tauri 命令：`get_plugin_settings`、`save_plugin_settings`、`clear_plugin_settings`、`get_plugin_settings_status`。三者写入路径都要求插件 id 合法且已被 `load_plugins` 发现，写入前按 manifest 声明过滤 `key`，并限制序列化大小 64 KB。
- `get_plugin_settings_status` 只返回 `{configured, missing}`，配置值不进入前端内存。
- `valid_plugin_id` 改为 `pub`，让命令层与向导校验共用同一套 id 规则。
- 新增 `validate_plugin_config`，接入 `validate_plugin_files` 与 `validate_scaffold`：校验 `schemaVersion`、`ui`、`fields` 数量与唯一性、`key` 规则、`label`、`type` 白名单、`select` 的 `options`、`default` 与 `type` 的匹配、`ui=custom` 时 `settingsPage` 是否存在。缺失 `config` 时直接通过，旧插件不受影响。

插件侧与界面：

- `pluginRuntime.js` 的 `createPluginContext` 暴露 `api.getConfig()` 与 `api.setConfig(values)`，都是即时 invoke 的函数，未引入新权限项。
- 新增 `src/panels/PluginSettings.jsx`：按 `config.fields` 渲染受控表单（text/password/textarea/number/boolean/select），读取时用声明的 `default` 兜底，保存前校验必填，支持「清除配置」。对声明做归一化：未知 `type` 降级为 `text`、重复 `key` 去重、超过 24 项截断——手动安装的插件绕过后端校验，必须由渲染侧兜底。
- 组件库卡片仅在声明了非空 `fields` 时显示「配置」按钮，沿用既有的整页替换视图模式；未声明 `config` 的插件表现不变。
- 触发时拦截：`App.jsx` 在选中 `action`/`python` 入口前检查该插件的必填项，缺失时不再执行插件，而是打开该插件的配置页。组件库通过 `pluginConfigId` 接收目标插件并自动进入配置页，用 ref 保证只消费一次。
- 向导新增「配置项」编辑区，生成 `manifest.config` 与 `scaffold.json` 中的同一份声明；没有配置项时不生成 `config` 字段。

## 验证

- 新增 Rust 单元测试：`config.rs` 的缺键反序列化、`main.rs` 的 `select_plugin_settings` 与 `is_empty_config_value`、`plugins.rs` 的 4 组 `config` 声明校验。
- 静态检索确认命令定义/注册、导出、前端接线与 CSS 类均在位；对每处改动做了逐段人工复核（含多次类型与借用检查，例如修正了「`read_local_config` 返回 `ConfigData` 而非 `Config`」这一错误）。
- `cargo test --bin lark plugin_config_tests`：**未能执行**。构建脚本被环境拒绝——先是 `build-script-build` 退出码 `0xc0000022`，随后为 `LoadLibraryExW failed: 拒绝访问 (os error 5)`（加载 `serde_derive` proc-macro DLL 被拒）。这与本仓库 0001-0006 chronicle 记录的 “compile verification blocked by environment” 一致。
- `rustfmt --edition 2021 --check`：**未能执行**。宿主把 rustfmt 判定为可能写文件的工具而拒绝（`bash cannot declare which files it changes`），声明写路径后仍被拒。
- 前端 `pnpm build` / `esbuild`：**未执行**，同类限制。

## 风险与限制

- 以上 Rust 与前端改动均**未经编译、构建或运行验证**，交付物是「已写出 + 静态复核通过」。首次在有编译能力的环境里跑 `cargo test` 与 `pnpm build` 时，需留意两侧都可能存在编译期问题。
- 密钥是明文，`password` 类型只解决静态泄露的一部分；插件与宿主同源（`csp: null`、blob module 加载），配置存储不构成插件间隔离。要真正隔离需要「密钥不出 Rust 的宿主代理请求」，本期明确不做。
- `config.json` 是整文件重写，新命令与既有 `save_setting` 等写路径之间没有额外串行化；并发写入仍是「最后写入者获胜」。
- 拦截只做在「选中入口」这一步：交互式插件一旦进入输入模式，后续输入变化不会再重复检查必填项。
- `ui: "custom"`（插件自绘配置页）只在校验层预留，未实现；空 `keywords` 天然不会进入搜索结果，因此将来实现时不需要新增 `hidden` 字段。

## 顺带修复

- `creation_tests::files()` 把 `scaffold.json` 写成 `"{}"`，而 `validate_plugin_files` 末尾会调用 `validate_scaffold`（要求 `schemaVersion == 1` 等），这使得该 fixture 下所有「合法输入应通过」的断言本来就不可能成立。已改为合法的最小 scaffold。此修复同样未经运行验证。
- 新写的缺键反序列化测试自带自检：先断言默认序列化确实包含 `plugins` 键，避免将来变成「什么都没验证」的假绿。

## 后续调整：配置项 key 不再复用插件 ID 规则

初版实现里，配置项 `key` 直接复用了 `valid_plugin_id`。那是插件 ID 的规则，目的是保证「会成为目录名」的标识符路径安全：强制小写以免在大小写不敏感文件系统上撞目录、禁空段、查 Windows 保留名。

但配置项 `key` 永远不会变成路径，它只是 `manifest.json` 与 `config.json` 里的 JSON 对象键。复用那套规则的后果是：逼插件作者取一个不能点号访问的名字，还要在向导提示、生成的 README 和开发规范里教他们用 `config.get("base-url")` 去绕——把一个设计缺陷写成了使用说明。

现在改为独立规则 `valid_config_key`：`^[a-z][a-z0-9_]{0,31}$`。于是 `config.api_key`（JS）与 `config["api_key"]`（Python）都能直接取用，三处「要用下标绕」的说明一并删除。

范围与兼容性：

- 插件 ID 与功能 ID 的规则**不变**（它们确实会成为目录名），`valid_plugin_id` 的 5 处 ID 调用点原样保留。
- 存储层不校验 key（`select_plugin_settings` 只按 manifest 声明过滤），放宽规则不会让任何已存配置失效。
- Rust 校验只在向导创建/更新时执行；`PluginSettings.jsx` 对 key 只要求非空字符串，故意保持宽松，让手工安装的插件用任意 key 也能渲染表单。
- 唯一的破坏性影响在向导路径：camelCase（`apiKey`）与 kebab-case（`base-url`）现在会被明确拒绝。本期 config 功能尚无真实插件使用，没有迁移负担。
- 测试 fixture 与两份文档的示例同步改为蛇形，否则它们本身就会违反新规则。

## 当前步骤

Complete
