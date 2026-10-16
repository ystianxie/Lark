export function activate(context) {
  let cleanup;
  return {
    async mount(container) {
      cleanup?.();
      let disposed = false;
      let editor;
      const abort = new AbortController();
      const controls = [];
      const root = document.createElement("div");
      root.className = "larkPluginPanelRoot";
      root.style.cssText = "box-sizing:border-box;height:100%;min-height:340px;padding:14px;display:flex;flex-direction:column;gap:10px;font-family:inherit;font-size:13px";
      const title = document.createElement("strong"); title.textContent = "文本 / 代码差异比较";
      const toolbar = document.createElement("div"); toolbar.style.cssText = "display:flex;flex-wrap:wrap;gap:8px;align-items:center";
      const status = document.createElement("div"); status.style.cssText = "font-size:12px;opacity:.75"; status.setAttribute("role", "status");
      status.textContent = "正在加载编辑器…";
      const surface = document.createElement("div"); surface.style.cssText = "flex:1;min-height:240px;border:1px solid #8884;border-radius:8px;overflow:hidden";
      const copyBar = document.createElement("div");
      copyBar.style.cssText = "display:grid;grid-template-columns:1fr 1fr;gap:8px;align-items:center";
      const originalCopy = document.createElement("div");
      const modifiedCopy = document.createElement("div");
      originalCopy.style.cssText = "display:flex;justify-content:flex-start";
      modifiedCopy.style.cssText = "display:flex;justify-content:flex-end";
      copyBar.append(originalCopy, modifiedCopy);
      root.append(title, toolbar, status, copyBar, surface); container.replaceChildren(root);
      const mount = (parent, type, props) => {
        const host = document.createElement("div"); parent.append(host);
        const dispose = context.ui.mountControl(host, type, props); controls.push(dispose); return dispose;
      };
      const control = (type, props) => mount(toolbar, type, props);
      cleanup = () => { disposed = true; abort.abort(); editor?.dispose(); controls.forEach(dispose => dispose()); root.remove(); };
      try {
        editor = await context.ui.mountDiffEditor(surface, {
          original: "", modified: "", language: "plaintext", signal: abort.signal,
          onDiff: result => {
            if (disposed) return;
            status.textContent = result.quitEarly ? "比较超时，结果可能不完整；请缩小文本后重试。" : result.identical ? "两侧文本一致（按当前空白设置比较）" : `${result.changes} 处差异 · 左侧原文本，右侧新文本；两侧均可编辑`;
          },
        });
        if (disposed) { editor.dispose(); return; }
        control("Select", {
          defaultValue: "plaintext", style: {width: 130}, "aria-label": "文本语言",
          options: [["plaintext", "纯文本"], ["json", "JSON"], ["javascript", "JavaScript"], ["typescript", "TypeScript"], ["python", "Python"], ["sql", "SQL"], ["yaml", "YAML"]].map(([value, label]) => ({value, label})),
          onChange: value => editor.setLanguage(value),
        });
        control("Checkbox", {label: "忽略行首尾空白", onChange: event => editor.updateOptions({ignoreTrimWhitespace: event.target.checked})});
        control("Checkbox", {label: "并排显示", defaultChecked: true, onChange: event => editor.updateOptions({renderSideBySide: event.target.checked})});
        control("Button", {label: "上一处", onClick: () => editor.navigate(false)});
        control("Button", {label: "下一处", onClick: () => editor.navigate(true)});
        control("Button", {label: "交换", onClick: () => { const texts = editor.getTexts(); editor.setTexts(texts.modified, texts.original); }});
        const copy = which => async () => {
          try { await context.api.writeClipboard(editor.getTexts()[which]); if (!disposed) status.textContent = "已复制"; }
          catch (error) { if (!disposed) status.textContent = `复制失败：${error.message || error}`; }
        };
        mount(originalCopy, "Button", {type: "text", size: "small", icon: "copy", title: "复制原文", "aria-label": "复制原文", onClick: copy("original")});
        mount(modifiedCopy, "Button", {type: "text", size: "small", icon: "copy", title: "复制新文", "aria-label": "复制新文", onClick: copy("modified")});
        status.textContent = "左侧粘贴原文本，右侧粘贴新文本；支持直接编辑和代码高亮。";
      } catch (error) {
        if (!disposed) status.textContent = `编辑器加载失败：${error.message || error}`;
      }
    },
    unmount() { cleanup?.(); cleanup = null; },
  };
}
