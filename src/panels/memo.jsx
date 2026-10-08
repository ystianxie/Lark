import React, {useEffect, useMemo, useState} from "react";
import {Button, Input, Modal, Popconfirm, Select} from "antd";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {DeleteOutlined, EditOutlined, EyeOutlined} from "@ant-design/icons";
import "./memo.css";

const STORAGE_KEY = "larkMemoItems";
const GROUP_STORAGE_KEY = "larkMemoGroups";

function loadItems(key) {
    try {
        const value = JSON.parse(localStorage.getItem(key) || "[]");
        return Array.isArray(value) ? value : [];
    } catch {
        return [];
    }
}

function formatTime(value) {
    return new Intl.DateTimeFormat("zh-CN", {
        month: "numeric",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit"
    }).format(value);
}

export default function Memo() {
    const [memos, setMemos] = useState(() => loadItems(STORAGE_KEY));
    const [activeId, setActiveId] = useState(null);
    const [editingId, setEditingId] = useState(null);
    const [query, setQuery] = useState("");
    const [groups, setGroups] = useState(() => loadItems(GROUP_STORAGE_KEY));
    const [selectedGroup, setSelectedGroup] = useState(null);
    const [creatingGroup, setCreatingGroup] = useState(false);
    const [groupName, setGroupName] = useState("");
    const [groupError, setGroupError] = useState("");

    useEffect(() => localStorage.setItem(GROUP_STORAGE_KEY, JSON.stringify(groups)), [groups]);

    useEffect(() => localStorage.setItem(STORAGE_KEY, JSON.stringify(memos)), [memos]);
    const active = memos.find((memo) => memo.id === activeId);
    const editing = active != null && editingId === active.id;
    const visible = useMemo(() => memos.filter((memo) => selectedGroup === null || (memo.groupId || "") === selectedGroup).filter((memo) => `${memo.title} ${memo.content}`.toLowerCase().includes(query.trim().toLowerCase())), [memos, query, selectedGroup]);

    const createMemo = () => {
        const now = Date.now();
        const memo = {
            id: `${now}-${Math.random().toString(36).slice(2)}`,
            title: "未命名备忘",
            content: "",
            groupId: selectedGroup || "",
            updatedAt: now
        };
        setMemos((items) => [memo, ...items]);
        setActiveId(memo.id);
        setEditingId(memo.id);
    };
    const updateActive = (patch) => setMemos((items) => items.map((memo) => memo.id === activeId ? {
        ...memo, ...patch,
        updatedAt: Date.now()
    } : memo));
    const removeActive = () => {
        setMemos((items) => items.filter((memo) => memo.id !== activeId));
        setActiveId(null);
    };

    const selectGroup = (id) => {
        setSelectedGroup(id);
        if (active && id !== null && (active.groupId || "") !== id) setActiveId(null);
    };
    const createGroup = () => {
        const name = groupName.trim();
        if (!name) {
            setGroupError("请输入分组名称");
            return;
        }
        if (groups.some((group) => group.name === name)) {
            setGroupError("分组名称已存在");
            return;
        }
        const id = globalThis.crypto?.randomUUID?.() || `${Date.now()}-${Math.random().toString(36).slice(2)}`;
        setGroups((items) => [...items, {id, name}]);
        selectGroup(id);
        setCreatingGroup(false);
    };
    const deleteGroup = (id) => {
        setMemos((items) => items.map((memo) => memo.groupId === id ? {...memo, groupId: ""} : memo));
        setGroups((items) => items.filter((group) => group.id !== id));
        if (selectedGroup === id) setSelectedGroup("");
    };

    return <div className="memo-page">
        <aside className="memo-sidebar">
            <div className="memo-sidebar-head"><h1>备忘录</h1>
                <button onClick={createMemo}>＋</button>
            </div>
            <div className="memo-groups" aria-label="备忘录分组">
                {[{id: null, name: "全部分组"}, {id: "", name: "未分组"}, ...groups].map((group) => (
                    <div key={group.id ?? "all-groups"}
                         className={`memo-group-tab${selectedGroup === group.id ? " active" : ""}`}>
                        <button type="button" className="memo-group-button" aria-pressed={selectedGroup === group.id}
                                onClick={() => selectGroup(group.id)}>{group.name}</button>
                        {group.id && <Popconfirm title={`删除分组“${group.name}”？`}
                                                 description="其中的备忘会移至未分组，不会被删除。" okText="删除"
                                                 cancelText="取消" onConfirm={() => deleteGroup(group.id)}>
                            <button type="button" className="memo-group-delete" aria-label={`删除分组 ${group.name}`}
                                    title="删除分组">×
                            </button>
                        </Popconfirm>}
                    </div>
                ))}
                <button type="button" className="memo-group-button memo-add-group" onClick={() => {
                    setGroupName("");
                    setGroupError("");
                    setCreatingGroup(true);
                }}>+
                </button>
            </div>
            <input className="memo-search" value={query} onChange={(event) => setQuery(event.target.value)}
                   placeholder="搜索备忘…"/>
            <div className="memo-list">{visible.map((memo) => <button key={memo.id}
                                                                      className={`memo-list-item${memo.id === activeId ? " active" : ""}`}
                                                                      onClick={() => {
                                                                          setActiveId(memo.id);
                                                                          setEditingId(null);
                                                                      }}>
                <strong>{memo.title || "未命名备忘"}</strong><span>{memo.content || "暂无内容"}</span>
                <time>{formatTime(memo.updatedAt)}</time>
            </button>)}{!visible.length &&
                <div className="memo-empty">{query ? "没有匹配的备忘" : "该分组还没有备忘"}</div>}</div>
        </aside>
        <main className="memo-editor">{active ? <>
            <div className="memo-title-row"><input className="memo-title" value={active.title}
                                                   onChange={(event) => updateActive({title: event.target.value})}
                                                   placeholder="标题" readOnly={!editing}/><Button
                type="text"
                className="memo-mode-button"
                icon={editing ? <EyeOutlined/> : <EditOutlined/>}
                onClick={() => setEditingId(editing ? null : active.id)}
                aria-label={editing ? "切换到预览模式" : "切换到编辑模式"}
                title={editing ? "预览" : "编辑"}
            /></div>
            {editing ? (
                <textarea aria-label="Markdown 正文" value={active.content}
                          onChange={(event) => updateActive({content: event.target.value})}
                          placeholder="支持 Markdown：标题、列表、链接、代码块…" autoFocus/>
            ) : (
                <div className="memo-markdown-preview" aria-label="备忘预览">
                    {active.content ? <ReactMarkdown remarkPlugins={[remarkGfm]}>{active.content}</ReactMarkdown> :
                        <div className="memo-preview-empty">暂无内容，点击右上角编辑图标开始记录</div>}
                </div>
            )}
            <div className="memo-editor-foot"><span>最后编辑于 {formatTime(active.updatedAt)}</span>
                <div className="memo-editor-actions"><Select
                    className="memo-group-select"
                    aria-label="备忘分组"
                    value={active.groupId || ""}
                    options={[{value: "", label: "未分组"}, ...groups.map((group) => ({
                        value: group.id,
                        label: group.name
                    }))]}
                    onChange={(groupId) => {
                        updateActive({groupId});
                        if (selectedGroup !== null && selectedGroup !== groupId) setSelectedGroup(groupId);
                    }}
                />
                    <button type="button" className="memo-delete-button" onClick={removeActive} aria-label="删除备忘"
                            title="删除备忘"><DeleteOutlined/></button>
                </div>
            </div>
        </> : <div className="memo-placeholder">
            <div>选择一条备忘开始编辑</div>
            <button onClick={createMemo}>新建备忘</button>
        </div>}</main>
        <Modal title="新建分组" open={creatingGroup} onCancel={() => setCreatingGroup(false)} footer={null} width={360}
               centered>
            <form className="memo-group-form" onSubmit={(event) => {
                event.preventDefault();
                createGroup();
            }}>
                <Input aria-label="分组名称" placeholder="分组名称" autoFocus maxLength={40} value={groupName}
                       onChange={(event) => {
                           setGroupName(event.target.value);
                           setGroupError("");
                       }} onKeyDown={(event) => {
                    if (event.key === "Enter" && event.nativeEvent.isComposing) event.preventDefault();
                }}/>
                {groupError && <div className="memo-group-error" role="alert">{groupError}</div>}
                <div className="memo-group-actions">
                    <button type="button" className="memo-group-cancel" onClick={() => setCreatingGroup(false)}>取消
                    </button>
                    <button type="submit" className="memo-group-submit">添加分组</button>
                </div>
            </form>
        </Modal>
    </div>;
}
