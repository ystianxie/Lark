const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const BODY_TYPES = [
  { value: "none", label: "无请求体" },
  { value: "json", label: "JSON" },
  { value: "form", label: "表单（application/x-www-form-urlencoded）" },
  { value: "text", label: "纯文本" },
];
let nextRowId = 1;

function row(key = "", value = "", enabled = true) {
  return { id: `row-${Date.now()}-${nextRowId++}`, key, value, enabled };
}
function rows(value) {
  return Array.isArray(value) && value.length ? value.map(item => row(item?.key, item?.value, item?.enabled !== false)) : [row()];
}
function draft() {
  return {
    name: "",
    method: "GET",
    url: "",
    params: [row()],
    headers: [row()],
    cookies: [row()],
    bodyType: "none",
    body: "",
    bodyParams: [row()],
  };
}
function normalizeRequest(value) {
  const base = draft();
  return {
    ...base,
    ...(value && typeof value === "object" ? value : {}),
    params: rows(value?.params),
    headers: rows(value?.headers),
    cookies: rows(value?.cookies),
    bodyParams: rows(value?.bodyParams),
  };
}
function esc(value) {
  return String(value ?? "");
}
function setText(node, value) {
  node.textContent = esc(value);
}
function enabledRows(items) {
  return items.filter(item => item.enabled && item.key.trim());
}
function mount(context, parent, type, props, disposers) {
  const host = document.createElement("div");
  host.className = `apiRequesterControl apiRequesterControl-${type.toLowerCase()}`;
  parent.append(host);
  const dispose = context.ui.mountControl(host, type, props);
  disposers.push(() => { dispose?.(); host.remove(); });
  return { host, dispose };
}
function button(context, parent, label, onClick, disposers, props = {}) {
  return mount(context, parent, "Button", { ...props, label, onClick }, disposers);
}
function fieldLabel(text) {
  const label = document.createElement("div");
  label.className = "apiRequesterLabel";
  label.textContent = text;
  return label;
}
function parseImportedRequest(value) {
  const source = value?.request || value;
  const postmanUrl = typeof source?.url === "object" ? source.url.raw || "" : source?.url;
  return normalizeRequest({
    ...source,
    url: postmanUrl || "",
    method: source?.method || "GET",
    headers: Array.isArray(source?.header) ? source.header.map(item => ({ key: item.key, value: item.value, enabled: !item.disabled })) : source?.headers,
    params: Array.isArray(source?.url?.query) ? source.url.query.map(item => ({ key: item.key, value: item.value, enabled: !item.disabled })) : source?.params,
    bodyType: source?.body?.mode === "raw" ? "json" : source?.body?.mode === "urlencoded" ? "form" : source?.bodyType,
    body: source?.body?.raw || source?.bodyText || source?.body || "",
    bodyParams: source?.body?.urlencoded,
  });
}
function parseCurl(command) {
  const tokens = command.match(/(?:[^\s"']+|"[^"]*"|'[^']*')+/g) || [];
  const unquote = value => value?.replace(/^(['"])(.*)\1$/, "$2") || "";
  const request = draft();
  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token === "-X" || token === "--request") request.method = String(unquote(tokens[++index] || "GET")).toUpperCase();
    else if (token === "-H" || token === "--header") { const [key, ...rest] = unquote(tokens[++index] || "").split(":"); if (key) request.headers.push(row(key.trim(), rest.join(":").trim())); }
    else if (["-b", "--cookie"].includes(token)) { unquote(tokens[++index] || "").split(";").forEach(pair => { const [key, ...rest] = pair.split("="); if (key?.trim()) request.cookies.push(row(key.trim(), rest.join("=").trim())); }); }
    else if (["-d", "--data", "--data-raw", "--data-binary"].includes(token)) { request.body = unquote(tokens[++index] || ""); request.bodyType = request.body.trim().startsWith("{") ? "json" : "text"; if (request.method === "GET") request.method = "POST"; }
    else if (!token.startsWith("-") && /^https?:\/\//i.test(unquote(token))) request.url = unquote(token);
  }
  return request;
}function section(title, description) {
  const element = document.createElement("section");
  element.className = "apiRequesterSection";
  const heading = document.createElement("div");
  heading.className = "apiRequesterSectionHeading";
  heading.textContent = title;
  element.append(heading);
  if (description) {
    const hint = document.createElement("div");
    hint.className = "apiRequesterHint";
    hint.textContent = description;
    element.append(hint);
  }
  return element;
}

export function activate(context) {
  let cleanup = () => {};
  return {
    async mount(container) {
      cleanup();
      let disposed = false;
      const disposers = [];
      const state = { request: draft(), saved: [], response: null, loading: false };
      const rowRenders = {};
      let bodyRender = () => {};
      const root = document.createElement("div");
      cleanup = () => { disposed = true; disposers.splice(0).forEach(dispose => dispose()); root.remove(); };
      root.className = "apiRequesterRoot";
      root.innerHTML = `
        <style>
          .apiRequesterRoot { box-sizing:border-box; height:100%; overflow:auto; padding:16px; color:var(--text-primary); font-size:13px; }
          .apiRequesterRoot * { box-sizing:border-box; }
          .apiRequesterTop { display:flex; gap:8px; align-items:end; flex-wrap:wrap; }
          .apiRequesterTop > .apiRequesterField { min-width:160px; }
          .apiRequesterTop > .apiRequesterUrl { flex:1 1 360px; min-width:260px; }
          .apiRequesterField { display:flex; flex-direction:column; gap:5px; }
          .apiRequesterLabel,.apiRequesterSectionHeading { color:var(--text-primary); font-weight:600; }
          .apiRequesterSection { margin-top:14px; }
          .apiRequesterSectionHeading { margin-bottom:7px; }
          .apiRequesterHint { color:var(--text-secondary); font-size:12px; margin:3px 0 8px; }
          .apiRequesterRows { display:flex; flex-direction:column; gap:7px; }
          .apiRequesterTabs { display:flex; gap:4px; margin-top:14px; border-bottom:1px solid var(--launcher-border); }
          .apiRequesterTab { padding:7px 12px; color:var(--text-secondary); cursor:pointer; border-bottom:2px solid transparent; }
          .apiRequesterTab.active { color:var(--text-primary); border-bottom-color:var(--color-primary, #4f7df3); font-weight:600; }
          .apiRequesterRow { display:grid; grid-template-columns:28px minmax(120px,1fr) minmax(160px,1.4fr) auto; gap:7px; align-items:center; }
          .apiRequesterControl { min-width:0; }
          .apiRequesterRow .apiRequesterControl:first-of-type { width:100%; }
          .apiRequesterActions { display:flex; gap:8px; align-items:center; flex-wrap:wrap; margin-top:10px; }
          .apiRequesterSection > .apiRequesterControl { display:inline-flex; width:auto; margin-top:8px; }
          .apiRequesterImport { display:flex; gap:8px; align-items:center; margin-top:10px; }
          .apiRequesterBodyText { min-height:150px; }
          .apiRequesterResponse { margin-top:16px; border-top:1px solid var(--launcher-border); padding-top:14px; }
          .apiRequesterResponseMeta { display:flex; gap:12px; align-items:center; flex-wrap:wrap; margin-bottom:8px; }
          .apiRequesterStatus { font-weight:700; }
          .apiRequesterResponseBody { margin:0; padding:12px; min-height:100px; max-height:360px; overflow:auto; white-space:pre-wrap; word-break:break-word; border:1px solid var(--launcher-border); border-radius:6px; background:var(--surface-muted); color:var(--text-primary); font:12px/1.5 ui-monospace,SFMono-Regular,Consolas,monospace; }
          .apiRequesterSaved { margin-top:16px; border-top:1px solid var(--launcher-border); padding-top:14px; }
          .apiRequesterSavedItem { display:flex; gap:8px; align-items:center; padding:7px 0; border-bottom:1px solid var(--launcher-border); }
          .apiRequesterSavedItem > span { flex:1; min-width:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
          .apiRequesterMuted { color:var(--text-secondary); }
          @media (max-width:700px) { .apiRequesterRow { grid-template-columns:28px 1fr auto; } .apiRequesterRow .apiRequesterControl:nth-child(3) { grid-column:2 / -1; } }
        </style>`;
      container.append(root);

      const top = document.createElement("div");
      top.className = "apiRequesterTop";
      root.append(top);
      const methodField = document.createElement("div"); methodField.className = "apiRequesterField"; methodField.append(fieldLabel("Method")); top.append(methodField);
      const method = mount(context, methodField, "Select", { value: state.request.method, options: METHODS.map(value => ({ value, label: value })), onChange: value => { state.request.method = value; } }, disposers);
      const urlField = document.createElement("div"); urlField.className = "apiRequesterField apiRequesterUrl"; urlField.append(fieldLabel("URL")); top.append(urlField);
      const url = mount(context, urlField, "Input", { value: state.request.url, placeholder: "https://api.example.com/users", onChange: event => { state.request.url = event?.target?.value ?? ""; } }, disposers);
      const nameField = document.createElement("div"); nameField.className = "apiRequesterField"; nameField.append(fieldLabel("接口名称（保存时使用）")); top.append(nameField);
      const name = mount(context, nameField, "Input", { value: state.request.name, placeholder: "例如：获取用户", onChange: event => { state.request.name = event?.target?.value ?? ""; } }, disposers);
      const topActions = document.createElement("div"); topActions.className = "apiRequesterActions"; root.append(topActions);
      const status = document.createElement("span"); status.className = "apiRequesterMuted"; topActions.append(status);      const importBar = document.createElement("div"); importBar.className = "apiRequesterImport"; root.append(importBar);
      const jsonInput = document.createElement("input"); jsonInput.type = "file"; jsonInput.accept = ".json,application/json"; jsonInput.hidden = true; root.append(jsonInput);
      button(context, importBar, null, () => jsonInput.click(), disposers, { icon: "upload", title: "导入 JSON", "aria-label": "导入 JSON" });
      button(context, importBar, null, () => {
        const curl = window.prompt("粘贴 cURL 命令");
        if (!curl) return;
        try { state.request = parseCurl(curl); method.dispose.update({ value: state.request.method }); url.dispose.update({ value: state.request.url }); name.dispose.update({ value: state.request.name }); Object.values(rowRenders).forEach(render => render()); bodyType.dispose.update({ value: state.request.bodyType }); bodyRender(); updateStatus("已导入 cURL"); }
        catch (error) { updateStatus(`cURL 导入失败：${error.message || error}`, true); }
      }, disposers, { icon: "code", title: "导入 cURL", "aria-label": "导入 cURL" });
      jsonInput.addEventListener("change", async event => {
        const file = event.target.files?.[0]; if (!file) return;
        try { state.request = parseImportedRequest(JSON.parse(await file.text())); method.dispose.update({ value: state.request.method }); url.dispose.update({ value: state.request.url }); name.dispose.update({ value: state.request.name }); Object.values(rowRenders).forEach(render => render()); bodyType.dispose.update({ value: state.request.bodyType }); bodyRender(); updateStatus("已导入 JSON"); }
        catch (error) { updateStatus(`JSON 导入失败：${error.message || error}`, true); }
        finally { jsonInput.value = ""; }
      });

      function updateStatus(text, error = false) { status.textContent = text; status.style.color = error ? "var(--color-error, #ff4d4f)" : ""; }
      function renderKV(parent, title, key, addLabel, allowEnabled = true) {
        const block = section(title, addLabel);
        const rowsHost = document.createElement("div"); rowsHost.className = "apiRequesterRows"; block.append(rowsHost);
        const render = () => {
          rowsHost.replaceChildren();
          state.request[key].forEach(item => {
            const line = document.createElement("div"); line.className = "apiRequesterRow"; rowsHost.append(line);
            if (allowEnabled) mount(context, line, "Checkbox", { checked: item.enabled, "aria-label": "启用", onChange: event => { item.enabled = event?.target?.checked ?? false; } }, disposers);
            else line.append(document.createElement("span"));
            mount(context, line, "Input", { value: item.key, placeholder: "键", onChange: event => { item.key = event?.target?.value ?? ""; } }, disposers);
            mount(context, line, "Input", { value: item.value, placeholder: "值", onChange: event => { item.value = event?.target?.value ?? ""; } }, disposers);
            button(context, line, null, () => { state.request[key] = state.request[key].filter(rowItem => rowItem.id !== item.id); if (!state.request[key].length) state.request[key].push(row()); render(); }, disposers, { icon: "delete", title: "删除", "aria-label": "删除", danger: true, size: "small" });
          });
          button(context, block, null, () => { state.request[key].push(row()); render(); }, disposers, { icon: "plus", title: "添加一行", "aria-label": "添加一行", size: "small" });
        };
        render(); rowRenders[key] = render; root.append(block); return block;
      }

      const requestTabs = document.createElement("div"); requestTabs.className = "apiRequesterTabs";
      const requestTabContent = document.createElement("div"); requestTabContent.className = "apiRequesterTabContent";
      const tabDefinitions = [
        ["params", "Params", "URL 查询参数会追加到地址后。"],
        ["headers", "Headers", "空键会被忽略。"],
        ["cookies", "Cookies", "会尝试转换为 Cookie 请求头。"],
      ];
      const tabBlocks = {};
      tabDefinitions.forEach(([key, title, hint], index) => {
        const tab = document.createElement("span"); tab.className = `apiRequesterTab${index === 0 ? " active" : ""}`; tab.textContent = title; tab.setAttribute("role", "tab"); tab.onclick = () => { Object.keys(tabBlocks).forEach(name => { tabBlocks[name].style.display = name === key ? "block" : "none"; }); requestTabs.querySelectorAll(".apiRequesterTab").forEach(node => node.classList.toggle("active", node === tab)); };
        requestTabs.append(tab);
        tabBlocks[key] = renderKV(requestTabContent, title, key, hint, true);
        tabBlocks[key].style.display = index === 0 ? "block" : "none";
      });
      root.append(requestTabs, requestTabContent);

      const bodyBlock = section("Body", "GET / HEAD 通常不应携带请求体；JSON 会先校验格式。");
      const bodyType = mount(context, bodyBlock, "Select", { value: state.request.bodyType, options: BODY_TYPES, onChange: value => { state.request.bodyType = value; bodyRender(); } }, disposers);
      const bodyHost = document.createElement("div"); bodyHost.className = "apiRequesterBodyHost"; bodyBlock.append(bodyHost); root.append(bodyBlock);
      let bodyDispose = null;
      bodyRender = () => {
        bodyDispose?.(); bodyDispose = null; bodyHost.replaceChildren();
        if (state.request.bodyType === "form") {
          const rowsHost = document.createElement("div"); rowsHost.className = "apiRequesterRows"; bodyHost.append(rowsHost);
          const render = () => {
            rowsHost.replaceChildren();
            state.request.bodyParams.forEach(item => {
              const line = document.createElement("div"); line.className = "apiRequesterRow"; rowsHost.append(line);
              line.append(document.createElement("span"));
              mount(context, line, "Input", { value: item.key, placeholder: "字段名", onChange: event => { item.key = event?.target?.value ?? ""; } }, disposers);
              mount(context, line, "Input", { value: item.value, placeholder: "字段值", onChange: event => { item.value = event?.target?.value ?? ""; } }, disposers);
              button(context, line, null, () => { state.request.bodyParams = state.request.bodyParams.filter(x => x.id !== item.id); if (!state.request.bodyParams.length) state.request.bodyParams.push(row()); render(); }, disposers, { icon: "delete", title: "删除", "aria-label": "删除", danger: true, size: "small" });
            });
            button(context, bodyHost, null, () => { state.request.bodyParams.push(row()); render(); }, disposers, { icon: "plus", title: "添加一行", "aria-label": "添加一行", size: "small" });
          }; render(); bodyDispose = () => rowsHost.replaceChildren();
        } else if (state.request.bodyType !== "none") {
          const mounted = mount(context, bodyHost, "TextArea", { value: state.request.body, rows: 7, className: "apiRequesterBodyText", placeholder: state.request.bodyType === "json" ? '{\n  "name": "Lark"\n}' : "请输入请求体", onChange: event => { state.request.body = event?.target?.value ?? ""; } }, disposers);
          bodyDispose = () => mounted.dispose?.();
        }
      };
      bodyRender();

      const actions = document.createElement("div"); actions.className = "apiRequesterActions"; root.append(actions);
      const responseBlock = section("Response"); responseBlock.classList.add("apiRequesterResponse"); root.append(responseBlock);
      const responseMeta = document.createElement("div"); responseMeta.className = "apiRequesterResponseMeta"; responseBlock.append(responseMeta);
      const responseBody = document.createElement("pre"); responseBody.className = "apiRequesterResponseBody"; responseBody.textContent = "发送请求后显示响应"; responseBlock.append(responseBody);
      function renderResponse() {
        responseMeta.replaceChildren();
        if (!state.response) return;
        const statusNode = document.createElement("span"); statusNode.className = "apiRequesterStatus"; statusNode.textContent = `${state.response.status} ${state.response.statusText}`; responseMeta.append(statusNode);
        const timeNode = document.createElement("span"); timeNode.className = "apiRequesterMuted"; timeNode.textContent = `${state.response.elapsed} ms`; responseMeta.append(timeNode);
        const headersNode = document.createElement("span"); headersNode.className = "apiRequesterMuted"; headersNode.textContent = `${state.response.headers.length} 个响应头`; responseMeta.append(headersNode);
        responseBody.textContent = state.response.body;
      }
      async function send() {
        if (state.loading) return;
        const rawUrl = state.request.url.trim();
        if (!/^https?:\/\//i.test(rawUrl)) { updateStatus("请输入完整的 http(s) URL", true); return; }
        let target;
        try { target = new URL(rawUrl); } catch { updateStatus("URL 格式无效", true); return; }
        if (!["http:", "https:"].includes(target.protocol)) { updateStatus("仅支持 HTTP / HTTPS", true); return; }
        const args = {
          method: state.request.method,
          url: state.request.url,
          params: state.request.params,
          headers: state.request.headers,
          cookies: state.request.cookies,
          bodyType: state.request.bodyType,
          body: state.request.body,
          bodyParams: state.request.bodyParams,
        };
        state.loading = true; updateStatus("Python 请求中…"); sendButton.dispose.update({ loading: true, disabled: true });
        try {
          const result = await context.api.runPython("request", args);
          if (disposed) return;
          if (!result?.ok) throw new Error(result?.error || "Python 请求失败");
          const data = result.result || {};
          state.response = {
            status: data.status,
            statusText: data.statusText || "",
            elapsed: data.elapsedMs ?? 0,
            headers: Array.isArray(data.headers) ? data.headers : [],
            body: data.body ?? "（空响应）",
          };
          renderResponse(); updateStatus("请求完成");
        } catch (error) {
          if (disposed) return;
          state.response = null; responseMeta.replaceChildren();
          responseBody.textContent = `请求失败：${error?.message || error}`;
          updateStatus("请求失败", true);
        } finally {
          state.loading = false;
          if (!disposed) sendButton.dispose.update({ loading: false, disabled: false });
        }
      }
      const sendButton = button(context, actions, null, send, disposers, { icon: "send", title: "发送请求", "aria-label": "发送请求", type: "primary" });
      button(context, actions, null, saveRequest, disposers, { icon: "save", title: "保存接口", "aria-label": "保存接口" });

      const savedBlock = section("已保存接口", "保存数据使用插件配置存储，仅保存在本机。"); savedBlock.classList.add("apiRequesterSaved"); const savedList = document.createElement("div"); savedBlock.append(savedList); root.append(savedBlock);
      function renderSaved() {
        savedList.replaceChildren();
        if (!state.saved.length) { const empty = document.createElement("div"); empty.className = "apiRequesterMuted"; empty.textContent = "暂无保存的接口"; savedList.append(empty); return; }
        state.saved.forEach(item => {
          const line = document.createElement("div"); line.className = "apiRequesterSavedItem"; savedList.append(line);
          const title = document.createElement("span"); title.textContent = `${item.name} · ${item.method} ${item.url}`; title.title = title.textContent; line.append(title);
          button(context, line, null, () => { state.request = normalizeRequest(item); method.dispose.update({ value: state.request.method }); url.dispose.update({ value: state.request.url }); name.dispose.update({ value: state.request.name }); bodyType.dispose.update({ value: state.request.bodyType }); updateStatus(`已加载：${item.name}`); }, disposers, { icon: "load", title: "加载", "aria-label": "加载", size: "small" });
          button(context, line, null, async () => { state.saved = state.saved.filter(x => x.id !== item.id); await context.api.setConfig({ savedRequests: state.saved }); renderSaved(); }, disposers, { icon: "delete", title: "删除", "aria-label": "删除", danger: true, size: "small" });
        });
      }
      async function saveRequest() {
        const request = normalizeRequest(state.request);
        if (!request.name.trim()) { updateStatus("请先填写接口名称", true); return; }
        if (!request.url.trim()) { updateStatus("请先填写 URL", true); return; }
        const saved = { ...request, id: state.request.id || `request-${Date.now()}` };
        const index = state.saved.findIndex(item => item.id === saved.id);
        if (index >= 0) state.saved[index] = saved; else state.saved.unshift(saved);
        state.request = saved; await context.api.setConfig({ savedRequests: state.saved }); renderSaved(); updateStatus("已保存");
      }
      try {
        const config = await context.api.getConfig();
        if (!disposed && config && typeof config === "object") { state.saved = Array.isArray(config.savedRequests) ? config.savedRequests.map(normalizeRequest) : []; renderSaved(); }
      } catch (error) { updateStatus(`读取保存接口失败：${error?.message || error}`, true); }
    },
    unmount() { cleanup(); cleanup = () => {}; },
  };
}



