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

- [ ] 创建 `src-tauri/src/notification/mod.rs`、`model.rs`、`manager.rs`、`action.rs`。
- [ ] 定义通知等级、通知 DTO、Action DTO 和序列化规则。
- [ ] 实现线程安全的 `NotificationManager`，并注册到 Tauri state。
- [ ] 实现唯一 ID、自动过期、手动关闭和 Action 后关闭。
- [ ] 注册 `notify`、`dismiss_notification`、`invoke_notification_action`、`get_notifications`、开关和暂停 commands。
- [ ] 注册 `notification-state`、`notification-window-resize` 事件。

## 2. 通知窗口

- [ ] 使用固定 label `notification` 创建通知 WebView 窗口。
- [ ] 配置无边框、透明/背景、跳过任务栏、置顶、不主动抢焦点。
- [ ] 验证通知窗口可接收鼠标事件，Action 不受鼠标穿透影响。
- [ ] 无通知时隐藏，新通知到达时显示。
- [ ] 验证窗口关闭、主窗口隐藏、销毁、重建和应用退出时的生命周期。

## 3. React 通知 UI

- [ ] 新增通知页面入口、容器和单条通知组件。
- [ ] 实现 success、info、warning、error 样式。
- [ ] 实现标题、消息、Action、关闭按钮和纵向堆叠。
- [ ] 默认最新通知插入顶部；展开状态下新通知仍插入顶部且不自动折叠。
- [ ] 点击层叠区域展开，展开状态保持；再次点击层叠区域或空白区域折叠。
- [ ] 单条标题最多 2 行、消息最多 4 行，并支持受高度限制的单条展开。
- [ ] 实现进入、退出和列表变化动画。
- [ ] 使用 `ResizeObserver` 测量实际容器尺寸。
- [ ] 对 resize 请求做短时间合并，避免高频调用 Rust `set_size`。
- [ ] 窗口加载后调用 `get_notifications` 同步状态。

## 4. 前端 API

- [ ] 新增 `src/utils/notify.js` 或同等位置的通知 API 封装。
- [ ] 实现 `notify.success/info/warning/error`。
- [ ] 实现 `actions.open(path)`、`actions.openDir(path)`、`actions.close()`。
- [ ] 明确 `durationMs` 默认值、最小值和最大值。
- [ ] 禁止函数、任意事件名和可执行字符串进入 DTO。
- [ ] 处理 command 失败，避免未捕获 Promise 报错。

## 5. 暂停与开关

- [ ] 实现 `set_notifications_paused(true/false)`。
- [ ] 暂停时立即隐藏窗口，暂停期间不显示通知。
- [ ] 暂停期间丢弃 `success/info`，保留 `warning/error`。
- [ ] 恢复时 `warning` 重新按完整 8 秒计时，不保存暂停前剩余时间。
- [ ] 实现 `set_notifications_enabled(true/false)`。
- [ ] 区分 `enabled=false` 与 `paused=true` 的行为和 UI 状态。
- [ ] 为暂停和全局关闭补充单元测试。

## 5.1 倒计时与交互

- [ ] 悬停时暂停 `success/info/warning` 倒计时，移开后继续。
- [ ] `error` 不自动关闭，只能手动关闭。
- [ ] 卡片空白区域不执行操作。
- [ ] Action 成功后关闭当前通知，失败时保留并显示错误状态。
- [ ] 首期不做重复通知合并。

## 6. 多屏右上角定位

- [ ] 获取主窗口所在显示器；主窗口不可用时回退主显示器。
- [ ] 使用排除任务栏的工作区。
- [ ] 实现 `x = right - width - marginRight`、`y = top + marginTop`。
- [ ] 统一逻辑像素、物理像素和 scale factor 转换。
- [ ] 窗口尺寸、显示器或 DPI 变化后重新定位。
- [ ] 验证任务栏位于顶部、底部、左侧和右侧时的结果。
- [ ] 验证主窗口跨显示器移动后的通知位置。

## 7. Action 安全与复用

- [ ] 校验路径非空、规范化并检查目标存在。
- [ ] 区分打开文件和打开目录的目标类型。
- [ ] 拒绝未知 Action、任意 Shell 命令和任意协议执行。
- [ ] 复用现有 `open_file` 和 `open_explorer` 能力。
- [ ] 为无权限、路径不存在、目标类型错误补充可读错误信息。
- [ ] 验证插件调用通知时仍受 Action 白名单约束。

## 8. 测试与验收

- [ ] `cargo fmt --check`。
- [ ] Rust 单元测试：ID、过期、关闭、暂停、Action 校验。
- [ ] `pnpm build`。
- [ ] 手工验证长标题、长消息、中文换行和多个按钮。
- [ ] 验证连续发送 1、3、10 条通知时只有一个窗口。
- [ ] 验证暂停、恢复、全局关闭和应用重启后的行为。
- [ ] 验证不同显示器、DPI 和缩放比例下的右上角定位与尺寸。
- [ ] 验证打开文件、打开目录、文件不存在和权限失败场景。
- [ ] 验证通知窗口不会抢焦点、阻塞主窗口或出现在任务栏。
- [ ] 验证超过 3 条时层叠卡片只露出一小行，超过 20 条时按等级淘汰。
- [ ] 验证展开状态保持、可折叠，且不因新通知自动折叠。

## 9. 后续增强

- [ ] 通知队列、淘汰和合并策略。
- [ ] 通知历史中心。
- [ ] 插件通知权限和来源标识。
- [ ] 通知配置持久化。
- [ ] macOS 窗口行为与视觉适配。

## 10. Python 与插件通知

- [ ] 为 JS 插件提供统一的 `context.ui.notify(...)` 接口，不暴露 Tauri `invoke`。
- [ ] 明确 Panel 默认使用页面内 loading、进度、成功和错误反馈。
- [ ] 设计 Panel 关闭或结果跨页面时转全局通知的宿主策略。
- [ ] 为普通 `action/python` 结果增加可选 `notification` 字段。
- [ ] 在 JS runtime 中校验并转发 Python 结果中的 `notification`。
- [ ] 为 Python 结果通知增加 `notification.send` 权限控制（如确定插件需要声明权限）。
- [ ] 限制 Python 结果通知只能使用通知 DTO 和白名单 Action。
- [ ] 文档明确：普通 Python 不直接调用 Tauri、不持有通知窗口对象。
- [ ] 文档明确：Panel 卸载后普通 Python 不承诺后台执行、结果投递或通知。
- [ ] 首期不实现 Python 中途主动通知和普通 Python 流式协议。

## 11. 未来后台任务（暂不实施）

- [ ] 当出现真实长任务需求时，再新增独立 `job`/`background-job` 模型。
- [ ] 设计 `jobId`、任务状态查询、取消、完成、失败和生命周期托管。
- [ ] 设计后台 Python 子进程与插件解绑后的权限和资源回收。
- [ ] 设计 `progress`、`log`、`notification`、`completed`、`failed` 事件。
- [ ] 评估 NDJSON 流式协议；不把它加入当前一次性 `run_python_plugin` 协议。
- [ ] 仅对下载、批处理、扫描、索引等长任务评估进度条通知。
