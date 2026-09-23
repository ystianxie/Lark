# 自定义通知实施 TODO

关联设计：[自定义通知.md](./自定义通知.md)

## 0. 开工前决策

- [x] 暂停期间丢弃 `success/info`，保留 `warning/error`；恢复后重新按完整时长计时。
- [x] 最多完整显示 3 条，其余以层叠卡片露出一小行；队列最多 20 条。
- [x] 默认时长：`success/info` 5 秒、`warning` 8 秒、`error` 手动关闭。
- [x] 折叠最大高度为工作区 40% 且不超过 520px；展开最大高度为工作区 70% 且不超过 800px。
- [x] 通知窗口宽度 380px，顶部/右侧边距 16px，跟随系统主题。
- [x] Windows 完整实现和验收；macOS 保留结构并做基础编译验证。

## 1. Rust 模块与数据模型

- [x] 创建 `src-tauri/src/notification/mod.rs`、`model.rs`、`manager.rs`、`action.rs`。
- [x] 定义通知等级、通知 DTO、Action DTO 和序列化规则。
- [x] 实现线程安全的 `NotificationManager`，并注册到 Tauri state。
- [x] 实现唯一 ID、自动过期、手动关闭和 Action 成功后关闭；暂停中的 warning 保留并在恢复后重新计时。
- [x] 注册 `notify`、`dismiss_notification`、`invoke_notification_action`、`get_notifications`、开关和暂停 commands。
- [x] 注册 `notification-state`、`notification-window-resize` command 契约。

## 2. 通知窗口

- [x] 使用固定 label `notification` 懒创建通知 WebView 窗口。
- [x] 配置无边框、透明背景、跳过任务栏、置顶、不主动抢焦点。
- [x] 窗口接收鼠标事件，Action 可交互（实机验证待完成）。
- [x] 无通知时隐藏，新通知到达时显示；暂停/全局关闭时隐藏。
- [ ] 验证窗口关闭、主窗口隐藏、销毁、重建和应用退出时的生命周期。

## 3. React 通知 UI

- [x] 新增通知页面入口、容器和单条通知组件。
- [x] 实现 success、info、warning、error 样式。
- [x] 实现标题、消息、Action、关闭按钮和纵向堆叠。
- [x] 默认最新通知插入顶部；展开状态下新通知仍插入顶部且不自动折叠。
- [x] 点击层叠区域展开，展开状态保持；再次点击层叠区域或空白区域折叠。
- [x] 单条标题最多 2 行、消息最多 4 行，并支持受高度限制的单条展开。
- [x] 实现进入、退出和列表变化动画。
- [x] 使用 `ResizeObserver` 测量实际容器尺寸。
- [x] 对 resize 请求做短时间合并，避免高频调用 Rust `set_size`。
- [x] 窗口加载后调用 `get_notifications` 同步状态。

## 4. 前端 API

- [x] 新增 `src/utils/notify.js` 通知 API 封装。
- [x] 实现 `notify.success/info/warning/error`。
- [x] 实现 `actions.open(path)`、`actions.openDir(path)`、`actions.close()`。
- [x] 明确默认时长由 Rust 按等级决定；自定义时长限制为 1000–60000ms，error 固定手动关闭。
- [x] 严格校验 DTO 字段和 Action 类型，不接受任意事件、函数或可执行代码。
- [x] command 失败转为 `{ok:false,error}` 并记录警告，避免未处理的 rejected Promise。

## 5. 暂停与开关

- [x] 实现 `set_notifications_paused(true/false)`。
- [x] 暂停时立即隐藏窗口，暂停期间不显示通知。
- [x] 暂停期间丢弃 `success/info`，保留 `warning/error`。
- [x] 恢复时 `warning` 重新按其完整配置时长计时，不保存暂停前剩余时间。
- [x] 实现 `set_notifications_enabled(true/false)`。
- [x] 区分 `enabled=false` 与 `paused=true` 的行为和 UI 状态；两者独立，全局关闭拒绝所有新通知，暂停仅丢弃低等级通知。
- [x] 为暂停和全局关闭补充单元测试。

## 5.1 倒计时与交互

- [x] 悬停时暂停 `success/info/warning` 倒计时，移开后继续。
- [x] `error` 不自动关闭，只能手动关闭。
- [x] 卡片空白区域不执行操作，卡片内部点击不会误触发通知列表折叠。
- [x] Action 成功后关闭当前通知，失败时保留并显示错误状态；执行中禁用按钮以避免重复触发。
- [x] 首期不做重复通知合并。

## 6. 多屏右上角定位

- [x] 获取主窗口所在显示器；主窗口不可用时回退主显示器。
- [x] 使用排除任务栏的工作区。
- [x] 实现右上角定位：`x = right - width - marginRight`、`y = top + marginTop`。
- [x] 窗口尺寸使用逻辑像素，工作区/屏幕位置使用物理像素，并按 scale factor 换算边距和通知尺寸。
- [x] 窗口尺寸变化及主窗口移动、大小/DPI 变化后重新定位。
- [ ] 验证任务栏位于顶部、底部、左侧和右侧时的结果。
- [ ] 验证主窗口跨显示器移动后的通知位置。

## 7. Action 安全与复用

- [x] 校验路径非空、规范化并检查目标存在。
- [x] 区分打开文件和打开目录的目标类型。
- [x] 拒绝未知 Action、任意 Shell 命令和任意协议执行。
- [x] 复用现有 `open_file` 和 `open_explorer` 能力，并将失败转换为通知可展示的错误。
- [x] 为无权限、路径不存在、目标类型错误补充可读错误信息。
- [x] 验证插件调用通知时仍受 Action 白名单约束（统一使用 `NotificationAction` DTO）。

## 8. 测试与验收

- [x] `cargo fmt --check`。
- [x] Rust 通知模块单元测试：ID、过期、关闭、暂停、Action 校验和定位计算。
- [ ] 全仓 `cargo test`：有 2 项 `api::explorer` 测试失败；另有 `api::proxy_pool::run` 超过 60 秒未结束，本次中止。
- [ ] `pnpm build`（当前环境 Vite/esbuild 启动失败：`spawn EPERM`，需在可启动 esbuild 子进程的环境重试）。
- [ ] 手工验证长标题、长消息、中文换行和多个按钮。
- [ ] 验证连续发送 1、3、10 条通知时只有一个窗口。
- [ ] 验证暂停、恢复、全局关闭和应用重启后的行为。
- [ ] 验证不同显示器、DPI 和缩放比例下的右上角定位与尺寸。
- [ ] 验证打开文件、打开目录、文件不存在和权限失败场景。
- [ ] 验证通知窗口不会抢焦点、阻塞主窗口或出现在任务栏。
- [ ] 验证超过 3 条时层叠卡片只露出一小行，超过 20 条时按等级淘汰。
- [ ] 验证展开状态保持、可折叠，且不因新通知自动折叠。

## 9. 后续增强（不属于当前 MVP 验收）

- [x] MVP 已实现内存通知队列（最多 20 条）和低等级优先淘汰；首期不做重复通知合并。
- [x] 插件 `notification.send` 权限和受限通知 API 已实现；通知来源展示/筛选可后续再评估。
- [ ] 仅当用户需要回看已消失通知时，再设计持久化通知历史中心。
- [ ] 仅当需要用户配置通知行为时，再持久化通知时长、开关等设置。
- [ ] 在 macOS 有可用测试环境后，验证并适配窗口置顶、焦点、工作区和多屏行为。

## 10. Python 与插件通知

- [x] 为声明 `notification.send` 的 JS 插件提供 `context.ui.notify` 和 `context.ui.actions`，不暴露 Tauri `invoke`。
- [x] 明确 Panel 默认使用页面内 loading、进度、成功和错误反馈；不因 Panel 内状态重复弹全局通知。
- [x] 设计 Panel 关闭或结果跨页面时转全局通知的宿主策略：首期仅由宿主在普通一次性 workflow 完成后决定是否转发，Panel 活跃期间不转发。
- [x] 为普通 `action/python` Python 最终 JSON 结果增加可选 `notification` 字段；`run_python_plugin` 直接返回 JSON Value，保留可选顶层字段。
- [x] 在 JS runtime 中校验 Python 结果通知，并仅在普通一次性 workflow 结束后按权限转发；interactive workflow/Panel 不触发全局通知。
- [x] `notification.send` 插件权限接入 manifest 校验、创建向导和运行时桥接。
- [x] 限制 Python 结果通知只能使用通知 DTO 和白名单 Action（复用 `createNotification` 前端校验和 Rust `NotificationInput::validate`）。
- [x] 文档明确：普通 Python 不直接调用 Tauri、不持有通知窗口对象。
- [x] 文档明确：Panel 卸载后普通 Python 不承诺后台执行、结果投递或通知。
- [x] 首期不实现 Python 中途主动通知和普通 Python 流式协议。

## 11. 未来后台任务（暂不实施）

- [ ] 当出现真实长任务需求时，再新增独立 `job`/`background-job` 模型。
- [ ] 设计 `jobId`、任务状态查询、取消、完成、失败和生命周期托管。
- [ ] 设计后台 Python 子进程与插件解绑后的权限和资源回收。
- [ ] 设计 `progress`、`log`、`notification`、`completed`、`failed` 事件。
- [ ] 评估 NDJSON 流式协议；不把它加入当前一次性 `run_python_plugin` 协议。
- [ ] 仅对下载、批处理、扫描、索引等长任务评估进度条通知。
