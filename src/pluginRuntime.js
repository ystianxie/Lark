import { invoke, convertFileSrc } from "@tauri-apps/api/core";

const runtimes = new Map();

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
    .replaceAll("\\\\", "/")
    .replace(/\/+$/, "");
}

export function createPluginContext(manifest, state = {}) {
  const api = {};
  for (const permission of manifest.permissions || []) {
    if (permissions[permission]) api[apiNames[permission] || permission.replace('.', '_')] = permissions[permission];
  }
  if ((manifest.permissions || []).includes("python.execute")) {
    api.runPython = (task, args = {}) => invoke("run_python_plugin", {
      scriptPath: `${normalizePluginPath(manifest.__root)}/python/main.py`,
      request: { id: `${manifest.id}-${Date.now()}`, task, args },
      interpreter: manifest.runtime?.pythonPath || null,
      timeoutMs: manifest.runtime?.timeoutMs || 30000,
    });
  }
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
  const module = await import(/* @vite-ignore */ url);
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
