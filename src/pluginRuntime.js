import { invoke, convertFileSrc } from "@tauri-apps/api/core";

const runtimes = new Map();

// 向导更新插件文件后，仅卸载该插件的旧运行时；下次执行时会从磁盘重新加载。
export async function invalidatePluginRuntime(pluginId) {
  const runtime = runtimes.get(pluginId);
  try {
    await runtime?.unmount?.();
  } catch (error) {
    console.warn(`插件 ${pluginId} 卸载失败`, error);
  } finally {
    runtimes.delete(pluginId);
  }
}

const permissions = {
  "url.open": (url) => invoke("open_url", { url }),
  "file.open": (path) => invoke("open_file", { filePath: path }),
  "clipboard.read": () => invoke("clipboard_control", { text: "", control: "read", paste: false }),
  "clipboard.write": (text) => invoke("clipboard_control", { text, control: "write", paste: false, dataType: "text" }),
};
const apiNames = {
  "url.open": "openUrl",
  "file.open": "openFile",
  "clipboard.read": "readClipboard",
  "clipboard.write": "writeClipboard",
};

export function normalizePluginPath(path) {
  return String(path || "")
    .replace(/^\\\\\?\\/, "")
    .replaceAll("\\", "/")
    .replace(/\/+$/, "");
}

export function createPluginContext(manifest, state = {}) {
  const api = {};
  for (const permission of manifest.permissions || []) {
    if (permissions[permission]) api[apiNames[permission] || permission.replace('.', '_')] = permissions[permission];
  }
  if ((manifest.permissions || []).includes("python.execute")) {
    // 解释器由宿主的「应用设置 → Python 环境」决定，不再从 manifest 读取：
    // manifest.runtime.pythonPath 从来没有写入端，是个只读不写的死字段（已移除）。
    // 同样不传 timeoutMs，由宿主用默认的 30 秒。
    api.runPython = async (task, args = {}) => {
      const response = await invoke("run_python_plugin", {
        scriptPath: `${normalizePluginPath(manifest.__root)}/python/main.py`,
        request: { id: `${manifest.id}-${Date.now()}`, task, args },
      });
      const stderr = response?.__larkPythonStderr;
      if (stderr) console.log(`[插件 ${manifest.id} / Python ${task}]\n${stderr}`);
      if (response && typeof response === "object") delete response.__larkPythonStderr;
      return response;
    };
  }
  // 配置是插件自己的数据，不额外引入权限项：宿主只按 manifest 的 config 声明过滤键。
  // 必须每次调用都向宿主取值，不能在此快照——runtime 会按插件 id 缓存且 activate 只执行一次，
  // 快照会导致用户在组件库改完配置后不生效。
  api.getConfig = () => invoke("get_plugin_settings", { pluginId: manifest.id });
  api.setConfig = (values) => invoke("save_plugin_settings", { pluginId: manifest.id, values });
  return {
    theme: state.theme || {},
    input: { text: state.text || "", file: state.file || null, selection: state.selection || null },
    ui: state.ui || {},
    api,
  };
}

export async function loadPluginRuntime(manifest, state = {}) {
  const key = manifest.id;
  if (runtimes.has(key)) return runtimes.get(key);
  if (manifest.entry?.type !== "js" || !manifest.entry.path) {
    throw new Error(`插件 ${key} 没有 JS entry`);
  }
  const root = manifest.__root || "";
  const path = manifest.entry.path.startsWith("./")
    ? `${root}/${manifest.entry.path.slice(2)}`
    : (manifest.entry.path.includes(":") ? manifest.entry.path : `${root}/${manifest.entry.path}`);
  const source = await fetch(convertFileSrc(path)).then((response) => {
    if (!response.ok) throw new Error(`插件入口加载失败: ${response.status}`);
    return response.text();
  });
  const url = URL.createObjectURL(new Blob([source], { type: "text/javascript" }));
  let module;
  try { module = await import(/* @vite-ignore */ url); }
  finally { URL.revokeObjectURL(url); }
  if (typeof module.activate !== "function") throw new Error(`插件 ${key} 未导出 activate()`);
  const runtime = await module.activate(createPluginContext(manifest, state));
  runtimes.set(key, runtime);
  return runtime;
}

export async function executePluginWorkflow(manifest, workflow, state = {}, payload = {}) {
  const runtime = await loadPluginRuntime(manifest, state);
  if (typeof runtime?.execute !== "function") throw new Error(`插件 ${manifest.id} 未提供 execute()`);
  return runtime.execute(workflow.handler || workflow.action || workflow.id, payload);
}
