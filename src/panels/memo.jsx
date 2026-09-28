import React, {useEffect, useMemo, useState} from "react";
import "./memo.css";

const STORAGE_KEY = "larkMemoItems";

function loadMemos() {
    try {
        const value = JSON.parse(localStorage.getItem(STORAGE_KEY) || "[]");
        return Array.isArray(value) ? value : [];
    } catch { return []; }
}

function formatTime(value) {
    return new Intl.DateTimeFormat("zh-CN", {month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit"}).format(value);
}

export default function Memo() {
    const [memos, setMemos] = useState(loadMemos);
    const [activeId, setActiveId] = useState(null);
    const [query, setQuery] = useState("");

    useEffect(() => localStorage.setItem(STORAGE_KEY, JSON.stringify(memos)), [memos]);
    const active = memos.find((memo) => memo.id === activeId);
    const visible = useMemo(() => memos.filter((memo) => `${memo.title} ${memo.content}`.toLowerCase().includes(query.trim().toLowerCase())), [memos, query]);

    const createMemo = () => {
        const now = Date.now();
        const memo = {id: `${now}-${Math.random().toString(36).slice(2)}`, title: "未命名备忘", content: "", updatedAt: now};
        setMemos((items) => [memo, ...items]);
        setActiveId(memo.id);
    };
    const updateActive = (patch) => setMemos((items) => items.map((memo) => memo.id === activeId ? {...memo, ...patch, updatedAt: Date.now()} : memo));
    const removeActive = () => { setMemos((items) => items.filter((memo) => memo.id !== activeId)); setActiveId(null); };

    return <div className="memo-page">
        <aside className="memo-sidebar">
            <div className="memo-sidebar-head"><h1>备忘录</h1><button onClick={createMemo}>＋</button></div>
            <input className="memo-search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索备忘…" />
            <div className="memo-list">{visible.map((memo) => <button key={memo.id} className={`memo-list-item${memo.id === activeId ? " active" : ""}`} onClick={() => setActiveId(memo.id)}><strong>{memo.title || "未命名备忘"}</strong><span>{memo.content || "暂无内容"}</span><time>{formatTime(memo.updatedAt)}</time></button>)}{!visible.length && <div className="memo-empty">还没有备忘</div>}</div>
        </aside>
        <main className="memo-editor">{active ? <><input className="memo-title" value={active.title} onChange={(event) => updateActive({title: event.target.value})} placeholder="标题" /><textarea value={active.content} onChange={(event) => updateActive({content: event.target.value})} placeholder="写下你的想法…" autoFocus /><div className="memo-editor-foot"><span>最后编辑于 {formatTime(active.updatedAt)}</span><button onClick={removeActive}>删除备忘</button></div></> : <div className="memo-placeholder"><div>选择一条备忘开始编辑</div><button onClick={createMemo}>新建备忘</button></div>}</main>
    </div>;
}
