const fieldStyle = "box-sizing:border-box;width:100%;padding:10px;border:1px solid #8886;border-radius:8px;background:transparent;color:inherit;font-family:inherit;font-size:13px;line-height:1.5";

function formatMatches(matches, truncated = false) {
  if (!matches.length) return "无匹配";
  const lines = matches.map((match, index) => {
    const groups = match.groups?.length ? `  捕获组=${JSON.stringify(match.groups)}` : "";
    const named = match.named && Object.keys(match.named).length ? `  命名组=${JSON.stringify(match.named)}` : "";
    return `${index + 1}. ${JSON.stringify(match.match)}  index=${match.index}${groups}${named}`;
  });
  if (truncated) lines.push("…结果已截断（最多显示 500 项）");
  return lines.join("\n");
}

export function activate(context) {
  let cleanup;
  return {
    mount(container) {
      const root = document.createElement("div");
      root.style.cssText = "padding:18px;display:flex;flex-direction:column;gap:10px;height:100%;box-sizing:border-box;font-family:inherit;font-size:13px;line-height:1.5";
      const engine = document.createElement("div");
      let engineValue = "javascript";
      const expression = document.createElement("input");
      expression.placeholder = "正则，例如：\\b[A-Z]\\w+\\b";
      expression.style.cssText = fieldStyle;
      const flags = document.createElement("input");
      flags.value = "g";
      flags.placeholder = "标志，例如：gim";
      flags.style.cssText = fieldStyle;
      const help = document.createElement("div");
      help.style.cssText = "opacity:.72;font-size:12px";
      const source = document.createElement("textarea");
      source.placeholder = "输入待匹配文本";
      source.style.cssText = `${fieldStyle};flex:1;min-height:100px;resize:vertical`;
      const output = document.createElement("pre");
      output.style.cssText = "margin:0;min-height:30px;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;font:12px/1.55 ui-monospace,Consolas,monospace";

      let timer;
      let revision = 0;
      const update = () => {
        clearTimeout(timer);
        const current = ++revision;
        const pattern = expression.value;
        const text = source.value;
        const selectedEngine = engineValue;
        const selectedFlags = flags.value.replace(/\s/g, "");
        if (!pattern || !text) {
          output.textContent = "输入正则表达式和待测文本";
          return;
        }
        if (selectedEngine === "javascript") {
          try {
            const regex = new RegExp(pattern, selectedFlags);
            const matches = [];
            let truncated = false;
            for (const match of text.matchAll(regex.global ? regex : new RegExp(regex.source, `${regex.flags}g`))) {
              if (matches.length === 500) { truncated = true; break; }
              matches.push({match: match[0], index: match.index, groups: match.slice(1), named: match.groups});
            }
            output.textContent = formatMatches(matches, truncated);
          } catch (error) {
            output.textContent = `JavaScript 正则错误：${error.message}`;
          }
          return;
        }

        output.textContent = "正在用 Python re 匹配…";
        timer = setTimeout(async () => {
          try {
            const response = await context.api.runPython("find_matches", {pattern, text, flags: selectedFlags});
            if (current !== revision) return;
            if (!response?.ok) throw new Error(response?.error || "Python 正则执行失败");
            output.textContent = formatMatches(response.result?.matches || [], response.result?.truncated);
          } catch (error) {
            if (current === revision) output.textContent = `Python re 错误：${error.message || error}`;
          }
        }, 120);
      };

      const updateEngine = () => {
        if (engineValue === "python") {
          flags.value = flags.value.replace(/g/gi, "");
          flags.placeholder = "标志：i m s x a（全局匹配默认开启）";
          help.textContent = "Python re；支持忽略大小写 i、多行 m、点匹配换行 s、扩展空白 x、ASCII a。全局匹配始终开启。";
        } else {
          flags.placeholder = "标志：g i m s u y d";
          help.textContent = "JavaScript RegExp；支持 g 全局、i 忽略大小写、m 多行、s 点匹配换行等标志。";
        }
        update();
      };

      const unmountEngine = context.ui.mountControl(engine, "Select", {
        defaultValue: "javascript",
        style: {width: "100%"},
        "aria-label": "正则引擎",
        options: [
          {value: "javascript", label: "JavaScript · RegExp"},
          {value: "python", label: "Python · re"},
        ],
        onChange: selected => { engineValue = selected; updateEngine(); },
      });
      expression.oninput = update;
      flags.oninput = update;
      source.oninput = update;
      root.append(engine, expression, flags, help, source, output);
      container.replaceChildren(root);
      updateEngine();
      expression.focus();
      cleanup = () => {
        revision++;
        clearTimeout(timer);
        unmountEngine();
        for (const element of [engine, expression, flags, source]) {
          element.onchange = null;
          element.oninput = null;
        }
        root.remove();
      };
    },
    unmount() {
      cleanup?.();
      cleanup = null;
    },
  };
}
