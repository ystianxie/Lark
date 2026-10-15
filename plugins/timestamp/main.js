export function activate(context) {
  return {
    async execute(name, payload = {}) {
      if (name !== "convert") throw new Error(`未知操作：${name}`);
      return context.api.runPython("convert", { text: payload.text || "" });
    },
  };
}
