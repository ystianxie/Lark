import React, {useEffect, useMemo, useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import "./hostsComponent.css";

const emptyEntry = () => ({ip: "127.0.0.1", hostname: "", comment: "", enabled: true});

export default function HostsComponent({onClose}) {
    const [entries, setEntries] = useState([]);
    const [fullEdit, setFullEdit] = useState(false), [filter, setFilter] = useState("");
    const [loading, setLoading] = useState(true), [saving, setSaving] = useState(false);
    const [message, setMessage] = useState(""), [dirty, setDirty] = useState(false);
    const load = async () => {
        setLoading(true);
        try {
            setEntries(await invoke(fullEdit ? "read_hosts_all" : "read_hosts"));
            setDirty(false);
            setMessage("");
        } catch (e) {
            setMessage(String(e));
        } finally {
            setLoading(false);
        }
    };
    useEffect(() => {
        load();
    }, []);
    const visible = useMemo(() => entries.map((entry, index) => ({
        entry,
        index
    })).filter(({entry}) => `${entry.ip} ${entry.hostname} ${entry.comment || ""}`.toLowerCase().includes(filter.toLowerCase())), [entries, filter]);
    const update = (index, patch) => {
        setEntries(v => v.map((e, i) => i === index ? {...e, ...patch} : e));
        setDirty(true);
    };
    const save = async () => {
        setSaving(true);
        setMessage("");
        try {
            await invoke(fullEdit ? "write_hosts_all" : "write_hosts", {entries});
            await invoke("flush_dns");
            setDirty(false);
            setMessage("已保存并刷新 DNS");
        } catch (e) {
            setMessage(String(e));
        } finally {
            setSaving(false);
        }
    };
    const close = () => {
        if (dirty && !window.confirm("还有未保存的 Hosts 修改，确定关闭吗？")) return;
        onClose?.();
    };
    const toggleFullEdit = async checked => {
        setFullEdit(checked);
        setLoading(true);
        try {
            setEntries(await invoke(checked ? "read_hosts_all" : "read_hosts"));
            setDirty(false);
        } catch (e) {
            setMessage(String(e));
        } finally {
            setLoading(false);
        }
    };
    return <div className="hostsPanel">
        <div className="hostsHeader"><div className="hostsTitle"><h2>Hosts</h2><span>{fullEdit ? "编辑全部有效条目" : "仅编辑 Lark 托管区块"}</span></div><div className="hostsHeaderActions"><label className="hostsFullEdit"><input type="checkbox" checked={fullEdit} onChange={e => toggleFullEdit(e.target.checked)} />全文</label></div></div>
        <div className="hostsToolbar"><input value={filter} onChange={e => setFilter(e.target.value)}
                                                           placeholder="搜索 IP、域名或备注"/>
            <button onClick={() => {
                setEntries(v => [...v, emptyEntry()]);
                setDirty(true);
            }}>添加条目
            </button>
            <button onClick={load} disabled={loading}>重新读取</button>
        </div>
        {message && <div className="hostsMessage">{message}</div>}
        {loading ? <div className="hostsEmpty">正在读取 hosts...</div> :
            <div className="hostsList">{visible.map(({entry, index}) => <div className="hostsRow" key={index}><input
                className="hostsEnabled" type="checkbox" checked={entry.enabled}
                onChange={e => update(index, {enabled: e.target.checked})}/><input value={entry.ip}
                                                                                   onChange={e => update(index, {ip: e.target.value})}
                                                                                   placeholder="IP 地址"/><input
                value={entry.hostname} onChange={e => update(index, {hostname: e.target.value})}
                placeholder="域名"/><input value={entry.comment || ""}
                                           onChange={e => update(index, {comment: e.target.value})}
                                           placeholder="备注（可选）"/>
                <button className="hostsDelete" onClick={() => {
                    setEntries(v => v.filter((_, i) => i !== index));
                    setDirty(true);
                }} aria-label="删除条目" title="删除条目">🗙</button>
            </div>)}{!visible.length && <div className="hostsEmpty">暂无条目，点击“添加条目”开始。</div>}</div>}
        <div className="hostsFooter">
            <span>{fullEdit ? "全文编辑" : `${entries.length} 条`}{dirty ? " · 有未保存修改" : ""}</span>
            <button className="hostsSave" onClick={save}
                    disabled={saving || loading}>{saving ? "保存中..." : "保存全部"}</button>
        </div>
    </div>;
}



