export function activate(context) {
  let dispose;
  return {
    mount(container) {
      dispose?.();
      const panel = document.createElement("div");
      panel.style.cssText = "padding:20px;display:flex;flex-direction:column;gap:12px;height:100%;box-sizing:border-box";
      const title = document.createElement("strong");
      title.textContent = "cURL 转 Python Requests";
      const input = document.createElement("textarea");
      input.placeholder = "粘贴 cURL 命令，自动转换";
      const output = document.createElement("textarea");
      output.readOnly = true;
      output.placeholder = "转换结果";
      for (const field of [input, output]) {
        field.style.cssText = "flex:1;min-height:90px;resize:none;padding:12px;border:1px solid #8886;border-radius:8px;background:transparent;color:inherit;font-family:monospace;font-size:13px;line-height:1.6";
      }
      let timer;
      let revision = 0;
      input.oninput = () => {
        clearTimeout(timer);
        const current = ++revision;
        if (!input.value.trim()) {
          output.value = "";
          return;
        }
        timer = setTimeout(async () => {
          try {
            const response = await context.api.runPython("convert", { curl: input.value });
            if (current === revision) {
              output.value = response.ok ? response.result.code : response.error;
            }
          } catch (error) {
            if (current === revision) output.value = String(error);
          }
        }, 250);
      };
      panel.append(title, input, output);
      container.replaceChildren(panel);
      input.focus();
      dispose = () => {
        revision++;
        clearTimeout(timer);
        input.oninput = null;
        panel.remove();
      };
    },
    unmount() {
      dispose?.();
      dispose = null;
    },
  };
}
