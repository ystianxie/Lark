const css = "padding:18px;display:flex;flex-direction:column;gap:12px;height:100%;box-sizing:border-box;font-family:inherit;font-size:13px;line-height:1.6";
function button(label, action) { const el = document.createElement("button"); el.textContent = label; el.onclick = action; return el; }
export function activate() { let cleanup;
  return { mount(container) {
    const root = document.createElement("div"); root.style.cssText = css;
    const toolbar = document.createElement("div"); toolbar.style.cssText = "display:flex;flex-wrap:wrap;gap:8px";
    const input = document.createElement("textarea"); input.placeholder = "输入或粘贴文本，每行一条"; input.style.cssText = "flex:1;min-height:130px;resize:none;padding:12px;border:1px solid #8886;border-radius:8px;background:transparent;color:inherit;font-family:inherit;font-size:13px;line-height:1.6";
    const output = document.createElement("textarea"); output.readOnly = true; output.placeholder = "处理结果"; output.style.cssText = input.style.cssText;
    const transform = fn => { output.value = fn(input.value); };
    const unique = text => [...new Set(text.split(/\r?\n/).filter(Boolean))].join("\n");
    toolbar.append(button("去重", () => transform(unique)), button("排序", () => transform(text => text.split(/\r?\n/).sort((a, b) => a.localeCompare(b, "zh-Hans")).join("\n"))), button("合并空白", () => transform(text => text.replace(/\s+/g, " ").trim())), button("小写", () => transform(text => text.toLowerCase())), button("大写", () => transform(text => text.toUpperCase())), button("驼峰", () => transform(text => text.trim().toLowerCase().replace(/[-_\s]+(.)?/g, (_, c) => c ? c.toUpperCase() : ""))));
    const stats = document.createElement("div"); stats.style.opacity = "0.7";
    input.oninput = () => { const lines = input.value ? input.value.split(/\r?\n/).length : 0; stats.textContent = `${input.value.length} 个字符 · ${lines} 行`; };
    root.append(toolbar, input, stats, output); container.replaceChildren(root); input.focus(); cleanup = () => { input.oninput = null; root.remove(); };
  }, unmount() { cleanup?.(); cleanup = null; } };
}
