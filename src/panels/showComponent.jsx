import React, {useEffect, useRef, useState} from "react";
import {Alert, Button, Empty, Input, Switch, Tag} from "antd";
import {convertFileSrc, invoke} from "@tauri-apps/api/core";
import "./showComponent.css";
import PluginCreator from "./PluginCreator";
import PluginSettings from "./PluginSettings";
import {modifyWindowSize} from "../template.jsx";

function PluginIcon({plugin}) {
    const [failed, setFailed] = useState(false);
    const workflows = Array.isArray(plugin.workflows) ? plugin.workflows : [];
    const icon = plugin.icon || workflows.find(workflow => typeof workflow?.icon === "string")?.icon;
    return <div className="library-icon">
        {typeof icon === "string" && !failed
            ? <img src={convertFileSrc(`${plugin.__root}/${icon.replace(/^\.\//, "")}`)} alt=""
                   onError={() => setFailed(true)}/>
            : String(plugin.name || plugin.__pluginId).slice(0, 2)}
    </div>;
}

export default function Component({plugins = {}, pluginStatus, loading, error, onRefresh, onToggle, onClose, pluginConfigId, panelDropHandlerRef}) {
    const [query, setQuery] = useState("");
    const [saveError, setSaveError] = useState("");
    const [creating, setCreating] = useState(false);
    const [editing, setEditing] = useState(null);
    const [configuring, setConfiguring] = useState(null);
    useEffect(() => {
        if (!creating && !editing) return undefined;
        modifyWindowSize("workspace");
        return () => { modifyWindowSize("expanded"); };
    }, [creating, editing]);
    // 由搜索结果拦截带过来的目标插件：组件库加载出插件后自动进入它的配置页。
    const openedConfigRequest = useRef("");
    useEffect(() => {
        if (!pluginConfigId || openedConfigRequest.current === pluginConfigId) return;
        const plugin = plugins[pluginConfigId];
        if (!plugin || !Array.isArray(plugin.config?.fields) || !plugin.config.fields.length) return;
        openedConfigRequest.current = pluginConfigId;
        setConfiguring(plugin);
    }, [pluginConfigId, plugins]);
    const entries = Object.values(plugins).sort((left, right) =>
        String(left.name || left.__pluginId).localeCompare(String(right.name || right.__pluginId), "zh-CN"));
    const enabledCount = entries.filter(plugin => !plugin.__error && pluginStatus?.[plugin.__pluginId]?.enable !== false).length;
    const visible = entries.filter(plugin => {
        const keywords = Array.isArray(plugin.workflows)
            ? plugin.workflows.flatMap(workflow => workflow?.keywords || []) : [];
        return [plugin.name, plugin.__pluginId, plugin.description, ...keywords]
            .join(" ").toLowerCase().includes(query.trim().toLowerCase());
    });

    if (configuring) return <PluginSettings plugin={configuring} onClose={() => setConfiguring(null)}
        onSaved={onRefresh}/>;

    if (creating || editing) return <PluginCreator editPlugin={editing} existingIds={Object.keys(plugins)}
        panelDropHandlerRef={panelDropHandlerRef} onClose={() => {setCreating(false); setEditing(null);}}
        onRegistered={async pluginId => {
            if (!editing) onToggle(pluginId, true);
            const result = await onRefresh();
            if (!result?.ok) throw new Error(result?.error || "刷新失败");
        }}/>;

    return <section className="plugin-library" onKeyDown={event => {
        if (event.key === "Escape") { event.stopPropagation(); onClose(); }
    }}>
        <header className="library-toolbar">
            <div><h2>组件库</h2><p>共 {entries.length} 个插件 · 已开启 {enabledCount} 个</p></div>
            <div className="library-actions">
                <Button type="primary" onClick={() => setCreating(true)}>新增插件</Button>
                <Button onClick={onRefresh} loading={loading}>刷新</Button>
                <Button onClick={onClose}>返回</Button>
            </div>
        </header>
        <Input placeholder="搜索名称、插件 ID 或关键词" aria-label="搜索插件" allowClear
               value={query} onChange={event => setQuery(event.target.value)}/>
        <p className="library-hint">关闭后不再出现在搜索结果中，设置自动保存。创建或编辑成功后会自动刷新。</p>
        {error && <Alert type="error" showIcon message="读取插件失败，可点击刷新重试" description={error}/>}
        {saveError && <Alert type="error" showIcon message="插件操作失败" description={saveError}/>}
        <div className="library-list" aria-busy={loading}>
            {!visible.length && <Empty image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={loading ? "正在加载插件…" : query ? "没有匹配的插件" : "未发现插件，请在插件目录添加 manifest.json"}/>}
            {visible.map(plugin => {
                const pluginId = plugin.__pluginId;
                const enabled = pluginStatus?.[pluginId]?.enable !== false;
                const workflows = Array.isArray(plugin.workflows) ? plugin.workflows : [];
                return <article className="library-card" key={pluginId}>
                    <PluginIcon key={`${pluginId}:${plugin.__root}`} plugin={plugin}/>
                    <div className="library-details">
                        <h3>{plugin.name || pluginId} <span>{plugin.version ? `v${plugin.version}` : ""}</span></h3>
                        <p>{plugin.description || "暂无描述"}</p>
                        <div className="library-id">{pluginId}</div>
                        {plugin.__error ? <Alert type="error" message="插件配置异常" description={plugin.__error}/>
                            : <div className="library-keywords">{workflows.map(workflow =>
                                <div key={workflow.id} title={workflow.description}>
                                    <span>{workflow.title || workflow.id}：</span>
                                    {workflow.keywords.map(keyword => <Tag key={keyword}>{keyword}</Tag>)}
                                </div>)}</div>}
                    </div>
                    {Array.isArray(plugin.config?.fields) && plugin.config.fields.length > 0
                        ? <Button size="small" title="填写该插件声明的配置项"
                            onClick={() => { setConfiguring(plugin); setSaveError(""); }}>配置</Button>
                        : null}
                    <Button size="small" disabled={Boolean(plugin.__error) || !plugin.__editable}
                            title={plugin.__editable ? "编辑向导配置并重新生成插件文件" : "仅支持编辑由新增向导创建的用户插件"}
                            onClick={async () => { try { const data = await invoke("load_plugin_editor", {pluginId}); setEditing(data); } catch (e) { setSaveError(String(e)); } }}>编辑</Button>
                    <Switch checked={!plugin.__error && enabled} disabled={Boolean(plugin.__error)}
                            checkedChildren="开启" unCheckedChildren="关闭" aria-label={`${plugin.name || pluginId}开关`}
                            onChange={checked => {
                                try { onToggle(pluginId, checked); setSaveError(""); }
                                catch (error) { setSaveError(String(error)); }
                            }}/>
                </article>;
            })}
        </div>
    </section>;
}
