# 时间戳 Python 插件示例

这是一个可复制的“输入驱动 Python action”模板：

- `manifest.json` 声明 `action` workflow 和 `python.execute` 权限；
- `dist/main.js` 导出 `activate()`，在 `execute()` 中调用 `context.api.runPython()`；
- `python/main.py` 通过 JSON Lines 接收任务，返回 `result.items`；
- 宿主会把每个 item 转成 `result`，因此按 Enter 选择某一项时会复制其 `data` 到剪贴板。

复制此目录后，至少需要修改 `manifest.json` 的 `id`、名称、关键词，以及 Python 的任务逻辑。
