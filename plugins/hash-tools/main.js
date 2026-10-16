const algorithms = ["SHA-1", "SHA-256", "SHA-384", "SHA-512"];
const iterations = 600000;
const maxTextLength = 8 * 1024 * 1024;
const encoder = new TextEncoder();

export function formatDigest(bytes, format = "hex") {
  if (format === "hex") return Array.from(bytes, byte => byte.toString(16).padStart(2, "0")).join("");
  let binary = "";
  // Chunking keeps large AES payloads within the function argument limit.
  for (let i = 0; i < bytes.length; i += 8192) binary += String.fromCharCode(...bytes.subarray(i, i + 8192));
  return btoa(binary);
}

export function parseDigest(text, format = "hex") {
  const value = text.trim();
  if (format === "hex") {
    if (!/^(?:[a-f\d]{2})+$/i.test(value)) throw new Error("请输入完整的 Hex 字节（仅支持 0–9、a–f）");
    return Uint8Array.from(value.match(/../g), byte => parseInt(byte, 16));
  }
  if (!/^(?:[A-Za-z\d+/]{4})*(?:[A-Za-z\d+/]{2}==|[A-Za-z\d+/]{3}=)?$/.test(value) || !value) throw new Error("请输入标准 Base64（区分大小写，包含必要的 = 填充）");
  const bytes = Uint8Array.from(atob(value), char => char.charCodeAt(0));
  if (formatDigest(bytes, "base64") !== value) throw new Error("Base64 编码不规范");
  return bytes;
}

function textBytes(text) {
  if (text.length > maxTextLength) throw new Error("输入过大，请使用不超过 8 MiB 的 UTF-8 文本");
  const bytes = encoder.encode(text);
  if (bytes.length > maxTextLength) throw new Error("输入过大，请使用不超过 8 MiB 的 UTF-8 文本");
  return bytes;
}
function checkAlgorithm(algorithm) {
  if (!algorithms.includes(algorithm)) throw new Error("不支持的摘要算法");
}
export async function hashText(text, algorithm) {
  checkAlgorithm(algorithm);
  return new Uint8Array(await crypto.subtle.digest(algorithm, textBytes(text)));
}
async function hmacKey(secret, algorithm) {
  checkAlgorithm(algorithm);
  if (!secret) throw new Error("请输入 HMAC 密钥（UTF-8 文本）");
  return crypto.subtle.importKey("raw", textBytes(secret), {name: "HMAC", hash: algorithm}, false, ["sign", "verify"]);
}
export async function signHmac(text, secret, algorithm) {
  const key = await hmacKey(secret, algorithm);
  return new Uint8Array(await crypto.subtle.sign("HMAC", key, textBytes(text)));
}
export async function verifyHmac(text, secret, algorithm, signature) {
  const key = await hmacKey(secret, algorithm);
  return crypto.subtle.verify("HMAC", key, signature, textBytes(text));
}
async function derive(password, salt) {
  if (!password) throw new Error("请输入 AES 密码");
  const base = await crypto.subtle.importKey("raw", textBytes(password), "PBKDF2", false, ["deriveKey"]);
  return crypto.subtle.deriveKey({name: "PBKDF2", salt, iterations, hash: "SHA-256"}, base, {name: "AES-GCM", length: 256}, false, ["encrypt", "decrypt"]);
}
export async function encryptText(text, password) {
  const data = textBytes(text);
  const salt = crypto.getRandomValues(new Uint8Array(16));
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const key = await derive(password, salt);
  const ciphertext = new Uint8Array(await crypto.subtle.encrypt({name: "AES-GCM", iv, tagLength: 128}, key, data));
  return JSON.stringify({v: 1, alg: "AES-256-GCM", kdf: "PBKDF2-SHA256", iter: iterations, salt: formatDigest(salt, "base64"), iv: formatDigest(iv, "base64"), data: formatDigest(ciphertext, "base64")}, null, 2);
}
export async function decryptText(packetText, password) {
  if (packetText.length > 12 * 1024 * 1024) throw new Error("数据包过大");
  let packet;
  try { packet = JSON.parse(packetText); } catch { throw new Error("请输入本工具生成的完整 JSON 数据包"); }
  if (packet?.v !== 1 || packet.alg !== "AES-256-GCM" || packet.kdf !== "PBKDF2-SHA256" || packet.iter !== iterations) throw new Error("不支持的数据包版本或算法参数");
  if (typeof packet.salt !== "string" || packet.salt.length !== 24 || typeof packet.iv !== "string" || packet.iv.length !== 16 || typeof packet.data !== "string") throw new Error("数据包字段不完整或长度不合法");
  const salt = parseDigest(packet.salt, "base64"), iv = parseDigest(packet.iv, "base64"), data = parseDigest(packet.data, "base64");
  if (salt.length !== 16 || iv.length !== 12 || data.length < 16 || data.length > maxTextLength + 16) throw new Error("数据包字段长度不合法");
  const key = await derive(password, salt);
  try {
    const bytes = await crypto.subtle.decrypt({name: "AES-GCM", iv, tagLength: 128}, key, data);
    return new TextDecoder("utf-8", {fatal: true}).decode(bytes);
  } catch { throw new Error("解密失败：密码错误或数据包已损坏"); }
}

export function activate(context) {
  let cleanup;
  return {
    mount(container) {
      cleanup?.();
      let disposed = false, revision = 0, busy = false, hasOutput = false;
      const state = {mode: "hash", operation: "encrypt", algorithm: "SHA-256", format: "hex", text: "", secret: "", expected: ""};
      const controls = [];
      const root = document.createElement("div");
      root.className = "larkPluginPanelRoot";
      root.style.cssText = "box-sizing:border-box;height:100%;padding:14px;display:flex;flex-direction:column;gap:10px;overflow:auto;font-family:inherit;font-size:13px;line-height:1.5";
      const title = document.createElement("strong"); title.textContent = "Hash / 加密工具";
      const help = document.createElement("div"); help.style.cssText = "font-size:12px;opacity:.75";
      const status = document.createElement("div"); status.style.cssText = "min-height:20px;font-size:12px"; status.setAttribute("role", "status");
      function mount(parent, type, props) {
        const host = document.createElement("div"); parent.append(host);
        const control = context.ui.mountControl(host, type, props); controls.push(control);
        return {host, control};
      }
      function field(parent, label) {
        const host = document.createElement("div"); host.style.cssText = "display:flex;flex-direction:column;gap:5px";
        const text = document.createElement("div"); text.style.cssText = "font-size:12px;opacity:.8"; text.textContent = label;
        host.append(text); parent.append(host); return {host, text};
      }
      root.append(title);
      const tabs = mount(root, "Tabs", {activeKey: "hash", size: "small", items: [{key: "hash", label: "Hash 摘要 / 校验"}, {key: "hmac", label: "HMAC 生成 / 验证"}, {key: "aes", label: "AES 加密 / 解密"}], onChange: value => {
        state.mode = value; state.secret = ""; secret.control.update({value: ""}); tabs.control.update({activeKey: value}); invalidate(); updateMode();
      }});
      const settings = document.createElement("div"); settings.style.cssText = "display:flex;gap:10px;flex-wrap:wrap"; root.append(settings);
      const algorithm = mount(settings, "Select", {value: state.algorithm, style: {width: 135}, "aria-label": "摘要算法", options: algorithms.map(value => ({value, label: value})), onChange: value => { state.algorithm = value; algorithm.control.update({value}); invalidate(); }});
      const format = mount(settings, "Select", {value: state.format, style: {width: 125}, "aria-label": "结果编码", options: [{value: "hex", label: "Hex"}, {value: "base64", label: "Base64"}], onChange: value => { state.format = value; format.control.update({value}); invalidate(); }});
      const operation = mount(settings, "Select", {value: "encrypt", style: {width: 150}, "aria-label": "AES 操作", options: [{value: "encrypt", label: "AES-GCM 加密"}, {value: "decrypt", label: "AES-GCM 解密"}], onChange: value => { state.operation = value; operation.control.update({value}); invalidate(); updateMode(); }});
      root.append(help);
      const sourceField = field(root, "输入文本（UTF-8，保留所有空白与换行）");
      mount(sourceField.host, "TextArea", {defaultValue: "", rows: 5, "aria-label": "输入文本", placeholder: "输入文本；空文本也可以计算摘要或加密", onChange: event => { state.text = event.target.value; invalidate(); }});
      const secretField = field(root, "密钥 / 密码（UTF-8 文本）");
      const secret = mount(secretField.host, "Password", {value: "", autoComplete: "off", "aria-label": "密钥或密码", placeholder: "只在当前面板使用，不保存", onChange: event => { state.secret = event.target.value; secret.control.update({value: state.secret}); invalidate(); }});
      const expectedField = field(root, "预期摘要 / HMAC（可选，编码与上方一致）");
      mount(expectedField.host, "Input", {"aria-label": "预期结果", placeholder: "填入后执行校验；Hex 不区分大小写，Base64 区分大小写", onChange: event => { state.expected = event.target.value; invalidate(); }});
      const actions = document.createElement("div"); actions.style.cssText = "display:flex;gap:8px"; root.append(actions);
      const execute = mount(actions, "Button", {type: "primary", label: "计算 / 校验", onClick: () => run()});
      const copy = mount(actions, "Button", {label: "复制输出", disabled: true, onClick: async () => {
        if (!hasOutput) return;
        try { await context.api.writeClipboard(outputValue); if (!disposed) status.textContent = "已复制"; }
        catch (error) { if (!disposed) status.textContent = `复制失败：${error.message || error}`; }
      }});
      const outputField = field(root, "输出");
      const output = mount(outputField.host, "TextArea", {value: "", readOnly: true, rows: 5, "aria-label": "计算结果", style: {fontFamily: "ui-monospace,Consolas,monospace"}});
      let outputValue = "";
      root.append(status); container.replaceChildren(root);
      function invalidate() {
        revision++;
        hasOutput = false; outputValue = ""; output.control.update({value: ""}); copy.control.update({disabled: true});
        status.textContent = busy ? "输入已改变，请等待本次计算结束后重新执行" : "";
      }
      function updateMode() {
        const aes = state.mode === "aes";
        algorithm.host.hidden = aes; format.host.hidden = aes; operation.host.hidden = !aes;
        secretField.host.style.display = state.mode === "hash" ? "none" : "flex";
        expectedField.host.style.display = aes ? "none" : "flex";
        sourceField.text.textContent = aes && state.operation === "decrypt" ? "本工具生成的完整 JSON 数据包" : "输入文本（UTF-8，保留所有空白与换行）";
        execute.control.update({label: aes ? state.operation === "encrypt" ? "加密" : "解密" : "计算 / 校验"});
        help.textContent = aes ? "AES-256-GCM · PBKDF2-SHA256 · 随机 salt / IV。解密需要完整数据包及原密码；不支持自动识别其他工具的密文格式。" : state.mode === "hmac" ? "HMAC 用于消息认证，支持生成与验证；密钥按 UTF-8 文本读取。" : "Hash 是不可逆摘要。输入预期摘要即可比较；SHA-1 仅用于兼容旧数据。";
      }
      async function run() {
        if (busy || disposed) return;
        const current = ++revision;
        const value = {...state};
        busy = true; hasOutput = false; outputValue = ""; output.control.update({value: ""}); copy.control.update({disabled: true});
        execute.control.update({loading: true}); status.textContent = "计算中…";
        try {
          let result, message;
          if (value.mode === "aes") {
            result = value.operation === "encrypt" ? await encryptText(value.text, value.secret) : await decryptText(value.text, value.secret);
            message = value.operation === "encrypt" ? "加密完成；请保存完整 JSON 数据包，并另行保管密码" : "解密成功";
          } else {
            // Validate a supplied expected value before doing any cryptographic work.
            const expected = value.expected.trim() ? parseDigest(value.expected, value.format) : null;
            const bytes = value.mode === "hash" ? await hashText(value.text, value.algorithm) : await signHmac(value.text, value.secret, value.algorithm);
            result = formatDigest(bytes, value.format);
            if (expected) {
              const valid = value.mode === "hmac" ? await verifyHmac(value.text, value.secret, value.algorithm, expected) : expected.length === bytes.length && expected.every((byte, i) => byte === bytes[i]);
              message = valid ? "校验通过：结果一致" : "校验失败：结果与预期不一致";
            } else message = `${value.mode === "hmac" ? "HMAC / " : ""}${value.algorithm} · ${bytes.length} 字节摘要`;
          }
          if (disposed || revision !== current) return;
          outputValue = result; hasOutput = true; output.control.update({value: result}); copy.control.update({disabled: false}); status.textContent = message;
        } catch (error) {
          if (!disposed && revision === current) status.textContent = error.message || String(error);
        } finally {
          busy = false;
          if (!disposed) execute.control.update({loading: false});
        }
      }
      updateMode();
      cleanup = () => { disposed = true; revision++; state.secret = ""; controls.forEach(dispose => dispose()); root.remove(); };
    },
    unmount() { cleanup?.(); cleanup = null; },
  };
}
