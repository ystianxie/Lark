const alphabet = "ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
function randomBytes(length) {
  const bytes = new Uint8Array(length);
  crypto.getRandomValues(bytes);
  return bytes;
}
function uuid() {
  const bytes = randomBytes(16);
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = [...bytes].map(byte => byte.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}
function random(length) {
  const size = Math.min(Math.max(Number.parseInt(length, 10) || 16, 1), 256);
  return [...randomBytes(size)].map(byte => alphabet[byte % alphabet.length]).join("");
}
export function activate() {
  return { execute(name, payload = {}) {
    if (name === "uuid") {
      const data = uuid();
      return { title: data, data, desc: "UUID v4 生成结果" };
    }
    if (name === "random") {
      const data = random(payload.text);
      return { title: data, data, desc: `随机字符串生成结果 · ${data.length} 位` };
    }
    throw new Error(`未知操作：${name}`);
  } };
}
