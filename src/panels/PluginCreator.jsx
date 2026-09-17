import React, {useRef, useState} from "react";
import {Alert, Button, Checkbox, Input, Select} from "antd";
import {invoke} from "@tauri-apps/api/core";
import {codeExamples, generatePluginFiles, generateDefaultIcon, isSafeSvg, pluginPermissions} from "../pluginScaffold";

const newWorkflow = number => ({id: `task-${number}`, title: "", type: "action", keywords: "",
    code: codeExamples.action, pythonCode: codeExamples.python, url: "", permissions: []});

export default function PluginCreator({existingIds, onRegistered, onClose, editPlugin}) {
    const [config, setConfig] = useState(editPlugin || {id: "", name: "", description: "", icon: "", workflows: [newWorkflow(1)]});
    const [files, setFiles] = useState(null);
    const [previewFile, setPreviewFile] = useState("manifest.json");
    const [error, setError] = useState("");
    const [saving, setSaving] = useState(false);
    const [createdPath, setCreatedPath] = useState("");
    const [registered, setRegistered] = useState(false);
    const submitting = useRef(false);
    const nextWorkflowNumber = useRef(2);
    const [iconError, setIconError] = useState("");
    const updateWorkflow = (index, field, value) => setConfig(current => ({...current,
        workflows: current.workflows.map((workflow, position) => position === index ? {...workflow, [field]: value} : workflow)}));

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

    const readIcon = file => {
        if (!file || file.type && file.type !== "image/svg+xml") { setIconError("目前只支持 SVG 图标"); return; }
        const reader = new FileReader();
        reader.onload = () => {
            if (!isSafeSvg(reader.result)) { setIconError("SVG 仅允许静态图形，不允许脚本、外部资源或事件属性"); return; }
            setConfig(current => ({...current, icon: reader.result}));
            setIconError("");
        };
        reader.onerror = () => setIconError("读取图标失败");
        reader.readAsText(file);
    };

    const create = async () => {
        if (submitting.current) return;
        submitting.current = true;
        setSaving(true);
        setError("");
        let path = createdPath;
        try {
            if (!path) {
                path = await invoke(editPlugin ? "update_plugin" : "create_plugin", {pluginId: config.id.trim(), files});
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
        <div className="library-list creator-content">
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
                    <div className="creator-icon-drop" onDragOver={event => event.preventDefault()}
                        onDrop={event => { event.preventDefault(); readIcon(event.dataTransfer.files?.[0]); }}>
                        <div className="creator-icon-preview" dangerouslySetInnerHTML={{__html: config.icon || generateDefaultIcon(config.id || "plugin")}} />
                        <div><p>拖拽 SVG 到这里，或选择文件</p>
                            <input type="file" accept="image/svg+xml,.svg" aria-label="上传 SVG 图标"
                                onChange={event => readIcon(event.target.files?.[0])}/>
                            <small>未上传时会生成随机风格 SVG 图标</small></div>
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
                        <label>{workflow.type === "python" ? "Python 函数体" : "JS 异步函数体"}
                            <Input.TextArea aria-label={`业务代码 ${index + 1}`} className="creator-code" rows={6} spellCheck={false}
                                value={workflow.type === "python" ? workflow.pythonCode : workflow.code}
                                onChange={event => updateWorkflow(index, workflow.type === "python" ? "pythonCode" : "code", event.target.value)}/></label>
                    <p className="library-hint">可用参数：{workflow.type === "python" ? "text、file" : "text、file、context"}。
                            返回文本、数字、{'{title, data, desc}'} 或数组；空返回不显示结果。</p>
                        {workflow.type === "python" ? <p className="library-hint">使用系统 Python 或已有解释器配置；本次不创建虚拟环境或安装依赖。</p>
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
            </>}
        </div>
        <footer className="library-actions creator-footer">
            {files && !createdPath && <Button disabled={saving} onClick={() => {setFiles(null); setError("");}}>返回修改</Button>}
            {!registered && <Button type="primary" loading={saving} onClick={files ? create : preview}>
                {createdPath ? "重试开启及刷新" : files ? (editPlugin ? "保存修改" : "创建并注册") : "预览生成文件"}
            </Button>}
        </footer>
    </section>;
}
