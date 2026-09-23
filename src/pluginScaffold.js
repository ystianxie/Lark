export const pluginPermissions = ["url.open", "file.open", "clipboard.read", "clipboard.write", "notification.send"];

// 与后端 validate_plugin_config 的类型白名单保持一致。
export const pluginConfigFieldTypes = [
  {value: "text", label: "text · 单行文本"},
  {value: "password", label: "password · 密钥（掩码显示）"},
  {value: "textarea", label: "textarea · 多行文本"},
  {value: "number", label: "number · 数字"},
  {value: "boolean", label: "boolean · 开关"},
  {value: "select", label: "select · 下拉选择"},
];

export const newConfigField = () => ({key: "", label: "", description: "", type: "text",
  required: false, defaultValue: "", options: ""});

// 向导里的默认值以文本录入，这里按类型转成 manifest 需要的 JSON 类型。
function configDefaultValue(field, fieldType) {
  const raw = String(field.defaultValue ?? "").trim();
  if (!raw) return undefined;
  if (fieldType === "number") {
    const parsed = Number(raw);
    if (!Number.isFinite(parsed)) throw new Error(`配置项 ${field.key} 的默认值必须是数字`);
    return parsed;
  }
  if (fieldType === "boolean") {
    if (raw !== "true" && raw !== "false") throw new Error(`配置项 ${field.key} 的默认值只能是 true 或 false`);
    return raw === "true";
  }
  return raw;
}

// 把向导里的配置项编辑结果转成 manifest.config.fields 并做前端校验，
// 规则与 Rust 的 validate_plugin_config 对齐，避免生成出后端拒绝的插件。
function normalizeConfigFields(fields) {
  if (!Array.isArray(fields) || !fields.length) return [];
  if (fields.length > 24) throw new Error("配置项最多 24 个");
  const keys = new Set();
  return fields.map(field => {
    const key = String(field.key || "").trim();
    if (!isConfigKey(key)) throw new Error("配置项变量名请使用小写字母开头的蛇形命名（小写字母、数字、下划线），最多 32 字符");
    if (keys.has(key)) throw new Error(`配置项变量名重复：${key}`);
    keys.add(key);
    const label = String(field.label || "").trim();
    if (!label) throw new Error(`请填写配置项 ${key} 的显示名称`);
    const type = pluginConfigFieldTypes.some(item => item.value === field.type) ? field.type : "text";
    const result = {key, label, type};
    const description = String(field.description || "").trim();
    if (description) result.description = description;
    if (field.required) result.required = true;
    if (type === "select") {
      const options = String(field.options || "").split(/[,，\n]/).map(item => item.trim()).filter(Boolean);
      if (!options.length) throw new Error(`配置项 ${key} 是下拉类型，请至少填写一个选项`);
      result.options = options;
    }
    const defaultValue = configDefaultValue(field, type);
    if (defaultValue !== undefined) result.default = defaultValue;
    return result;
  });
}

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

// 从 Figma / Illustrator / Inkscape 导出的 SVG 常带 XML 声明、注释或 DOCTYPE 前缀，
// 它们都属于正常的静态图形，因此结构检查允许这些前缀，只要求以 <svg> 开始、以 </svg> 结束。
// 注释前缀限定最多 8 个：真实文件不会更多，同时避免大量未闭合的 <!-- 触发灾难性回溯。
const svgFilePattern = /^\s*(?:<\?xml[^>]*\?>\s*)?(?:<!--[\s\S]*?-->\s*){0,8}(?:<!DOCTYPE[^>]*>\s*)?<svg\b[\s\S]*<\/svg>\s*$/i;

// 返回不允许用作插件图标的原因，null 表示可用。拆成返回值是为了让向导提示具体原因，
// 而不是笼统地说「SVG 仅允许静态图形」，让用户无从下手。
// 注意 url() 只禁止指向外部资源：url(#id) 是同文件内的渐变/滤镜/图案引用，属于正常静态图形。
export function svgRejectionReason(value) {
  if (typeof value !== "string" || !value.trim()) return "文件内容为空";
  if (value.length > 256 * 1024) return "文件超过 256 KB";
  if (!svgFilePattern.test(value)) {
    return "不是完整的 SVG 文件（需要以 <svg> 开始、以 </svg> 结束）";
  }
  if (/<\s*(script|foreignObject|iframe|object|embed)\b/i.test(value)) {
    return "包含不允许的元素（script、foreignObject、iframe、object 或 embed）";
  }
  if (/\bon\w+\s*=/i.test(value)) return "包含事件属性（如 onclick）";
  if (/(?:href|src)\s*=\s*["']\s*(?:https?:|data:|javascript:)/i.test(value)) {
    return "引用了外部资源（href 或 src 指向 http、data 或 javascript）";
  }
  if (/@import/i.test(value)) return "包含 @import";
  if (/<!ENTITY/i.test(value)) return "包含实体定义（<!ENTITY）";
  // 不能写成 url\s*\(\s*["']?\s*(?!\s*#)：可选引号在断言失败后会回溯成「不匹配引号」，
  // 于是 url("#g") 这类同文件引用会被误判成外部资源。这里先吃掉引号，再要求下一个字符不是 # 或引号。
  if (/url\s*\(\s*["']?[^#\s"']/i.test(value)) {
    return "引用了外部资源（url() 只能指向同一文件内的 #id）";
  }
  return null;
}

export function isSafeSvg(value) {
  return svgRejectionReason(value) === null;
}

// 位图图标的 MIME 与扩展名映射。数据在向导内以 data URL 传递，落盘时由宿主解码二进制。
const bitmapMimeExtensions = {
  "image/png": "png",
  "image/jpeg": "jpg",
  "image/webp": "webp",
  "image/x-icon": "ico",
  "image/vnd.microsoft.icon": "ico",
};

/// 图标是位图 data URL 时返回扩展名，SVG 文本或空值返回 null。
export function bitmapExtension(value) {
  if (typeof value !== "string") return null;
  const match = /^data:([^;,]+);base64,/.exec(value);
  if (!match) return null;
  return bitmapMimeExtensions[match[1].trim().toLowerCase()] || null;
}

// 与 svgRejectionReason 对称：返回不允许使用的原因，null 表示可用。
// 文件头校验与宿主侧 icon_file_bytes 保持一致，避免前端放行、后端拒绝。
export function bitmapRejectionReason(value) {
  const extension = bitmapExtension(value);
  if (!extension) return "只支持 PNG、JPEG、WebP 或 ICO 图标";
  const payload = value.slice(value.indexOf(",") + 1);
  let head;
  try {
    // 判断文件头只需要开头若干字节，不必解码整张图。
    head = atob(payload.slice(0, 64));
  } catch {
    return "图标数据已损坏，请重新选择文件";
  }
  const bytes = [...head].slice(0, 12).map(char => char.charCodeAt(0));
  const startsWith = signature => signature.every((byte, index) => bytes[index] === byte);
  const matched = {
    png: () => startsWith([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    jpg: () => startsWith([0xff, 0xd8, 0xff]),
    webp: () => head.slice(0, 4) === "RIFF" && head.slice(8, 12) === "WEBP",
    ico: () => startsWith([0x00, 0x00, 0x01, 0x00]),
  }[extension];
  if (!matched()) return `文件内容不像是 ${extension.toUpperCase()} 图片`;
  if (payload.length * 3 / 4 > 256 * 1024) return "文件超过 256 KB";
  return null;
}

export function isPluginId(value) {
  return typeof value === "string" && /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(value) && value.length <= 64 &&
    !/^(con|prn|aux|nul|com[0-9]|lpt[0-9])$/.test(value);
}

// 配置项 key 只是 JSON 对象键，不会变成目录名，因此不复用插件 ID 的规则：
// 限定蛇形命名，让 JS 侧可以点号访问（config.api_key）；Python 侧读字典照常用下标或 get。
export function isConfigKey(value) {
  return typeof value === "string" && /^[a-z][a-z0-9_]{0,31}$/.test(value);
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
  // 图标可能是位图 data URL，也可能仍是 SVG 文本；位图交给宿主解码落盘。
  const iconBitmap = bitmapExtension(config.icon);
  const icon = iconBitmap
    ? config.icon
    : (isSafeSvg(config.icon) ? config.icon : generateDefaultIcon(`${id}:${Date.now()}`));
  const iconPath = iconBitmap ? `./assets/icon.${iconBitmap}` : "./assets/icon.svg";
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
      description: config.description.trim(), icon: iconPath};
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
      result.interactive = workflow.interactive !== false;
      if (result.interactive) result.debounceMs = workflow.debounceMs === 200 ? 200 : 80;
      if (workflow.type === "python") {
        permissions.add("python.execute");
      }
      for (const permission of workflow.permissions || []) {
        if (!pluginPermissions.includes(permission)) throw new Error("不支持的 API 权限");
        permissions.add(permission);
      }
    }
    return result;
  });
  const configFields = normalizeConfigFields(config.configFields);
  const manifest = {id, name, version: "1.0.0", apiVersion: 1, description: config.description.trim(),
    icon: iconPath, entry: {type: "js", path: "dist/main.js"}, workflows, permissions: [...permissions]};
  // 没有配置项时不写 config 字段，生成结果与旧版本完全一致。
  if (configFields.length) manifest.config = {schemaVersion: 1, fields: configFields};
  const scaffoldConfig = {schemaVersion: 1, ...config, id, name};
  if (configFields.length) scaffoldConfig.config = {schemaVersion: 1, fields: configFields};
  const handlers = config.workflows.filter(workflow => workflow.type !== "url").map(workflow => {
    const body = workflow.type === "python"
      ? `const config = await context.api.getConfig();\nconst response = await context.api.runPython(${JSON.stringify(workflow.id.trim())}, {text, file, config});\nif (!response?.ok) throw new Error(response?.error || "Python 执行失败");\nreturn {ok: true, result: response.result, ...(response.notification === undefined ? {} : {notification: response.notification})};`
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
      const value = await handler(text, file, currentContext);
      if (value && typeof value === "object" && !Array.isArray(value) && Object.hasOwn(value, "notification")) {
        return {ok: value.ok === true, result: normalizePluginResults(value.result, ${JSON.stringify(name)}), notification: value.notification};
      }
      return normalizePluginResults(value, ${JSON.stringify(name)});
    }
  };
}
`;
  // 位图只把 data URL 里的 base64 部分交给宿主，由宿主解码成二进制写入；
  // SVG 仍是完整文本，与旧行为一致。
  const iconFiles = iconBitmap
    ? {[`assets/icon.${iconBitmap}`]: icon.slice(icon.indexOf(",") + 1)}
    : {"assets/icon.svg": icon};
  const files = {
    "manifest.json": JSON.stringify(manifest, null, 2) + "\n",
    "dist/main.js": javascript,
    ...iconFiles,
    "scaffold.json": JSON.stringify(scaffoldConfig, null, 2) + "\n",
    "README.md": `# ${name}\n\n${manifest.description}\n\n在 Lark 中搜索功能关键词并选择入口。action / python 选中后会先用空输入执行一次，随后在非空输入变化后以 200ms 防抖实时执行；不要用于需要确认的破坏性操作。结果选中后沿用宿主的文本粘贴行为。\n\n## 修改代码\n\n入口位于 dist/main.js，Python 逻辑位于 python/main.py（如有）。无需编译。scaffold.json 保存创建时的配置，仅供开发参考，修改它不会自动重新生成入口。修改已经加载的 JS 后请重启 Lark，manifest 修改后刷新组件库。\n\nJS 函数体可使用 text、file、context；Python 函数体可使用 text、file、config（已保存的插件配置字典，未配置时为空字典；键即配置项变量名，用 config.get("api_key") 读取）。返回字符串、数字、{title, data, desc} 对象或其数组；返回 null / None 或空字符串表示无结果。JS 可使用 await，但应通过 context.api 调用已声明的宿主能力。声明了 config.fields 的插件可用 await context.api.getConfig() 读取用户在组件库中填写的配置，并用 context.api.setConfig(values) 写回；配置保存在宿主配置里，不会改写插件文件。Python stdout 留给 JSON 协议，print 会被包装层重定向到 stderr。\n\n每个 workflow 的 handler 字段就是入口函数绑定名；Python 插件共用一个 python/main.py，通过 request.task 按 handler 名称分发到对应函数，因此一个 Python 文件可以提供多个能力。\n\nPython 使用宿主现有解释器设置或系统 python.exe / python3，本模板不创建虚拟环境或安装依赖。\n`,
  };
  const pythonWorkflows = config.workflows.filter(workflow => workflow.type === "python");
  if (pythonWorkflows.length) {
    const functions = pythonWorkflows.map((workflow, index) =>
      `def handler_${index}(text, file, config):\n${workflow.code.split("\n").map(line => `    ${line}`).join("\n")}\n`);
    const dispatch = pythonWorkflows.map((workflow, index) => `${JSON.stringify(workflow.id.trim())}: handler_${index}`).join(", ");
    files["python/main.py"] = `import contextlib\nimport json\nimport sys\n\n${functions.join("\n")}\nhandlers = {${dispatch}}\n\ntry:\n    request = json.load(sys.stdin)\n    args = request.get("args", {})\n    handler = handlers[request["task"]]\n    with contextlib.redirect_stdout(sys.stderr):\n        result = handler(args.get("text", ""), args.get("file"), args.get("config") or {})\n    if isinstance(result, dict) and "notification" in result:\n        response = {"ok": True, "result": result.get("result"), "notification": result["notification"]}\n    else:\n        response = {"ok": True, "result": result}\n    encoded = json.dumps(response, ensure_ascii=True)\nexcept Exception as error:\n    encoded = json.dumps({"ok": False, "error": str(error)}, ensure_ascii=True)\nprint(encoded, flush=True)\n`;
  }
  return files;
}
