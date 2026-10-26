export function activate(context) {
  const getSelectedFile = (payload) => {
    const file = typeof payload.file === "string" ? payload.file.trim() : "";
    if (!file) throw new Error("请先将一个文件拖入主窗口，再运行此 Action 示例。");
    return file;
  };

  const getParentDirectory = (file) => {
    const lastSeparator = Math.max(file.lastIndexOf("/"), file.lastIndexOf("\\"));
    if (lastSeparator <= 0) throw new Error("无法从当前文件路径识别所在目录。");
    return file.slice(0, lastSeparator);
  };

  return {
    async execute(name, payload = {}) {
      if (name === "javascriptNotification") {
        const data = "JavaScript 能力执行完成";
        return {
          ok: true,
          result: { title: data, data, desc: "通知示例" },
          notification: { level: "info", title: "通知示例", message: data, durationMs: 3000 },
        };
      }

      if (name === "closeActionNotification") {
        await context.ui.notify.info({
          title: "关闭 Action 示例",
          message: "点击按钮可关闭这条通知。",
          actions: [context.ui.actions.close("知道了")],
        });
        return { title: "关闭 Action 示例" };
      }

      if (name === "openActionNotification" || name === "openDirActionNotification") {
        const file = getSelectedFile(payload);
        const isDirectoryAction = name === "openDirActionNotification";
        const target = isDirectoryAction ? getParentDirectory(file) : file;
        const action = isDirectoryAction
          ? context.ui.actions.openDir(target, "打开所在位置")
          : context.ui.actions.open(file, "打开文件");
        await context.ui.notify.info({
          title: isDirectoryAction ? "打开目录 Action 示例" : "打开文件 Action 示例",
          message: target,
          actions: [action],
        });
        return { title: "通知 Action 示例", data: target };
      }

      if (name === "pythonNotification") {
        const response = await context.api.runPython(name, {
          text: payload.text ?? "",
          file: payload.file ?? null,
        });
        if (!response?.ok) throw new Error(response?.error || "Python 执行失败");
        return response;
      }

      throw new Error(`未知操作：${name}`);
    },
  };
}
