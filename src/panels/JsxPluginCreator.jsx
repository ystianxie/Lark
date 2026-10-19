import React, { useRef, useState } from "react";
import { Alert, Button, Input } from "antd";
import { invoke } from "@tauri-apps/api/core";

export default function JsxPluginCreator({ existingIds, onRegistered, onClose }) {
    const [id, setId] = useState("");
    const [name, setName] = useState("");
    const [description, setDescription] = useState("");
    const [path, setPath] = useState("");
    const [error, setError] = useState("");
    const [saving, setSaving] = useState(false);
    const [registered, setRegistered] = useState(false);
    const submitting = useRef(false);
    const create = async () => {
        if (submitting.current) return;
        submitting.current = true;
        setSaving(true);
        setError("");
        let destination = path;
        try {
            if (!destination) {
                if (existingIds.some(value => value.toLowerCase() === id.trim().toLowerCase())) {
                    throw new Error("插件 ID 已存在，请更换 ID");
                }
                destination = await invoke("create_jsx_plugin", {
                    pluginId: id.trim(), name: name.trim(), description: description.trim(),
                });
                setPath(destination);
            }
            await onRegistered(id.trim());
            setRegistered(true);
        } catch (error) {
            setError(`${destination ? "插件文件已创建，刷新失败，可重试：" : "创建失败："}${String(error)}`);
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
            <div><h2>{path ? "JSX 插件已创建" : "新增 JSX 界面插件"}</h2>
                <p>使用 SDK 编写界面 · build 后刷新即可加载</p></div>
            <Button disabled={saving} onClick={onClose}>{path ? "完成" : "返回组件库"}</Button>
        </header>
        {error && <Alert type="error" showIcon message={error} />}
        <div className="library-list creator-content">
            {path ? <>
                <Alert type={registered ? "success" : "info"} showIcon
                    message={registered ? "已开启并更新组件库，可直接打开初始页面" : "文件已创建，请重试刷新"}
                    description={<div className="creator-path">{path}</div>} />
                <p>在上面的目录打开终端，第一次执行：</p>
                <pre className="creator-preview">{"npm install\nnpm run dev"}</pre>
                <p>主要修改 src/App.jsx。完成后执行 npm run build，再点击组件库的“刷新”，即可加载新代码。</p>
                <p>SDK 已随 Lark 提供。开发需要 Node.js，首次安装依赖需要联网；详细说明见插件目录中的 README.md。</p>
            </> : <>
                <div className="creator-grid">
                    <label>插件 ID<Input aria-label="JSX 插件 ID" placeholder="my-plugin" value={id}
                        onChange={event => setId(event.target.value)} /></label>
                    <label>插件名称<Input aria-label="JSX 插件名称" value={name}
                        onChange={event => setName(event.target.value)} /></label>
                </div>
                <label>描述<Input.TextArea aria-label="JSX 插件描述" rows={2} value={description}
                    onChange={event => setDescription(event.target.value)} /></label>
                <p>将生成完整开发项目、SDK 引用和可直接运行的初始页面。默认用插件名称和 ID 作为搜索关键词，可在 manifest.json 中修改。</p>
            </>}
            {(!path || !registered) && <Button type="primary" loading={saving}
                disabled={!path && (!id.trim() || !name.trim())} onClick={create}>
                {path ? "重试刷新" : "创建 JSX 插件"}
            </Button>}
        </div>
    </section>;
}
