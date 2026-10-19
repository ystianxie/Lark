const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const BODY_TYPES = [
    {value: "none", label: "无请求体"},
    {value: "json", label: "JSON"},
    {value: "form", label: "表单（application/x-www-form-urlencoded）"},
    {value: "text", label: "纯文本"},
];
let nextRowId = 1;

function row(key = "", value = "", enabled = true) {
    return {id: `row-${Date.now()}-${nextRowId++}`, key, value, enabled};
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

function selectValue(value) {
    return typeof value === "string" ? value : value?.target?.value ?? value?.value ?? "";
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
    disposers.push(() => {
        dispose?.();
        host.remove();
    });
    return {host, dispose};
}

function button(context, parent, label, onClick, disposers, props = {}) {
    return mount(context, parent, "Button", {...props, label, onClick}, disposers);
}

function fieldLabel(text) {
    const label = document.createElement("div");
    label.className = "apiRequesterLabel";
    label.textContent = text;
    return label;
}

function importedSource(value) {
    if (Array.isArray(value)) {
        const request = value.find(item => item && typeof item === "object" && (item.request || item.url || item.method));
        return request ? importedSource(request) : value;
    }
    if (!value || typeof value !== "object") return {};
    if (value.request && typeof value.request === "object") return importedSource(value.request);
    if (value.data && typeof value.data === "object") return importedSource(value.data);
    if (Array.isArray(value.item)) {
        const nested = value.item.flatMap(item => Array.isArray(item?.item) ? item.item : [item]).find(item => item?.request);
        if (nested?.request) return importedSource(nested.request);
    }
    return value;
}

function parseImportedRequest(value) {
    const source = importedSource(value);
    const rawUrl = typeof source?.url === "object" ? source.url.raw || "" : source?.url;
    let url = String(rawUrl || "");
    const queryFromUrl = (() => {
        try {
            const parsed = new URL(url);
            const pairs = [...parsed.searchParams.entries()].map(([key, value]) => ({key, value, enabled: true}));
            parsed.search = "";
            url = parsed.toString();
            return pairs;
        } catch {
            return [];
        }
    })();
    const toRows = value => {
        if (typeof value === "string") return value.split(/[\r\n;]+/).map(item => {
            const splitAt = item.indexOf(":") >= 0 ? item.indexOf(":") : item.indexOf("=");
            return splitAt > 0 ? {
                key: item.slice(0, splitAt).trim(),
                value: item.slice(splitAt + 1).trim(),
                enabled: true
            } : null;
        }).filter(Boolean);
        if (Array.isArray(value)) return value.map(item => typeof item === "string"
            ? toRows(item)[0] || {key: item, value: "", enabled: true}
            : ({
                key: item?.key ?? item?.name ?? "",
                value: item?.value ?? "",
                enabled: !item?.disabled && item?.enabled !== false
            }));
        if (value && typeof value === "object") return Object.entries(value).map(([key, item]) => ({
            key,
            value: String(item ?? ""),
            enabled: true
        }));
        return [];
    };
    const hasRequestFields = source?.url || source?.method || source?.headers || source?.header || source?.params || source?.query || source?.cookies || source?.cookie || source?.body || source?.bodyType;
    const explicitParams = toRows(source?.params || source?.query || source?.queryParams || source?.requestParams || source?.url?.query || (!hasRequestFields ? source : null));
    const headers = toRows(source?.headers || source?.header || source?.requestHeaders);
    const cookieHeader = headers.find(item => item.key.toLowerCase() === "cookie");
    let cookies = toRows(source?.cookies || source?.cookie || source?.requestCookies);
    if (!cookies.length && cookieHeader) {
        cookies = cookieHeader.value.split(";").map(part => {
            const splitAt = part.indexOf("=");
            return splitAt > 0 ? {
                key: part.slice(0, splitAt).trim(),
                value: part.slice(splitAt + 1).trim(),
                enabled: true
            } : null;
        }).filter(Boolean);
    }
    const bodySource = source?.body;
    const bodyRaw = bodySource?.raw ?? source?.bodyText ?? (typeof bodySource === "string" ? bodySource : "");
    const mode = bodySource?.mode;
    return normalizeRequest({
        ...source,
        url,
        method: source?.method || "GET",
        headers: headers.filter(item => item !== cookieHeader),
        cookies,
        params: explicitParams.length ? explicitParams : queryFromUrl,
        bodyType: source?.bodyType || (mode === "urlencoded" ? "form" : mode === "raw" && /json/i.test(headers.find(item => item.key.toLowerCase() === "content-type")?.value || bodyRaw) ? "json" : bodyRaw ? "text" : "none"),
        body: bodyRaw,
        bodyParams: toRows(bodySource?.urlencoded || source?.bodyParams),
    });
}

function parseCurl(command) {
    const tokens = [];
    let token = "", quote = "", escaped = false;
    for (const char of String(command)) {
        if (escaped) {
            token += char;
            escaped = false;
        } else if (char === "\\" && quote !== "'") escaped = true;
        else if (quote) {
            if (char === quote) quote = ""; else token += char;
        } else if (char === "'" || char === '"') quote = char;
        else if (/\s/.test(char)) {
            if (token) {
                tokens.push(token);
                token = "";
            }
        } else token += char;
    }
    if (escaped) token += "\\";
    if (token) tokens.push(token);
    if (quote) throw new Error("cURL 引号未闭合");
    const request = draft();
    request.params = [];
    request.headers = [];
    request.cookies = [];
    const take = index => tokens[index + 1] || "";
    for (let index = 0; index < tokens.length; index += 1) {
        const current = tokens[index];
        if (["-X", "--request"].includes(current)) request.method = take(index++).toUpperCase();
        else if (["-H", "--header"].includes(current)) {
            const raw = take(index++);
            const splitAt = raw.indexOf(":");
            if (splitAt > 0) request.headers.push(row(raw.slice(0, splitAt).trim(), raw.slice(splitAt + 1).trim()));
        } else if (["-b", "--cookie"].includes(current)) {
            take(index++).split(";").forEach(pair => {
                const splitAt = pair.indexOf("=");
                if (splitAt > 0) request.cookies.push(row(pair.slice(0, splitAt).trim(), pair.slice(splitAt + 1).trim()));
            });
        } else if (["-d", "--data", "--data-raw", "--data-binary", "--data-urlencode"].includes(current)) {
            request.body = take(index++);
            request.bodyType = request.body.trim().startsWith("{") ? "json" : "text";
            if (request.method === "GET") request.method = "POST";
        } else if (current === "--url") request.url = take(index++);
        else if (current.startsWith("--url=")) request.url = current.slice(6);
        else if (!current.startsWith("-") && /^https?:\/\//i.test(current)) request.url = current;
    }
    const contentType = request.headers.find(item => item.key.toLowerCase() === "content-type")?.value || "";
    if (request.body && /application\/x-www-form-urlencoded/i.test(contentType)) {
        request.bodyType = "form";
        request.bodyParams = [...new URLSearchParams(request.body).entries()].map(([key, value]) => row(key, value));
    }
    const cookieHeader = request.headers.find(item => item.key.toLowerCase() === "cookie");
    if (cookieHeader) {
        request.cookies.push(...cookieHeader.value.split(";").map(part => {
            const splitAt = part.indexOf("=");
            return splitAt > 0 ? row(part.slice(0, splitAt).trim(), part.slice(splitAt + 1).trim()) : null;
        }).filter(Boolean));
        request.headers = request.headers.filter(item => item !== cookieHeader);
    }
    const parsed = parseImportedRequest({url: request.url});
    request.params = parsed.params;
    if (!request.params.length) request.params = [row()];
    if (!request.headers.length) request.headers = [row()];
    if (!request.cookies.length) request.cookies = [row()];
    return request;
}

function section(title, description) {
    const element = document.createElement("section");
    element.className = "apiRequesterSection";
    if (title) {
        const heading = document.createElement("div");
        heading.className = "apiRequesterSectionHeading";
        heading.textContent = title;
        element.append(heading);
    }
    if (description) {
        const hint = document.createElement("div");
        hint.className = "apiRequesterHint";
        hint.textContent = description;
        element.append(hint);
    }
    return element;
}

export function activate(context) {
    let cleanup = () => {
    };
    return {
        async mount(container) {
            cleanup();
            let disposed = false;
            const disposers = [];
            const state = {request: draft(), saved: [], response: null, loading: false};
            const rowRenders = {};
            let bodyRender = () => {
            };
            const root = document.createElement("div");
            cleanup = () => {
                disposed = true;
                disposers.splice(0).forEach(dispose => dispose());
                root.remove();
            };
            root.className = "apiRequesterRoot";
            root.innerHTML = `
        <style>
          .apiRequesterRoot { box-sizing:border-box; height:100%; overflow:auto; padding:16px; color:var(--text-primary); font-size:13px; }
          .apiRequesterRoot * { box-sizing:border-box; }
          .apiRequesterTop { display:grid; grid-template-columns:88px minmax(180px,420px) auto; gap:8px; align-items:end; }
          .apiRequesterTop > .apiRequesterField { min-width:0; }
          .apiRequesterTop > .apiRequesterUrl { min-width:0; }
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
          .apiRequesterTopActions { display:flex; gap:6px; align-items:center; }
          .apiRequesterTopActions .apiRequesterControl { display:inline-flex; }
          .apiRequesterTopActions .apiRequesterControl .ant-btn { width:32px; height:32px; padding:0; }
          .apiRequesterSectionTools { display:flex; gap:6px; align-items:center; margin-top:8px; }
          .apiRequesterSectionTools .apiRequesterControl { display:inline-flex; }
          .apiRequesterBodyText { min-height:150px; }
          .apiRequesterResponse { margin-top:16px; border-top:1px solid var(--launcher-border); padding-top:14px; }
          .apiRequesterResponseUrl { margin:0 0 10px; color:var(--text-secondary); overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
          .apiRequesterResponseMeta { display:flex; gap:12px; align-items:center; flex-wrap:wrap; margin-bottom:8px; }
          .apiRequesterResponseTabs { display:flex; gap:4px; border-bottom:1px solid var(--launcher-border); }
          .apiRequesterResponseTab { padding:7px 12px; color:var(--text-secondary); cursor:pointer; border-bottom:2px solid transparent; }
          .apiRequesterResponseTab.active { color:var(--text-primary); border-bottom-color:var(--color-primary, #4f7df3); font-weight:600; }
          .apiRequesterResponsePanel { margin-top:10px; }
          .apiRequesterResponsePairs { margin:0; padding:10px 12px; min-height:72px; max-height:260px; overflow:auto; white-space:pre-wrap; word-break:break-word; border:1px solid var(--launcher-border); border-radius:6px; background:var(--surface-muted); color:var(--text-primary); font:12px/1.5 ui-monospace,SFMono-Regular,Consolas,monospace; }
          .apiRequesterStatus { display:inline-flex; align-items:center; gap:5px; font-weight:700; }
          .apiRequesterStatusDot { width:7px; height:7px; border-radius:50%; background:var(--text-secondary); }
          .apiRequesterStatus-success .apiRequesterStatusDot { background:#22a06b; }
          .apiRequesterStatus-error .apiRequesterStatusDot { background:#d64545; }
          .apiRequesterStatus-warning .apiRequesterStatusDot { background:#d99a18; }
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
            const methodField = document.createElement("div");
            methodField.className = "apiRequesterField";
            methodField.append(fieldLabel("Method"));
            top.append(methodField);
            const method = mount(context, methodField, "Select", {
                value: state.request.method,
                options: METHODS.map(value => ({value, label: value})),
                onChange: value => {
                    state.request.method = selectValue(value);
                }
            }, disposers);
            const urlField = document.createElement("div");
            urlField.className = "apiRequesterField apiRequesterUrl";
            urlField.append(fieldLabel("URL"));
            top.append(urlField);
            const url = mount(context, urlField, "Input", {
                value: state.request.url,
                placeholder: "https://api.example.com/users",
                onChange: event => {
                    state.request.url = event?.target?.value ?? "";
                }
            }, disposers);
            const topActions = document.createElement("div");
            topActions.className = "apiRequesterTopActions";
            top.append(topActions);
            const status = document.createElement("span");
            status.className = "apiRequesterMuted";
            root.append(status);
            const sendButton = button(context, topActions, null, send, disposers, {
                icon: "send",
                title: "发送请求",
                "aria-label": "发送请求",
                type: "primary"
            });
            button(context, topActions, null, () => {
                const curl = window.prompt("粘贴 cURL 命令");
                if (!curl) return;
                try {
                    applyImportedRequest(parseCurl(curl));
                    updateStatus("已导入 cURL");
                } catch (error) {
                    updateStatus(`cURL 导入失败：${error.message || error}`, true);
                }
            }, disposers, {icon: "code", title: "导入 cURL", "aria-label": "导入 cURL"});
            button(context, topActions, null, saveRequest, disposers, {
                icon: "save",
                title: "保存接口",
                "aria-label": "保存接口"
            });

            function applyImportedRequest(next) {
                state.request = normalizeRequest(next);
                method.dispose.update({value: state.request.method});
                url.dispose.update({value: state.request.url});
                bodyType.dispose.update({value: state.request.bodyType});
                Object.values(rowRenders).forEach(render => render());
                bodyRender();
            }

            function updateStatus(text, error = false) {
                status.textContent = text;
                status.style.color = error ? "var(--color-error, #ff4d4f)" : "";
            }

            function renderKV(parent, title, key, allowEnabled = true) {
                const block = section("");
                const rowsHost = document.createElement("div");
                rowsHost.className = "apiRequesterRows";
                block.append(rowsHost);
                const renderDisposers = [];
                disposers.push(() => renderDisposers.splice(0).forEach(dispose => dispose?.()));
                const render = () => {
                    renderDisposers.splice(0).forEach(dispose => dispose?.());
                    rowsHost.replaceChildren();
                    state.request[key].forEach(item => {
                        const line = document.createElement("div");
                        line.className = "apiRequesterRow";
                        rowsHost.append(line);
                        if (allowEnabled) mount(context, line, "Checkbox", {
                            checked: item.enabled,
                            "aria-label": "启用",
                            onChange: event => {
                                item.enabled = event?.target?.checked ?? false;
                            }
                        }, renderDisposers);
                        else line.append(document.createElement("span"));
                        mount(context, line, "Input", {
                            value: item.key, placeholder: "键", onChange: event => {
                                item.key = event?.target?.value ?? "";
                            }
                        }, renderDisposers);
                        mount(context, line, "Input", {
                            value: item.value, placeholder: "值", onChange: event => {
                                item.value = event?.target?.value ?? "";
                            }
                        }, renderDisposers);
                        button(context, line, null, () => {
                            state.request[key] = state.request[key].filter(rowItem => rowItem.id !== item.id);
                            if (!state.request[key].length) state.request[key].push(row());
                            render();
                        }, renderDisposers, {
                            icon: "delete",
                            title: "删除",
                            "aria-label": "删除",
                            danger: true,
                            size: "small"
                        });
                    });
                };
                const tools = document.createElement("div");
                tools.className = "apiRequesterSectionTools";
                block.append(tools);
                button(context, tools, null, () => {
                    state.request[key].push(row());
                    render();
                }, disposers, {icon: "plus", title: "添加一行", "aria-label": "添加一行", size: "small"});
                button(context, tools, null, () => {
                    const text = window.prompt("粘贴 JSON 文本（可包含 params、headers、cookies）");
                    if (!text) return;
                    try {
                        const raw = JSON.parse(text);
                        const parsed = parseImportedRequest(raw);
                        const candidate = raw?.[key] ?? raw;
                        const direct = Array.isArray(candidate)
                            ? candidate.map(item => typeof item === "string" ? {
                                key: item,
                                value: "",
                                enabled: true
                            } : ({
                                key: item?.key ?? item?.name ?? "",
                                value: item?.value ?? "",
                                enabled: !item?.disabled && item?.enabled !== false
                            }))
                            : candidate && typeof candidate === "object" && !candidate.url && !candidate.request
                                ? Object.entries(candidate).map(([name, value]) => ({
                                    key: name,
                                    value: String(value ?? ""),
                                    enabled: true
                                }))
                                : [];
                        const imported = direct.length ? direct.map(item => row(item.key, item.value, item.enabled)) : parsed[key];
                        if (!Array.isArray(imported) || !imported.length) throw new Error(`JSON 中没有可导入的 ${title} 数据`);
                        state.request[key] = imported;
                        render();
                        updateStatus(`已导入 ${title}`);
                    } catch (error) {
                        updateStatus(`JSON 导入失败：${error.message || error}`, true);
                    }
                }, disposers, {
                    icon: "upload",
                    title: `导入${title} JSON`,
                    "aria-label": `导入${title} JSON`,
                    size: "small"
                });
                render();
                rowRenders[key] = render;
                parent.append(block);
                return block;
            }

            const requestTabs = document.createElement("div");
            requestTabs.className = "apiRequesterTabs";
            const requestTabContent = document.createElement("div");
            requestTabContent.className = "apiRequesterTabContent";
            const tabDefinitions = [
                ["params", "Params"],
                ["headers", "Headers"],
                ["cookies", "Cookies"],
            ];
            const tabBlocks = {};
            tabDefinitions.forEach(([key, title], index) => {
                const tab = document.createElement("span");
                tab.className = `apiRequesterTab${index === 0 ? " active" : ""}`;
                tab.textContent = title;
                tab.setAttribute("role", "tab");
                tab.onclick = () => {
                    Object.keys(tabBlocks).forEach(name => {
                        tabBlocks[name].style.display = name === key ? "block" : "none";
                    });
                    requestTabs.querySelectorAll(".apiRequesterTab").forEach(node => node.classList.toggle("active", node === tab));
                };
                requestTabs.append(tab);
                tabBlocks[key] = renderKV(requestTabContent, title, key, true);
                tabBlocks[key].style.display = index === 0 ? "block" : "none";
            });
            root.append(requestTabs, requestTabContent);

            const bodyBlock = section("Body", "GET / HEAD 通常不应携带请求体；JSON 会先校验格式。");
            const bodyType = mount(context, bodyBlock, "Select", {
                value: state.request.bodyType,
                options: BODY_TYPES,
                onChange: value => {
                    state.request.bodyType = selectValue(value);
                    bodyRender();
                }
            }, disposers);
            const bodyHost = document.createElement("div");
            bodyHost.className = "apiRequesterBodyHost";
            bodyBlock.append(bodyHost);
            root.append(bodyBlock);
            let bodyDispose = null;
            const bodyDisposers = [];
            disposers.push(() => bodyDisposers.splice(0).forEach(dispose => dispose?.()));
            bodyRender = () => {
                bodyDisposers.splice(0).forEach(dispose => dispose?.());
                bodyDispose?.();
                bodyDispose = null;
                bodyHost.replaceChildren();
                if (state.request.bodyType === "form") {
                    const rowsHost = document.createElement("div");
                    rowsHost.className = "apiRequesterRows";
                    bodyHost.append(rowsHost);
                    const render = () => {
                        rowsHost.replaceChildren();
                        state.request.bodyParams.forEach(item => {
                            const line = document.createElement("div");
                            line.className = "apiRequesterRow";
                            rowsHost.append(line);
                            line.append(document.createElement("span"));
                            mount(context, line, "Input", {
                                value: item.key, placeholder: "字段名", onChange: event => {
                                    item.key = event?.target?.value ?? "";
                                }
                            }, bodyDisposers);
                            mount(context, line, "Input", {
                                value: item.value,
                                placeholder: "字段值",
                                onChange: event => {
                                    item.value = event?.target?.value ?? "";
                                }
                            }, bodyDisposers);
                            button(context, line, null, () => {
                                state.request.bodyParams = state.request.bodyParams.filter(x => x.id !== item.id);
                                if (!state.request.bodyParams.length) state.request.bodyParams.push(row());
                                render();
                            }, bodyDisposers, {
                                icon: "delete",
                                title: "删除",
                                "aria-label": "删除",
                                danger: true,
                                size: "small"
                            });
                        });
                    };
                    render();
                    bodyDispose = () => rowsHost.replaceChildren();
                    button(context, bodyHost, null, () => {
                        state.request.bodyParams.push(row());
                        render();
                    }, bodyDisposers, {icon: "plus", title: "添加一行", "aria-label": "添加一行", size: "small"});
                } else if (state.request.bodyType !== "none") {
                    const mounted = mount(context, bodyHost, "TextArea", {
                        value: state.request.body,
                        rows: 7,
                        className: "apiRequesterBodyText",
                        placeholder: state.request.bodyType === "json" ? '{\n  "name": "Lark"\n}' : "请输入请求体",
                        onChange: event => {
                            state.request.body = event?.target?.value ?? "";
                        }
                    }, bodyDisposers);
                    bodyDispose = () => mounted.dispose?.();
                }
            };
            bodyRender();

            const responseBlock = section("Response");
            responseBlock.classList.add("apiRequesterResponse");
            root.append(responseBlock);
            const responseUrl = document.createElement("div");
            responseUrl.className = "apiRequesterResponseUrl";
            responseUrl.textContent = "请求完成后显示响应 URL";
            responseBlock.append(responseUrl);
            const responseMeta = document.createElement("div");
            responseMeta.className = "apiRequesterResponseMeta";
            responseBlock.append(responseMeta);
            const responseTabs = document.createElement("div");
            responseTabs.className = "apiRequesterResponseTabs";
            responseBlock.append(responseTabs);
            const responsePanelsHost = document.createElement("div");
            responseBlock.append(responsePanelsHost);
            const responsePanels = {};
            ["body", "headers", "cookies"].forEach((key, index) => {
                const tab = document.createElement("span");
                tab.className = `apiRequesterResponseTab${index === 0 ? " active" : ""}`;
                tab.textContent = {body: "Body", headers: "Headers", cookies: "Set-Cookie"}[key];
                tab.onclick = () => {
                    Object.entries(responsePanels).forEach(([name, panel]) => {
                        panel.style.display = name === key ? "block" : "none";
                    });
                    responseTabs.querySelectorAll(".apiRequesterResponseTab").forEach(node => node.classList.toggle("active", node === tab));
                };
                responseTabs.append(tab);
                const panel = document.createElement(key === "body" ? "pre" : "pre");
                panel.className = key === "body" ? "apiRequesterResponseBody apiRequesterResponsePanel" : "apiRequesterResponsePairs apiRequesterResponsePanel";
                panel.textContent = key === "body" ? "发送请求后显示响应" : "暂无数据";
                panel.style.display = index === 0 ? "block" : "none";
                responsePanelsHost.append(panel);
                responsePanels[key] = panel;
            });

            function renderResponse() {
                responseMeta.replaceChildren();
                if (!state.response) {
                    responseUrl.textContent = "请求完成后显示响应 URL";
                    return;
                }
                const statusNode = document.createElement("span");
                const statusClass = state.response.status >= 200 && state.response.status < 300
                    ? "success"
                    : state.response.status >= 400 && state.response.status < 600
                        ? "error"
                        : "warning";
                statusNode.className = `apiRequesterStatus apiRequesterStatus-${statusClass}`;
                const statusDot = document.createElement("span");
                statusDot.className = "apiRequesterStatusDot";
                statusDot.setAttribute("aria-hidden", "true");
                statusNode.append(statusDot, document.createTextNode(`${state.response.status} ${state.response.statusText}`));
                responseMeta.append(statusNode);
                const timeNode = document.createElement("span");
                timeNode.className = "apiRequesterMuted";
                timeNode.textContent = `${state.response.elapsed} ms`;
                responseMeta.append(timeNode);
                const headersNode = document.createElement("span");
                headersNode.className = "apiRequesterMuted";
                headersNode.textContent = `${state.response.headers.length} 个响应头`;
                responseMeta.append(headersNode);
                responseUrl.textContent = `响应 URL：${state.response.responseUrl}`;
                responseUrl.title = state.response.responseUrl;
                responsePanels.body.textContent = state.response.body;
                responsePanels.headers.textContent = state.response.headers.filter(([key]) => key.toLowerCase() !== "set-cookie").map(([key, value]) => `${key}: ${value}`).join("\n") || "暂无响应头";
                responsePanels.cookies.textContent = state.response.cookies.map(item => `${item.key}: ${item.value}`).join("\n") || "暂无 Set-Cookie";
            }

            async function send() {
                if (state.loading) return;
                const rawUrl = state.request.url.trim();
                if (!/^https?:\/\//i.test(rawUrl)) {
                    updateStatus("请输入完整的 http(s) URL", true);
                    return;
                }
                let target;
                try {
                    target = new URL(rawUrl);
                } catch {
                    updateStatus("URL 格式无效", true);
                    return;
                }
                if (!["http:", "https:"].includes(target.protocol)) {
                    updateStatus("仅支持 HTTP / HTTPS", true);
                    return;
                }
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
                state.loading = true;
                updateStatus("Python 请求中…");
                sendButton.dispose.update({loading: true, disabled: true});
                try {
                    const result = await context.api.runPython("request", args);
                    if (disposed) return;
                    if (!result?.ok) throw new Error(result?.error || "Python 请求失败");
                    const data = result.result || {};
                    state.response = {
                        responseUrl: data.responseUrl || state.request.url,
                        status: data.status,
                        statusText: data.statusText || "",
                        elapsed: data.elapsedMs ?? 0,
                        headers: Array.isArray(data.headers) ? data.headers : [],
                        cookies: (Array.isArray(data.headers) ? data.headers : []).filter(([key]) => String(key).toLowerCase() === "set-cookie").map(([, value]) => {
                            const splitAt = String(value).indexOf("=");
                            return splitAt > 0 ? {
                                key: String(value).slice(0, splitAt),
                                value: String(value).slice(splitAt + 1).split(";")[0]
                            } : null;
                        }).filter(Boolean),
                        body: data.body ?? "（空响应）",
                    };
                    renderResponse();
                    updateStatus("请求完成");
                } catch (error) {
                    if (disposed) return;
                    state.response = null;
                    responseMeta.replaceChildren();
                    responseUrl.textContent = "请求失败";
                    responsePanels.body.textContent = `请求失败：${error?.message || error}`;
                    updateStatus("请求失败", true);
                } finally {
                    state.loading = false;
                    if (!disposed) sendButton.dispose.update({loading: false, disabled: false});
                }
            }

            const savedBlock = section("已保存接口", "保存数据使用插件配置存储，仅保存在本机。");
            savedBlock.classList.add("apiRequesterSaved");
            const savedList = document.createElement("div");
            savedBlock.append(savedList);
            root.append(savedBlock);

            function renderSaved() {
                savedList.replaceChildren();
                if (!state.saved.length) {
                    const empty = document.createElement("div");
                    empty.className = "apiRequesterMuted";
                    empty.textContent = "暂无保存的接口";
                    savedList.append(empty);
                    return;
                }
                state.saved.forEach(item => {
                    const line = document.createElement("div");
                    line.className = "apiRequesterSavedItem";
                    savedList.append(line);
                    const title = document.createElement("span");
                    title.textContent = `${item.name} · ${item.method} ${item.url}`;
                    title.title = title.textContent;
                    line.append(title);
                    button(context, line, null, () => {
                        applyImportedRequest(item);
                        updateStatus(`已加载：${item.name}`);
                    }, disposers, {icon: "load", title: "加载", "aria-label": "加载", size: "small"});
                    button(context, line, null, async () => {
                        state.saved = state.saved.filter(x => x.id !== item.id);
                        await context.api.setConfig({savedRequests: state.saved});
                        renderSaved();
                    }, disposers, {icon: "delete", title: "删除", "aria-label": "删除", danger: true, size: "small"});
                });
            }

            async function saveRequest() {
                const request = normalizeRequest(state.request);
                if (!request.url.trim()) {
                    updateStatus("请先填写 URL", true);
                    return;
                }
                const name = window.prompt("请输入接口名称", request.name.trim());
                if (name === null) return;
                request.name = name.trim();
                if (!request.name) {
                    updateStatus("接口名称不能为空", true);
                    return;
                }
                const saved = {...request, id: state.request.id || `request-${Date.now()}`};
                const index = state.saved.findIndex(item => item.id === saved.id);
                if (index >= 0) state.saved[index] = saved; else state.saved.unshift(saved);
                state.request = saved;
                await context.api.setConfig({savedRequests: state.saved});
                renderSaved();
                updateStatus("已保存");
            }

            try {
                const config = await context.api.getConfig();
                if (!disposed && config && typeof config === "object") {
                    state.saved = Array.isArray(config.savedRequests) ? config.savedRequests.map(normalizeRequest) : [];
                    renderSaved();
                }
            } catch (error) {
                updateStatus(`读取保存接口失败：${error?.message || error}`, true);
            }
        },
        unmount() {
            cleanup();
            cleanup = () => {
            };
        },
    };
}






