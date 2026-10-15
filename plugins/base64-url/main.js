function result(method, data, text) {
  return {
    title: data || "（空文本）",
    data,
    desc: `${method}结果${data === text ? " · 与原文相同" : ""}`,
  };
}
function utf8Encode(value) {
  const bytes = new TextEncoder().encode(value);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}
function utf8Decode(value) {
  const binary = atob(value.replace(/\s/g, ""));
  return new TextDecoder("utf-8", { fatal: true }).decode(Uint8Array.from(binary, char => char.charCodeAt(0)));
}
export function activate() {
  return { execute(name, payload = {}) {
    const text = payload.text || "";
    if (!text) return [];
    if (name !== "convert") throw new Error(`未知操作：${name}`);
    const methods = [
      ["Base64 编码", utf8Encode],
      ["Base64 解码", utf8Decode],
      ["URL 编码", encodeURIComponent],
      ["URL 解码", decodeURIComponent],
    ];
    return methods.flatMap(([method, convert]) => {
      try {
        return [result(method, convert(text), text)];
      } catch {
        // A failed decoder must not hide results from the other methods.
        return [];
      }
    });
  } };
}
