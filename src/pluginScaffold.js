export const pluginPermissions = ["url.open", "file.open", "clipboard.read", "clipboard.write"];

export const codeExamples = {
  action: 'return text.toUpperCase();',
  python: 'return text.upper()',
};

function iconHash(value) {
  return [...String(value || "lark-plugin")].reduce((hash, char) => ((hash << 5) - hash + char.charCodeAt(0)) | 0, 0) >>> 0;
}

export function generateDefaultIcon(seed) {
  const hash = iconHash(seed);
  const hue = hash % 360;
  const secondHue = (hue + 42 + ((hash >>> 8) % 50)) % 360;
  const mark = (String(seed || "P").replace(/[^\p{L}\p{N}]/gu, "").slice(0, 2) || "P").toUpperCase();
  return `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop stop-color="hsl(${hue} 78% 48%)"/><stop offset="1" stop-color="hsl(${secondHue} 78% 42%)"/></linearGradient></defs><rect width="64" height="64" rx="14" fill="url(#g)"/><text x="32" y="38" text-anchor="middle" font-family="Arial,sans-serif" font-size="18" font-weight="700" fill="white">${mark}</text></svg>\n`;
}

export function isSafeSvg(value) {
  return typeof value === "string" && value.length <= 256 * 1024 &&
    /^\s*<svg\b[^>]*>[\s\S]*<\/svg>\s*$/i.test(value) &&
    !/<\s*(script|foreignObject|iframe|object|embed)\b/i.test(value) &&
    !/\bon\w+\s*=/i.test(value) && !/(?:href|src)\s*=\s*["']\s*(?:https?:|data:|javascript:)/i.test(value) &&
    !/(?:url\s*\(|@import|<!ENTITY)/i.test(value);
}

export function isPluginId(value) {
  return typeof value === "string" && /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(value) && value.length <= 64 &&
    !/^(con|prn|aux|nul|com[0-9]|lpt[0-9])$/.test(value);
}

export function normalizePluginResults(value, title = "插件结果") {
  if (value === null || value === undefined || value === "") return [];
  const items = Array.isArray(value) ? value : value?.items || value?.results || [value];
  if (!Array.isArray(items)) throw new Error("插件结果必须是文本、数字、结果对象或数组");
  return items.filter(item => item !== null && item !== undefined && item !== "").map(item => {
    if (typeof item === "string" || typeof item === "number") {
      return {type: "result", title: String(item), data: String(item), desc: ""};
    }
    if (typeof item !== "object" || Array.isArray(item)) throw new Error("不支持的插件结果格式");
    const data = item.data ?? item.value ?? "";
    return {type: "result", title: String(item.title ?? item.label ?? title),
      data: typeof data === "object" ? JSON.stringify(data) : String(data),
      desc: String(item.desc ?? item.description ?? ""), ...(item.icon ? {icon: item.icon} : {})};
  });
}

export function generatePluginFiles(config, existingIds = []) {
  const id = config.id.trim();
  const name = config.name.trim();
  if (!isPluginId(id)) throw new Error("插件 ID 请使用小写字母、数字及单个短横线，最多 64 字符，不能使用系统保留名称");
  if (existingIds.some(existing => existing.toLowerCase() === id)) throw new Error("插件 ID 已存在，请更换 ID");
  if (!name || !config.description.trim()) throw new Error("请填写插件名称和描述");
  if (!config.workflows.length) throw new Error("请至少添加一个功能入口");
  const ids = new Set();
  const permissions = new Set();
  const workflows = config.workflows.map(workflow => {
    const workflowId = workflow.id.trim();
    if (!isPluginId(workflowId) || ids.has(workflowId)) throw new Error("功能 ID 必须合法且不能重复");
    ids.add(workflowId);
    if (!workflow.title.trim()) throw new Error(`请填写 ${workflowId} 的名称`);
    const keywords = [...new Set(workflow.keywords.split(/[,，\n]/).map(keyword => keyword.trim()).filter(Boolean))];
    if (!keywords.length) throw new Error(`请填写 ${workflowId} 的关键词，多个关键词用逗号分隔`);
    if (!["action", "url", "python"].includes(workflow.type)) throw new Error("不支持的功能类型");
    const result = {id: workflowId, title: workflow.title.trim(), keywords, type: workflow.type,
      description: config.description.trim(), icon: "./assets/icon.svg"};
    if (workflow.type === "url") {
      let url;
      try { url = new URL(workflow.url.trim()); } catch { throw new Error(`请填写 ${workflowId} 的完整网址`); }
      if (!["https:", "http:", "mailto:", "chrome-extension:", "moz-extension:"].includes(url.protocol)) {
        throw new Error("网址仅支持 http、https、mailto 或浏览器扩展协议");
      }
      permissions.add("url.open");
      result.data = url.href;
    } else {
      if (!workflow.code.trim()) throw new Error(`请填写 ${workflowId} 的业务代码`);
      result.handler = workflowId;
      result.interactive = true;
      result.debounceMs = 200;
      if (workflow.type === "python") permissions.add("python.execute");
      else for (const permission of workflow.permissions || []) {
        if (!pluginPermissions.includes(permission)) throw new Error("不支持的 API 权限");
        permissions.add(permission);
      }
    }
    return result;
  });
  const icon = isSafeSvg(config.icon) ? config.icon : generateDefaultIcon(`${id}:${Date.now()}`);
  const manifest = {id, name, version: "1.0.0", apiVersion: 1, description: config.description.trim(),
    icon: "./assets/icon.svg", entry: {type: "js", path: "dist/main.js"}, workflows, permissions: [...permissions]};
  const handlers = config.workflows.filter(workflow => workflow.type !== "url").map(workflow => {
    const body = workflow.type === "python"
      ? `const response = await context.api.runPython(${JSON.stringify(workflow.id.trim())}, {text, file});\nif (!response?.ok) throw new Error(response?.error || "Python 执行失败");\nreturn response.result;`
      : workflow.code;
    return `[${JSON.stringify(workflow.id.trim())}, async (text, file, context) => {\n${body}\n}]`;
  });
  const javascript = `const normalizePluginResults = ${normalizePluginResults.toString()};

export async function activate(context) {
  const handlers = new Map([
${handlers.join(",\n")}
  ]);
  return {
    async execute(name, payload = {}) {
      const handler = handlers.get(name);
      if (!handler) throw new Error("未知功能：" + name);
      const text = payload.text ?? "";
      const file = payload.file ?? null;
      const currentContext = {...context, input: {...context.input, text, file}};
      return normalizePluginResults(await handler(text, file, currentContext), ${JSON.stringify(name)});
    }
  };
}
`;
  const files = {
    "manifest.json": JSON.stringify(manifest, null, 2) + "\n",
    "dist/main.js": javascript,
    "assets/icon.svg": icon,
    "scaffold.json": JSON.stringify({schemaVersion: 1, ...config, id, name}, null, 2) + "\n",
    "README.md": `# ${name}\n\n${manifest.description}\n\n在 Lark 中搜索功能关键词并选择入口。action / python 选中后会先用空输入执行一次，随后在非空输入变化后以 200ms 防抖实时执行；不要用于需要确认的破坏性操作。结果选中后沿用宿主的文本粘贴行为。\n\n## 修改代码\n\n入口位于 dist/main.js，Python 逻辑位于 python/main.py（如有）。无需编译。scaffold.json 保存创建时的配置，仅供开发参考，修改它不会自动重新生成入口。修改已经加载的 JS 后请重启 Lark，manifest 修改后刷新组件库。\n\nJS 函数体可使用 text、file、context；Python 函数体可使用 text、file。返回字符串、数字、{title, data, desc} 对象或其数组；返回 null / None 或空字符串表示无结果。JS 可使用 await，但应通过 context.api 调用已声明的宿主能力。Python stdout 留给 JSON 协议，print 会被包装层重定向到 stderr。\n\n每个 workflow 的 handler 字段就是入口函数绑定名；Python 插件共用一个 python/main.py，通过 request.task 按 handler 名称分发到对应函数，因此一个 Python 文件可以提供多个能力。\n\nPython 使用宿主现有解释器设置或系统 python.exe / python3，本模板不创建虚拟环境或安装依赖。\n`,
  };
  const pythonWorkflows = config.workflows.filter(workflow => workflow.type === "python");
  if (pythonWorkflows.length) {
    const functions = pythonWorkflows.map((workflow, index) =>
      `def handler_${index}(text, file):\n${workflow.code.split("\n").map(line => `    ${line}`).join("\n")}\n`);
    const dispatch = pythonWorkflows.map((workflow, index) => `${JSON.stringify(workflow.id.trim())}: handler_${index}`).join(", ");
    files["python/main.py"] = `import contextlib\nimport json\nimport sys\n\n${functions.join("\n")}\nhandlers = {${dispatch}}\n\ntry:\n    request = json.load(sys.stdin)\n    args = request.get("args", {})\n    handler = handlers[request["task"]]\n    with contextlib.redirect_stdout(sys.stderr):\n        result = handler(args.get("text", ""), args.get("file"))\n    response = {"ok": True, "result": result}\n    encoded = json.dumps(response, ensure_ascii=True)\nexcept Exception as error:\n    encoded = json.dumps({"ok": False, "error": str(error)}, ensure_ascii=True)\nprint(encoded, flush=True)\n`;
  }
  return files;
}
