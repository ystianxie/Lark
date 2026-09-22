import React, {useCallback, useEffect, useRef, useState} from "react";
import {Alert, Button, Checkbox, Input, Select} from "antd";
import {invoke} from "@tauri-apps/api/core";
import {bitmapExtension, bitmapRejectionReason, codeExamples, generatePluginFiles, generateDefaultIcon, newConfigField, pluginConfigFieldTypes, pluginPermissions, svgRejectionReason} from "../pluginScaffold";
import {invalidatePluginRuntime} from "../pluginRuntime";
import CodeEditor from "./CodeEditor";

const svgMime = "image/svg+xml";
const iconMimeByExtension = {svg: svgMime, png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg",
    webp: "image/webp", ico: "image/x-icon"};
const iconMimeByType = {"image/svg+xml": svgMime, "image/png": "image/png", "image/jpeg": "image/jpeg",
    "image/webp": "image/webp", "image/x-icon": "image/x-icon", "image/vnd.microsoft.icon": "image/x-icon"};

// 文件选择框能拿到 MIME，窗口拖放只有路径，因此先按扩展名判断，再用 MIME 兜底。
function iconKindFromNameAndType(name, type) {
    const extension = String(name || "").split(".").pop().toLowerCase();
    return iconMimeByExtension[extension] || iconMimeByType[String(type || "").toLowerCase()] || null;
}

const newWorkflow = number => ({id: `task-${number}`, title: "", type: "action", keywords: "",
    code: codeExamples.action, pythonCode: codeExamples.python, url: "", permissions: []});

export default function PluginCreator({existingIds, onRegistered, onClose, editPlugin, panelDropHandlerRef}) {
    const [config, setConfig] = useState(() => ({id: "", name: "", description: "", icon: "",
        workflows: [newWorkflow(1)], configFields: [], ...(editPlugin || {})}));
    const [files, setFiles] = useState(null);
    const [previewFile, setPreviewFile] = useState("manifest.json");
    const [error, setError] = useState("");
    const [saving, setSaving] = useState(false);
    const [createdPath, setCreatedPath] = useState("");
    const [registered, setRegistered] = useState(false);
    const [focusedWorkflow, setFocusedWorkflow] = useState(null);
    const submitting = useRef(false);
    const nextWorkflowNumber = useRef(2);
    const [iconError, setIconError] = useState("");
    const updateWorkflow = (index, field, value) => setConfig(current => ({...current,
        workflows: current.workflows.map((workflow, position) => position === index ? {...workflow, [field]: value} : workflow)}));
    const updateConfigField = (index, field, value) => setConfig(current => ({...current,
        configFields: (current.configFields || []).map((item, position) => position === index ? {...item, [field]: value} : item)}));

    const preview = () => {
        try {
            const prepared = {...config, workflows: config.workflows.map(workflow => ({...workflow,
                code: workflow.type === "python" ? workflow.pythonCode : workflow.code}))};
            const otherIds = editPlugin
                ? existingIds.filter(id => String(id).toLowerCase() !== String(config.id).trim().toLowerCase())
                : existingIds;
            setFiles(generatePluginFiles(prepared, otherIds));
            setPreviewFile("manifest.json");
            setError("");
        } catch (error) { setError(String(error.message || error)); }
    };

    const applyIconText = useCallback(text => {
        const reason = svgRejectionReason(text);
        if (reason) { setIconError(`无法使用这个 SVG：${reason}`); return; }
        setConfig(current => ({...current, icon: text}));
        setIconError("");
    }, []);

    // 位图以 data URL 存进 config.icon，生成文件时由宿主解码成二进制写入 assets/icon.<ext>。
    const applyIconBitmap = useCallback(dataUrl => {
        const reason = bitmapRejectionReason(dataUrl);
        if (reason) { setIconError(`无法使用这个图标：${reason}`); return; }
        setConfig(current => ({...current, icon: dataUrl}));
        setIconError("");
    }, []);

    // 文件选择框与 HTML5 拖放给的是 File 对象；窗口拖放（Tauri 事件）只给路径。
    const readIcon = file => {
        if (!file) return;
        const kind = iconKindFromNameAndType(file.name, file.type);
        if (!kind) { setIconError("只支持 SVG、PNG、JPEG、WebP 或 ICO 图标"); return; }
        const reader = new FileReader();
        reader.onload = () => {
            if (kind === svgMime) {
                applyIconText(String(reader.result));
                return;
            }
            // 不沿用浏览器给出的 MIME：file.type 缺失时会得到 application/octet-stream。
            applyIconBitmap(`data:${kind};base64,${String(reader.result).split(",")[1] || ""}`);
        };
        reader.onerror = () => setIconError("读取图标失败");
        if (kind === svgMime) reader.readAsText(file);
        else reader.readAsDataURL(file);
    };

    const readIconPath = useCallback(async path => {
        const kind = typeof path === "string" ? iconKindFromNameAndType(path, "") : null;
        if (!kind) {
            setIconError("只支持 SVG、PNG、JPEG、WebP 或 ICO 图标");
            return;
        }
        try {
            // SVG 是文本可以直接读；位图要走二进制读取，再拼成 data URL。
            if (kind === svgMime) {
                applyIconText(await invoke("read_txt", {filePath: path}));
            } else {
                const payload = await invoke("read_file_to_base64", {path});
                applyIconBitmap(`data:${kind};base64,${String(payload)}`);
            }
        } catch (error) {
            setIconError(`读取图标失败：${String(error)}`);
        }
    }, [applyIconBitmap, applyIconText]);

    // 窗口的拖放事件统一由 App 分发；面板挂载期间在此接管，卸载时交还。
    useEffect(() => {
        if (!panelDropHandlerRef) return undefined;
        panelDropHandlerRef.current = readIconPath;
        return () => { panelDropHandlerRef.current = null; };
    }, [panelDropHandlerRef, readIconPath]);

    const create = async () => {
        if (submitting.current) return;
        submitting.current = true;
        setSaving(true);
        setError("");
        let path = createdPath;
        try {
            if (!path) {
                path = await invoke(editPlugin ? "update_plugin" : "create_plugin", {pluginId: config.id.trim(), files});
                if (editPlugin) await invalidatePluginRuntime(config.id.trim());
                setCreatedPath(path);
            }
            await onRegistered(config.id.trim());
            setRegistered(true);
        } catch (error) {
            setError(`${path ? "文件已保存，但开启或刷新列表失败，请重试：" : (editPlugin ? "保存失败：" : "创建失败：")}${String(error)}`);
        } finally {
            submitting.current = false;
            setSaving(false);
        }
    };

    return <section className="plugin-library plugin-creator" onKeyDown={event => {
        event.stopPropagation();
        if (event.key === "Escape" && !saving && !event.nativeEvent.isComposing) onClose();
    }}>
        <header className="library-toolbar">
            <div><h2>{createdPath ? (editPlugin ? "插件已更新" : "插件已创建") : files ? "预览插件文件" : (editPlugin ? "编辑插件" : "新增插件")}</h2>
                <p>自动生成入口 · 无需构建 · 默认版本 1.0.0</p></div>
            <Button disabled={saving} onClick={onClose}>{createdPath ? "完成" : "返回组件库"}</Button>
        </header>
        {error && <Alert type="error" showIcon message={error}/>}
        <div className={`library-list creator-content${focusedWorkflow !== null ? " creator-content-focus" : ""}`}>
            {createdPath ? <>
                <Alert type={registered ? "success" : "info"} showIcon
                    message={registered ? (editPlugin ? "已保存并更新组件库" : "已开启并更新组件库") : "文件已保存，请重试开启及刷新"}
                    description={<div className="creator-path">{createdPath}</div>}/>
                <p>使用功能关键词搜索此插件。后续可直接修改生成的文件；修改已加载的 JS 后请重启应用。</p>
            </> : files ? <>
                <Alert type="info" message="仅预览生成文件，创建时不会运行你的代码。"/>
                <Select aria-label="预览文件" value={previewFile} onChange={setPreviewFile}
                    options={Object.keys(files).map(path => ({value: path, label: path}))}/>
                <pre className="creator-preview">{files[previewFile]}</pre>
            </> : <>
                <div className="creator-grid">
                    <label>插件 ID<Input aria-label="插件 ID" placeholder="my-plugin" value={config.id}
                        disabled={Boolean(editPlugin)} onChange={event => setConfig({...config, id: event.target.value})}/></label>
                    <label>插件名称<Input aria-label="插件名称" value={config.name}
                        onChange={event => setConfig({...config, name: event.target.value})}/></label>
                </div>
                <label>描述<Input.TextArea aria-label="插件描述" rows={2} value={config.description}
                    onChange={event => setConfig({...config, description: event.target.value})}/></label>
                <div className="creator-icon-field">
                    <span>插件图标</span>
                    {/* Windows 上 HTML5 拖放事件被 Tauri 的 drop handler 接管，收不到；
                        真正生效的是 readIconPath 通过 panelDropHandlerRef 注册的路径。
                        这里保留为其他平台 / 关闭 dragDropEnabled 时的兜底。 */}
                    <div className="creator-icon-drop" onDragOver={event => event.preventDefault()}
                        onDrop={event => { event.preventDefault(); readIcon(event.dataTransfer.files?.[0]); }}>
                        <div className="creator-icon-preview">
                            {bitmapExtension(config.icon)
                                ? <img src={config.icon} alt=""/>
                                : <div dangerouslySetInnerHTML={{__html: config.icon || generateDefaultIcon(config.id || "plugin")}}/>}
                        </div>
                        <div><p>拖拽 SVG、PNG、JPEG、WebP 或 ICO 到这里，或选择文件</p>
                            <input type="file"
                                accept=".svg,.png,.jpg,.jpeg,.webp,.ico,image/svg+xml,image/png,image/jpeg,image/webp,image/x-icon"
                                aria-label="上传图标"
                                onChange={event => readIcon(event.target.files?.[0])}/>
                            <small>未上传时会生成随机风格 SVG 图标；位图最大 256 KB</small></div>
                    </div>
                    {iconError && <Alert type="error" message={iconError}/>} 
                </div>
                <Alert type="info" message="输入变化会反复运行代码"
                    description="action / python 在非空输入后实时执行（200ms 防抖）。已启动的脚本不会因清空输入而撤销。"/>
                {config.workflows.map((workflow, index) => <fieldset className="creator-workflow" key={index}>
                    <legend>功能入口 {index + 1}</legend>
                    <div className="creator-grid">
                        <label>功能 ID<Input aria-label={`功能 ID ${index + 1}`} value={workflow.id}
                            onChange={event => updateWorkflow(index, "id", event.target.value)}/></label>
                        <label>功能名称<Input aria-label={`功能名称 ${index + 1}`} value={workflow.title}
                            onChange={event => updateWorkflow(index, "title", event.target.value)}/></label>
                        <label>关键词<Input aria-label={`关键词 ${index + 1}`} placeholder="多个关键词用逗号分隔"
                            value={workflow.keywords} onChange={event => updateWorkflow(index, "keywords", event.target.value)}/></label>
                        <label>类型<Select aria-label={`功能类型 ${index + 1}`} value={workflow.type}
                            options={[{value: "action", label: "action · JavaScript"}, {value: "url", label: "url · 打开网址"},
                                {value: "python", label: "python · Python 脚本"}]}
                            onChange={value => updateWorkflow(index, "type", value)}/></label>
                    </div>
                    {workflow.type === "url" ? <label>完整网址<Input aria-label={`网址 ${index + 1}`} placeholder="https://example.com"
                        value={workflow.url} onChange={event => updateWorkflow(index, "url", event.target.value)}/></label> : <>
                        <div className="creator-code-shell"><label>{workflow.type === "python" ? "Python 函数体" : "JS 异步函数体"}
                            <CodeEditor language={workflow.type === "python" ? "python" : "javascript"}
                                value={workflow.type === "python" ? workflow.pythonCode : workflow.code}
                                onChange={value => updateWorkflow(index, workflow.type === "python" ? "pythonCode" : "code", value)}/></label>
                            <Button size="small" onClick={() => setFocusedWorkflow(index)}>⛶ 专注编辑</Button>
                        </div>
                    <p className="library-hint">可用参数：{workflow.type === "python" ? "text、file、config（已保存的插件配置，未配置时为空字典）" : "text、file、context"}。
                            返回文本、数字、{'{title, data, desc}'} 或数组；空返回不显示结果。</p>
                        {workflow.type === "python" ? <p className="library-hint">使用系统 Python 或已有解释器配置；本次不创建虚拟环境或安装依赖。
                            config 的键即变量名，用 config.get("api_key") 读取；未配置时请自行兜底默认值。</p>
                            : <label>宿主 API 权限<Checkbox.Group options={pluginPermissions} value={workflow.permissions}
                                onChange={value => updateWorkflow(index, "permissions", value)}/></label>}
                    </>}
                    <Button danger size="small" disabled={config.workflows.length === 1} onClick={() => setConfig({...config,
                        workflows: config.workflows.filter((_, position) => position !== index)})}>删除此入口</Button>
                </fieldset>)}
                <Button onClick={() => {
                    const workflow = newWorkflow(nextWorkflowNumber.current++);
                    setConfig({...config, workflows: [...config.workflows, workflow]});
                }}>添加功能入口</Button>
                <fieldset className="creator-workflow">
                    <legend>配置项</legend>
                    <p className="library-hint">可选。声明后组件库会出现「配置」按钮，用户填写的值保存在宿主配置中，
                        插件用 <code>await context.api.getConfig()</code> 读取；没有配置项的插件不需要填写这里。</p>
                    {(config.configFields || []).map((field, index) => <div className="creator-config-item" key={index}>
                        <div className="creator-grid">
                            <label>变量名<Input aria-label={`配置项变量名 ${index + 1}`} placeholder="api_key"
                                value={field.key} onChange={event => updateConfigField(index, "key", event.target.value)}/></label>
                            <label>显示名称<Input aria-label={`配置项名称 ${index + 1}`} placeholder="API Key"
                                value={field.label} onChange={event => updateConfigField(index, "label", event.target.value)}/></label>
                            <label>类型<Select aria-label={`配置项类型 ${index + 1}`} value={field.type}
                                options={pluginConfigFieldTypes}
                                onChange={value => updateConfigField(index, "type", value)}/></label>
                            <label>{field.type === "boolean" ? "默认值（true / false）" : "默认值（可留空）"}
                                <Input aria-label={`配置项默认值 ${index + 1}`} value={field.defaultValue}
                                    onChange={event => updateConfigField(index, "defaultValue", event.target.value)}/></label>
                        </div>
                        <label>说明<Input aria-label={`配置项说明 ${index + 1}`} placeholder="在服务商控制台获取"
                            value={field.description} onChange={event => updateConfigField(index, "description", event.target.value)}/></label>
                        {field.type === "select" ? <label>选项（逗号分隔）<Input aria-label={`配置项选项 ${index + 1}`}
                            placeholder="zh, en, ja" value={field.options}
                            onChange={event => updateConfigField(index, "options", event.target.value)}/></label> : null}
                        <div className="creator-config-footer">
                            <Checkbox checked={Boolean(field.required)}
                                onChange={event => updateConfigField(index, "required", event.target.checked)}>必填</Checkbox>
                            <Button danger size="small" onClick={() => setConfig(current => ({...current,
                                configFields: (current.configFields || []).filter((_, position) => position !== index)}))}>删除此配置项</Button>
                        </div>
                    </div>)}
                    <Button onClick={() => setConfig(current => ({...current,
                        configFields: [...(current.configFields || []), newConfigField()]}))}>添加配置项</Button>
                </fieldset>
            </>}
            {focusedWorkflow !== null && config.workflows[focusedWorkflow] && (() => {
                const workflow = config.workflows[focusedWorkflow];
                const field = workflow.type === "python" ? "pythonCode" : "code";
                return <div className="creator-editor-focus" role="dialog" aria-label="专注编辑模式">
                    <div className="creator-editor-focus-toolbar">
                        <strong>{workflow.type === "python" ? "Python 函数体" : "JS 异步函数体"} · 功能入口 {focusedWorkflow + 1}</strong>
                        <Button size="small" onClick={() => setFocusedWorkflow(null)}>↙ 退出专注</Button>
                    </div>
                    <CodeEditor autoFocus className="creator-editor-focus-input"
                        language={workflow.type === "python" ? "python" : "javascript"}
                        value={workflow[field]} onChange={value => updateWorkflow(focusedWorkflow, field, value)}/>
                </div>;
            })()}
        </div>
        <footer className="library-actions creator-footer">
            {files && !createdPath && <Button disabled={saving} onClick={() => {setFiles(null); setError("");}}>返回修改</Button>}
            {!registered && <Button type="primary" loading={saving} onClick={files ? create : preview}>
                {createdPath ? "重试开启及刷新" : files ? (editPlugin ? "保存修改" : "创建并注册") : "预览生成文件"}
            </Button>}
        </footer>
    </section>;
}
