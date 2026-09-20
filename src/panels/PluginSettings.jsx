import React, {useEffect, useState} from "react";
import {Alert, Button, Input, InputNumber, Popconfirm, Select, Switch} from "antd";
import {invoke} from "@tauri-apps/api/core";

const fieldTypes = ["text", "password", "number", "boolean", "select", "textarea"];
// 与后端 validate_plugin_config 的上限保持一致；手动安装的插件绕过校验，这里再截断一次。
const maxFields = 24;

function normalizeOptions(options) {
    if (!Array.isArray(options)) return [];
    const normalized = [];
    for (const option of options) {
        if (typeof option === "string" || typeof option === "number") {
            normalized.push({value: option, label: String(option)});
        } else if (option && typeof option === "object" &&
            (typeof option.value === "string" || typeof option.value === "number")) {
            normalized.push({value: option.value,
                label: typeof option.label === "string" ? option.label : String(option.value)});
        }
    }
    return normalized;
}

// 手动安装或缺少合法 scaffold 的插件不经过向导校验，因此渲染前做归一化，未知类型降级为 text。
function normalizeField(field) {
    if (!field || typeof field !== "object") return null;
    if (typeof field.key !== "string" || !field.key) return null;
    return {
        key: field.key,
        label: typeof field.label === "string" && field.label.trim() ? field.label : field.key,
        type: fieldTypes.includes(field.type) ? field.type : "text",
        required: field.required === true,
        description: typeof field.description === "string" ? field.description : "",
        placeholder: typeof field.placeholder === "string" ? field.placeholder : "",
        default: field.default,
        min: typeof field.min === "number" ? field.min : undefined,
        max: typeof field.max === "number" ? field.max : undefined,
        options: normalizeOptions(field.options),
    };
}

function isEmptyValue(value) {
    if (value === null || value === undefined) return true;
    return typeof value === "string" && value.trim() === "";
}

export default function PluginSettings({plugin, onClose, onSaved}) {
    const pluginId = plugin?.__pluginId || plugin?.id;
    // 归一化 + 去重 + 截断：组件库的 plugin.config 直接来自 manifest，
    // 手动安装的插件没有经过后端校验，重复 key 会让 React key 冲突并互相覆盖。
    const fields = [];
    const seenKeys = new Set();
    for (const rawField of Array.isArray(plugin?.config?.fields) ? plugin.config.fields : []) {
        const field = normalizeField(rawField);
        if (!field || seenKeys.has(field.key)) continue;
        seenKeys.add(field.key);
        fields.push(field);
        if (fields.length >= maxFields) break;
    }
    const [values, setValues] = useState({});
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [error, setError] = useState("");
    const [notice, setNotice] = useState("");

    const defaults = () => Object.fromEntries(fields.map(field => [field.key, field.default]));

    useEffect(() => {
        let disposed = false;
        setLoading(true);
        invoke("get_plugin_settings", {pluginId}).then((stored) => {
            if (disposed) return;
            const next = {};
            const record = stored && typeof stored === "object" ? stored : {};
            for (const field of fields) {
                next[field.key] = Object.hasOwn(record, field.key) ? record[field.key] : field.default;
            }
            setValues(next);
            setError("");
        }).catch((err) => {
            if (!disposed) setError(`读取配置失败：${String(err)}`);
        }).finally(() => {
            if (!disposed) setLoading(false);
        });
        return () => { disposed = true; };
        // plugin 在配置视图的生命周期内不变，因此这里只跟随 pluginId 重新拉取。
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [pluginId]);

    const updateValue = (key, value) => {
        setValues(current => ({...current, [key]: value}));
        setNotice("");
    };

    const save = async () => {
        const missing = fields
            .filter(field => field.required && isEmptyValue(values[field.key]))
            .map(field => field.label);
        if (missing.length) {
            setNotice("");
            setError(`请先填写：${missing.join("、")}`);
            return;
        }
        setSaving(true);
        try {
            await invoke("save_plugin_settings", {pluginId, values});
            setError("");
            setNotice("配置已保存");
            // 通知宿主刷新「必填是否齐全」摘要，下次搜索就会放行该插件。
            onSaved?.();
        } catch (err) {
            setNotice("");
            setError(`保存失败：${String(err)}`);
        } finally {
            setSaving(false);
        }
    };

    const clear = async () => {
        try {
            await invoke("clear_plugin_settings", {pluginId});
            setValues(defaults());
            setError("");
            setNotice("配置已清除");
            onSaved?.();
        } catch (err) {
            setNotice("");
            setError(`清除失败：${String(err)}`);
        }
    };

    const renderControl = (field) => {
        const value = values[field.key];
        const disabled = loading || saving;
        if (field.type === "boolean") {
            return <Switch checked={value === true} disabled={disabled}
                onChange={(checked) => updateValue(field.key, checked)}/>;
        }
        if (field.type === "number") {
            return <InputNumber style={{width: "100%"}} min={field.min} max={field.max}
                value={typeof value === "number" ? value : null} disabled={disabled}
                placeholder={field.placeholder}
                onChange={(next) => updateValue(field.key, next ?? null)}/>;
        }
        if (field.type === "select") {
            return <Select value={value ?? null} options={field.options} disabled={disabled} allowClear
                placeholder={field.placeholder}
                onChange={(next) => updateValue(field.key, next ?? null)}/>;
        }
        if (field.type === "textarea") {
            return <Input.TextArea autoSize={{minRows: 2, maxRows: 6}} disabled={disabled}
                value={typeof value === "string" ? value : ""} placeholder={field.placeholder}
                onChange={(event) => updateValue(field.key, event.target.value)}/>;
        }
        if (field.type === "password") {
            return <Input.Password disabled={disabled} value={typeof value === "string" ? value : ""}
                placeholder={field.placeholder}
                onChange={(event) => updateValue(field.key, event.target.value)}/>;
        }
        return <Input disabled={disabled} value={typeof value === "string" ? value : ""}
            placeholder={field.placeholder}
            onChange={(event) => updateValue(field.key, event.target.value)}/>;
    };

    return <section className="plugin-library plugin-creator" onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === "Escape" && !saving && !event.nativeEvent.isComposing) onClose();
    }}>
        <header className="library-toolbar">
            <div>
                <h2>{plugin?.name || pluginId} 配置</h2>
                <p>表单由插件声明生成 · 保存后立即对插件生效</p>
            </div>
            <Button onClick={onClose}>返回组件库</Button>
        </header>
        {error && <Alert type="error" showIcon message={error}/>}
        {!error && notice && <Alert type="success" showIcon message={notice}/>}
        <p className="library-hint">配置保存在宿主配置中，插件通过 context.api.getConfig() 读取；此处不会改写插件文件。</p>
        <div className="library-list creator-content">
            {loading ? <p className="library-hint">正在读取配置…</p>
                : fields.length === 0 ? <p className="library-hint">该插件没有声明任何配置项。</p>
                    : fields.map(field => <label className="plugin-settings-field" key={field.key}>
                        <span>{field.label}{field.required ? <span className="plugin-settings-required">*</span> : null}</span>
                        {renderControl(field)}
                        {field.description ? <span className="plugin-settings-description">{field.description}</span> : null}
                    </label>)}
        </div>
        <footer className="library-actions creator-footer">
            <Popconfirm title="清除该插件的全部配置？" okText="清除" cancelText="取消" onConfirm={clear}>
                <Button danger disabled={loading || saving || fields.length === 0}>清除配置</Button>
            </Popconfirm>
            <Button type="primary" loading={saving} disabled={loading || fields.length === 0} onClick={save}>保存配置</Button>
        </footer>
    </section>;
}
