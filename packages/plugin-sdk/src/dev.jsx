import { definePlugin } from "./index.jsx";
import { mountControl } from "./controlRenderer.jsx";

export function createDevContext(overrides = {}) {
  let config = {};
  return {
    theme: {},
    ...overrides,
    input: { text: "", file: null, selection: null, ...overrides.input },
    ui: {
      mountControl,
      close: () => console.info("[Lark 预览] 关闭插件"),
      notify: (...args) => console.info("[Lark 预览] 通知", ...args),
      ...overrides.ui,
    },
    api: {
      getConfig: async () => structuredClone(config),
      setConfig: async (values) => { config = { ...config, ...structuredClone(values) }; },
      ...Object.fromEntries(["runPython", "openUrl", "openFile", "readClipboard", "writeClipboard"].map(name => [name, async () => {
        throw new Error(`${name} 需要在 Lark 中测试；也可以通过 createDevContext({api: {...}}) 提供测试实现`);
      }])),
      ...overrides.api,
    },
  };
}

export function previewPlugin(App, container, context = createDevContext()) {
  const runtime = definePlugin(App)(context);
  runtime.mount(container);
  return runtime;
}
