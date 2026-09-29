# 插件图标支持位图格式 Chronicle

日期：2026-09-18
状态：实现完成；编译与前端构建未能在本环境验证
来源：用户在完成 `docs/plans/0008__plugin-configuration.md` 后提出的优化，未单独编写计划文档

## 背景

向导创建/编辑插件时只接受 SVG 图标：`readIcon` 用 `readAsText` 读文本、`config.icon` 存 SVG 源码、`generatePluginFiles` 写死 `assets/icon.svg`，Rust 的 `PLUGIN_FILES` 白名单也只允许这一个图标路径。

但运行时其实早就支持位图——`plugins/json-handle/manifest.json` 就是 `"icon": "./icon.png"`，组件库用 `convertFileSrc(root + icon路径)` 塞进 `<img>` 加载。所以缺的只是向导这条链路：从设计工具导出一张 PNG 想当图标，只能手工改 manifest 和目录。

## 决策

两条路线里选了「把位图写成 `assets/icon.<ext>` 文件」而不是「把 data URL 内联进 manifest」：前者与既有 SVG 处理完全对称，manifest 保持只存路径，图标还能被手工替换；后者虽只改前端，但 manifest 与 `scaffold.json` 会被几十 KB base64 撑大，且与"图标是独立文件"的设计不一致。

格式范围一次性放开到 PNG、JPEG、WebP、ICO（都按扩展名枚举，不是任意路径），体积上限沿用与 SVG 一致的 256 KB。

## 实施结果

宿主侧（`src-tauri/src/config/plugins.rs`）：

- `PLUGIN_FILES` 从 6 项拆为 5 项固定文件，新增 `ICON_FILES`（svg/png/jpg/webp/ico）与 `is_plugin_file`；必填检查改为「固定文件 + 至少一个图标」。
- 新增 `BASE64_ICON_FILES`、`ICON_MAX_BYTES` 与 `icon_file_bytes`：位图先 base64 解码，再校验文件头（PNG 8 字节 / JPEG `FF D8 FF` / WebP 需同时含 `RIFF` 与 `WEBP` / ICO `00 00 01 00`）与体积；SVG 仍按文本直通。`validate_plugin_files` 与 `write_plugin_files` 共用这一个函数，避免"校验的字节"和"写下的字节"不一致。
- `update_plugin` 的备份集合扩到图标文件，写盘前解码，并在切换格式后删除本次未提交的旧图标，避免 `png` 换回 `svg` 时残留两个文件。
- `load_plugin_editor` 回填图标改为 `read_icon_for_editor`：优先读 `assets/icon.svg` 文本，否则把位图读成 `data:<mime>;base64,...` 供前端预览。

前端：

- `pluginScaffold.js` 新增 `bitmapExtension` 与 `bitmapRejectionReason`（与 `svgRejectionReason` 对称，返回具体原因或 `null`），文件头规则与宿主 `icon_file_bytes` 保持一致，防止前端放行、后端拒绝。
- `generatePluginFiles` 按图标类型生成 `assets/icon.svg` 或 `assets/icon.<ext>`，manifest 与每个 workflow 的 `icon` 都指向实际提供的那个；位图只把 data URL 里逗号之后的 base64 部分交给宿主。
- `PluginCreator.jsx` 增加 `applyIconBitmap` 与 `iconKindFromNameAndType`：`readIcon`（File 对象）与 `readIconPath`（窗口拖放只给路径）都按类型分流——SVG 走 `readAsText` / `read_txt`，位图走 `readAsDataURL` / `read_file_to_base64`；预览按 `bitmapExtension` 决定用 `<img>` 还是内联 SVG；`accept` 与拖拽提示同步。

顺带改动：「元数据读取命令的健壮性」

- `read_file_to_base64` 原本是 `-> String` 且内部用 `.expect()`，读不到文件就 panic。拖放位图必须经它读二进制，一次误拖不该影响整个应用，因此改为 `-> Result<String, String>`，并同步修正 `read_icns_to_base64` 内部那处调用。该命令此前没有前端调用方，改签名不影响现有功能。

## 验证

- `node --check src/pluginScaffold.js`：通过（项目 `"type": "module"`，按 ESM 解析）。这是本次唯一跑通的自动验证。
- 静态检索确认：白名单判定、必填、4 种位图扩展名、`icon_file_bytes` 的调用点（校验与写盘）、`read_icon_for_editor`、前端 `bitmapExtension`/`bitmapRejectionReason` 的使用点均已就位。
- 新增 Rust 单测（`icon_bytes_decode_bitmap_and_validate_header_and_size`、`plugin_creation_requires_an_icon_file`）覆盖文件头、体积、非法 base64 与"图标至少一个"分支，但**未运行**。
- `cargo test`：环境不可用（proc-macro DLL 加载被拒，同 0008）。
- 前端 `pnpm build`：未执行（同类限制）。

## 风险与限制

- 位图从上传到落盘是一条跨语言的 base64 链路，前端只校验了开头 64 个字符的文件头，宿主再次校验；两侧规则一致但都未实际运行比对过。
- 图标体积上限 256 KB 是硬限制，超过会被前端与宿主分别拒绝。
- `manifest.icon` 与图标文件名必须匹配（向导保证）；手工安装的插件不受白名单限制，`icon` 仍可指向插件目录内任意图片。
- `PluginCreator.jsx` 是 JSX，`node --check` 无法解析，其接线为逐行阅读确认，没有机器验证。
- 需要人工回归：向导上传 PNG → 预览 → 生成 → 组件库显示；以及 png 与 svg 互相切换后目录里不残留旧图标。

## 当前步骤

Complete
