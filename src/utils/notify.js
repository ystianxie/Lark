import { invoke } from "@tauri-apps/api/core";

const LEVELS = new Set(["success", "info", "warning", "error"]);
const OPTION_KEYS = new Set(["title", "message", "actions", "durationMs"]);
const ACTION_KEYS = {
  open: new Set(["type", "path", "label"]),
  open_dir: new Set(["type", "path", "label"]),
  close: new Set(["type", "label"]),
};

function isRecord(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function assertKnownKeys(value, allowed, name) {
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) throw new TypeError(`${name} 不支持字段：${key}`);
  }
}

function normalizeAction(action, index) {
  if (!isRecord(action) || !ACTION_KEYS[action.type]) {
    throw new TypeError(`第 ${index + 1} 个通知 Action 类型无效`);
  }
  assertKnownKeys(action, ACTION_KEYS[action.type], `Action ${index + 1}`);
  const normalized = { type: action.type };
  if (action.type !== "close") {
    if (typeof action.path !== "string" || !action.path.trim()) {
      throw new TypeError(`Action ${index + 1} 必须提供有效路径`);
    }
    normalized.path = action.path.trim();
  }
  if (action.label !== undefined) {
    if (typeof action.label !== "string" || action.label.length > 40) {
      throw new TypeError(`Action ${index + 1} 的 label 必须是 40 字以内的文本`);
    }
    normalized.label = action.label;
  }
  return normalized;
}

export function createNotification(level, options) {
  if (!LEVELS.has(level)) throw new TypeError("通知等级无效");
  if (!isRecord(options)) throw new TypeError("通知参数必须是对象");
  assertKnownKeys(options, OPTION_KEYS, "通知");
  if (typeof options.title !== "string" || !options.title.trim() || options.title.length > 200) {
    throw new TypeError("通知 title 必填且不能超过 200 个字符");
  }
  if (options.message !== undefined && (typeof options.message !== "string" || options.message.length > 4000)) {
    throw new TypeError("通知 message 必须是 4000 个字符以内的文本");
  }
  if (options.actions !== undefined && (!Array.isArray(options.actions) || options.actions.length > 3)) {
    throw new TypeError("通知 actions 必须是最多 3 项的数组");
  }
  if (level === "error" && options.durationMs !== undefined) {
    throw new TypeError("error 通知必须手动关闭，不能设置 durationMs");
  }
  if (options.durationMs !== undefined && (!Number.isInteger(options.durationMs) || options.durationMs < 1000 || options.durationMs > 60_000)) {
    throw new TypeError("durationMs 必须是 1000 到 60000 之间的整数毫秒");
  }

  return {
    level,
    title: options.title.trim(),
    ...(options.message !== undefined ? { message: options.message } : {}),
    ...(options.actions ? { actions: options.actions.map(normalizeAction) } : {}),
    ...(options.durationMs !== undefined ? { durationMs: options.durationMs } : {}),
  };
}

async function send(level, options) {
  const notification = createNotification(level, options);
  // 通知是旁路副作用，不能让插件业务等待窗口创建/显示。
  // invoke 的 Promise 仍在后台完成；调用方立即得到 queued 状态。
  void invoke("notify", { notification }).catch((error) => {
    console.warn("发送 Lark 通知失败：", error instanceof Error ? error.message : String(error));
  });
  return { ok: true, id: null, suppressed: false, queued: true };
}

export const notify = Object.freeze({
  success: (options) => send("success", options),
  info: (options) => send("info", options),
  warning: (options) => send("warning", options),
  error: (options) => send("error", options),
});

export const actions = Object.freeze({
  open: (path, label) => ({ type: "open", path, ...(label === undefined ? {} : { label }) }),
  openDir: (path, label) => ({ type: "open_dir", path, ...(label === undefined ? {} : { label }) }),
  close: (label) => ({ type: "close", ...(label === undefined ? {} : { label }) }),
});
