// 手动回归页：先 npm run build，再 npm run dev，打开 /smoke.html。
// 与 Lark 相同，以 Blob 动态导入生成的 JS，并接上宿主共用的控件实现。
import { createDevContext } from "@lark/plugin-sdk/dev";
const renderControl = createDevContext().ui.mountControl;

let runtime;
let activeControls = 0;
const status = document.getElementById("status");
const container = document.getElementById("root");
const renderStatus = (message) => {
  status.textContent = `${message}；活动控件：${activeControls}；插件样式：${document.querySelectorAll('style[data-lark-plugin="jsx-demo"]').length}`;
};
const context = createDevContext({
  input: { text: "构建测试" },
  ui: {
    mountControl(element, type, props) {
      const handle = renderControl(element, type, props);
      activeControls += 1;
      renderStatus("插件运行中");
      let disposed = false;
      const dispose = () => {
        if (disposed) return;
        disposed = true;
        handle();
        activeControls -= 1;
        renderStatus("控件已清理");
      };
      dispose.update = handle.update;
      return dispose;
    },
  },
});

async function reload() {
  try {
    runtime?.unmount();
    await Promise.resolve();
    const response = await fetch("/release/jsx-demo/dist/main.js", { cache: "no-store" });
    if (!response.ok) throw new Error(`请先 npm run build：${response.status}`);
    const url = URL.createObjectURL(new Blob([await response.text()], { type: "text/javascript" }));
    try {
      const { activate } = await import(/* @vite-ignore */ url);
      runtime = activate(context);
      runtime.mount(container);
    } finally { URL.revokeObjectURL(url); }
    requestAnimationFrame(() => requestAnimationFrame(() => renderStatus("加载成功")));
  } catch (error) { status.textContent = String(error); }
}
document.getElementById("reload").onclick = reload;
document.getElementById("unmount").onclick = async () => {
  runtime?.unmount();
  await Promise.resolve();
  renderStatus("卸载完成");
};
reload();
