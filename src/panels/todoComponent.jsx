import React, {useEffect, useMemo, useRef, useState} from "react";
import {Empty, Modal, Popconfirm, Select, Spin} from "antd";
import {
    addTodoGroup,
    deleteTodoGroup,
    listTodoGroups,
    addTodoItem,
    deleteTodoItem,
    listTodoItems,
    updateTodoItem,
} from "../todoStorage.js";
import "./todoComponent.css";

const FILTERS = [
    {key: "all", label: "全部"},
    {key: "active", label: "未完成"},
    {key: "completed", label: "已完成"},
];

function sortTasks(tasks) {
    return [...tasks].sort((left, right) => {
        if (left.completed !== right.completed) return left.completed ? 1 : -1;
        return (right.updatedAt || right.createdAt) - (left.updatedAt || left.createdAt);
    });
}

function getErrorMessage(error) {
    return error instanceof Error ? error.message : String(error || "操作失败，请稍后重试");
}

function formatCreatedAt(timestamp) {
    if (!timestamp) return "";
    const date = new Date(timestamp);
    if (Number.isNaN(date.getTime())) return "";
    const now = new Date();
    const sameYear = date.getFullYear() === now.getFullYear();
    const weekdays = ["日", "一", "二", "三", "四", "五", "六"];
    const month = date.getMonth() + 1;
    const day = date.getDate();
    const hours = String(date.getHours()).padStart(2, "0");
    const minutes = String(date.getMinutes()).padStart(2, "0");
    const dateLabel = sameYear ? `${month}/${day}` : `${date.getFullYear()}/${month}/${day}`;
    return `${dateLabel} 周${weekdays[date.getDay()]} ${hours}:${minutes}`;
}

function TaskEditor({
                        groups,
                        initialGroupId = "",
                        initialTitle,
                        initialNote,
                        submitLabel,
                        onSubmit,
                        onCancel,
                        onEscape,
                        disabled,
                        clearAfterSubmit = false
                    }) {
    const [title, setTitle] = useState(initialTitle);
    const [note, setNote] = useState(initialNote);
    const [groupId, setGroupId] = useState(initialGroupId);
    const [validationError, setValidationError] = useState("");
    const titleRef = useRef(null);

    useEffect(() => {
        titleRef.current?.focus();
    }, []);

    const submit = async (event) => {
        event?.preventDefault();
        const cleanTitle = title.trim();
        if (!cleanTitle) {
            setValidationError("请输入待办标题");
            titleRef.current?.focus();
            return;
        }
        setValidationError("");
        const saved = await onSubmit({title: cleanTitle, note, groupId});
        if (saved && clearAfterSubmit) {
            setTitle("");
            setNote("");
        }
    };

    const handleEscape = (event) => {
        if (event.key !== "Escape" || event.nativeEvent.isComposing) return;
        event.preventDefault();
        if (onCancel) {
            onCancel();
        } else if (title || note) {
            setTitle("");
            setNote("");
            setValidationError("");
        } else {
            onEscape?.();
        }
    };

    const handleTitleKeyDown = (event) => {
        handleEscape(event);
        if (event.defaultPrevented) return;
        if (event.key === "Enter" && !event.nativeEvent.isComposing) {
            event.preventDefault();
            void submit(event);
        }
    };

    return (
        <form className="todo-editor" onSubmit={submit}>
            <input
                ref={titleRef}
                className="todo-title-input"
                value={title}
                onChange={(event) => setTitle(event.target.value)}
                onKeyDown={handleTitleKeyDown}
                placeholder="要做什么？"
                aria-label="待办标题"
                disabled={disabled}
            />
            <textarea
                className="todo-note-input"
                value={note}
                onChange={(event) => setNote(event.target.value)}
                onKeyDown={handleEscape}
                placeholder="备注（可选）"
                aria-label="待办备注"
                rows={2}
                disabled={disabled}
            />
            <Select
                className="todo-group-select"
                aria-label="待办分组"
                value={groupId}
                disabled={disabled}
                options={[{value: "", label: "未分组"}, ...groups.map((group) => ({
                    value: group.id,
                    label: group.name
                }))]}
                onChange={setGroupId}
            />
            {validationError && <div className="todo-field-error">{validationError}</div>}
            <div className="todo-editor-actions">
                <button className="todo-primary-button" type="submit" disabled={disabled}>
                    {disabled ? "保存中…" : submitLabel}
                </button>
                {onCancel && (
                    <button className="todo-secondary-button" type="button" onClick={onCancel} disabled={disabled}>
                        取消
                    </button>
                )}
            </div>
        </form>
    );
}

function EditIcon() {
    return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path d="M4 20h4l10.5-10.5a2.83 2.83 0 0 0-4-4L4 16v4Z"/>
            <path d="m13.5 6.5 4 4"/>
        </svg>
    );
}

function DeleteIcon() {
    return (
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path d="M4 7h16"/>
            <path d="M9 7V4h6v3"/>
            <path d="m6 7 1 13h10l1-13"/>
            <path d="M10 11v5M14 11v5"/>
        </svg>
    );
}

function TodoTaskRow({groups, task, onToggle, onEdit, onDelete, busy}) {
    const [editing, setEditing] = useState(false);

    if (editing) {
        return (
            <li className="todo-task todo-task-editing">
                <TaskEditor
                    groups={groups}
                    initialGroupId={task.groupId}
                    initialTitle={task.title}
                    initialNote={task.note}
                    submitLabel="保存"
                    disabled={busy}
                    onCancel={() => setEditing(false)}
                    onSubmit={async (draft) => {
                        const saved = await onEdit(task, draft);
                        if (saved) setEditing(false);
                        return saved;
                    }}
                />
            </li>
        );
    }

    return (
        <li className={`todo-task${task.completed ? " is-completed" : ""}`}>
            <button
                type="button"
                className="todo-check-button"
                onClick={() => onToggle(task)}
                disabled={busy}
                aria-label={task.completed ? "恢复未完成" : "标记为已完成"}
                aria-pressed={task.completed}
            >
                {task.completed ? "✓" : ""}
            </button>
            <div className="todo-task-content">
                <div className="todo-task-title">{task.title}</div>
                {task.note && <div className="todo-task-note" title={task.note}>{task.note}</div>}
            </div>
            <div className="todo-task-actions">
                <time className="todo-task-created"
                      dateTime={task.createdAt ? new Date(task.createdAt).toISOString() : undefined}>
                    {formatCreatedAt(task.createdAt)}
                </time>
                <button
                    type="button"
                    className="todo-icon-button"
                    onClick={() => setEditing(true)}
                    disabled={busy}
                    aria-label="编辑待办"
                    title="编辑"
                >
                    <EditIcon/>
                </button>
                <Popconfirm
                    title="删除这个待办？"
                    description="删除后无法恢复。"
                    okText="删除"
                    cancelText="取消"
                    onConfirm={() => onDelete(task)}
                >
                    <button
                        type="button"
                        className="todo-icon-button todo-delete-button"
                        disabled={busy}
                        aria-label="删除待办"
                        title="删除"
                    >
                        <DeleteIcon/>
                    </button>
                </Popconfirm>
            </div>
        </li>
    );
}

export default function TodoComponent({onClose}) {
    const [tasks, setTasks] = useState([]);
    const [groups, setGroups] = useState([]);
    const [selectedGroup, setSelectedGroup] = useState(null);
    const [creatingGroup, setCreatingGroup] = useState(false);
    const [groupName, setGroupName] = useState("");
    const [groupError, setGroupError] = useState("");
    const mutationRef = useRef(false);

    const refreshData = async () => {
        const [items, savedGroups] = await Promise.all([listTodoItems(), listTodoGroups()]);
        setTasks(sortTasks(items));
        setGroups(savedGroups);
    };
    const [filter, setFilter] = useState("all");
    const [query, setQuery] = useState("");
    const [creating, setCreating] = useState(false);
    const [loading, setLoading] = useState(true);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState("");

    const loadTasks = async () => {
        setLoading(true);
        setError("");
        try {
            await refreshData();
        } catch (loadError) {
            setError(getErrorMessage(loadError));
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        void loadTasks();
    }, []);

    const runMutation = async (mutation) => {
        if (mutationRef.current) return false;
        mutationRef.current = true;
        setBusy(true);
        setError("");
        try {
            await mutation();
            await refreshData();
            return true;
        } catch (mutationError) {
            setError(getErrorMessage(mutationError));
            return false;
        } finally {
            mutationRef.current = false;
            setBusy(false);
        }
    };

    const groupedTasks = useMemo(() => tasks.filter((task) => selectedGroup === null || task.groupId === selectedGroup), [tasks, selectedGroup]);
    const visibleTasks = useMemo(() => groupedTasks.filter((task) => {
        if (filter === "active") return !task.completed;
        if (filter === "completed") return task.completed;
        return true;
    }).filter((task) => {
        const keyword = query.trim().toLowerCase();
        return !keyword || `${task.title} ${task.note}`.toLowerCase().includes(keyword);
    }), [groupedTasks, filter, query]);

    const activeCount = tasks.filter((task) => !task.completed).length;
    const completedTasks = groupedTasks.filter((task) => task.completed);
    const completedCount = completedTasks.length;

    return (
        <div className="todo-page">
            <div className="todo-header">
                <div className="todo-heading-row">
                    <h1>待办</h1>
                    <span className="todo-count">{activeCount} 未完成 · 共 {tasks.length} 项</span>
                </div>
                <div className="todo-header-actions">
                    <button type="button" className="todo-create-button" disabled={busy || loading}
                            onClick={() => setCreating(true)}>+ 新建
                    </button>
                    {onClose && <button type="button" className="todo-close-button" onClick={onClose}>关闭</button>}
                </div>
            </div>

            <div className="todo-groups" aria-label="待办分组">
                {[{id: null, name: "全部分组"}, {id: "", name: "未分组"}, ...groups].map((group) => (
                    <div className={`todo-group-tab${selectedGroup === group.id ? " is-active" : ""}`}
                         key={group.id ?? "all-groups"}>
                        <button type="button" className="todo-filter-button" aria-pressed={selectedGroup === group.id}
                                onClick={() => setSelectedGroup(group.id)}>{group.name}</button>
                        {group.id && (
                            <Popconfirm title={`删除分组“${group.name}”？`}
                                        description="其中的待办会移至未分组，不会被删除。" okText="删除" cancelText="取消"
                                        onConfirm={() => runMutation(async () => {
                                            await deleteTodoGroup(group.id);
                                            if (selectedGroup === group.id) setSelectedGroup("");
                                        })}>
                                <button type="button" className="todo-group-delete"
                                        aria-label={`删除分组 ${group.name}`} title="删除分组"
                                        disabled={busy || loading}>×
                                </button>
                            </Popconfirm>
                        )}
                    </div>
                ))}
                <button type="button" className="todo-filter-button todo-add-group" disabled={busy || loading}
                        onClick={() => {
                            setGroupName("");
                            setGroupError("");
                            setCreatingGroup(true);
                        }}>+
                </button>
            </div>

            <div className="todo-toolbar">
                <div className="todo-filters" role="tablist" aria-label="待办筛选">
                    {FILTERS.map((item) => (
                        <button
                            key={item.key}
                            type="button"
                            role="tab"
                            aria-selected={filter === item.key}
                            className={`todo-filter-button${filter === item.key ? " is-active" : ""}`}
                            onClick={() => setFilter(item.key)}
                        >
                            {item.label}
                        </button>
                    ))}
                </div>
                <div className="todo-search-wrap">
                    <input
                        className="todo-search-input"
                        value={query}
                        onChange={(event) => setQuery(event.target.value)}
                        placeholder="搜索待办…"
                        aria-label="搜索待办"
                    />
                    {query && <button type="button" className="todo-search-clear" onClick={() => setQuery("")}
                                      aria-label="清除搜索">×</button>}
                </div>
                {completedCount > 0 && (
                    <Popconfirm
                        title={`清除 ${completedCount} 项已完成待办？`}
                        description="删除后无法恢复。"
                        okText="清除"
                        cancelText="取消"
                        onConfirm={() => runMutation(async () => {
                            await Promise.all(completedTasks.map((task) => deleteTodoItem(task.id)));
                        })}
                    >
                        <button type="button" className="todo-clear-button" disabled={busy}>清除已完成</button>
                    </Popconfirm>
                )}
            </div>

            {error && (
                <div className="todo-error" role="alert">
                    <span>{error}</span>
                    <button type="button" onClick={() => void loadTasks()}>重试</button>
                </div>
            )}

            {loading ? (
                <div className="todo-state"><Spin size="small"/><span>正在加载待办…</span></div>
            ) : visibleTasks.length === 0 ? (
                <div className="todo-empty">
                    <Empty image={Empty.PRESENTED_IMAGE_SIMPLE}
                           description={query ? "没有匹配的待办" : filter === "completed" ? "还没有已完成的待办" : filter === "active" ? "所有待办都完成了" : "还没有待办"}/>
                </div>
            ) : (
                <ul className="todo-list">
                    {visibleTasks.map((task) => (
                        <TodoTaskRow
                            key={task.id}
                            groups={groups}
                            task={task}
                            busy={busy}
                            onToggle={(item) => runMutation(() => updateTodoItem({
                                ...item,
                                completed: !item.completed,
                                completedAt: item.completed ? null : Date.now(),
                            }))}
                            onEdit={(item, draft) => runMutation(() => updateTodoItem({...item, ...draft}))}
                            onDelete={(item) => runMutation(() => deleteTodoItem(item.id))}
                        />
                    ))}
                </ul>
            )}

            <Modal className="todo-create-modal" title="新建分组" open={creatingGroup} footer={null} width={360}
                   centered onCancel={() => {
                if (!busy) setCreatingGroup(false);
            }}>
                <form className="todo-editor" onSubmit={async (event) => {
                    event.preventDefault();
                    if (!groupName.trim()) {
                        setGroupError("请输入分组名称");
                        return;
                    }
                    setGroupError("");
                    const saved = await runMutation(async () => {
                        const group = await addTodoGroup(groupName);
                        setSelectedGroup(group.id);
                    });
                    if (saved) setCreatingGroup(false);
                }}>
                    <input className="todo-title-input" aria-label="分组名称" placeholder="分组名称" maxLength={40}
                           autoFocus value={groupName} disabled={busy}
                           onChange={(event) => setGroupName(event.target.value)} onKeyDown={(event) => {
                        if (event.key === "Enter" && event.nativeEvent.isComposing) event.preventDefault();
                    }}/>
                    {groupError && <div className="todo-field-error">{groupError}</div>}
                    {error && <div className="todo-field-error" role="alert">{error}</div>}
                    <div className="todo-editor-actions">
                        <button type="submit" className="todo-primary-button"
                                disabled={busy}>{busy ? "保存中…" : "添加分组"}</button>
                        <button type="button" className="todo-secondary-button" disabled={busy}
                                onClick={() => setCreatingGroup(false)}>取消
                        </button>
                    </div>
                </form>
            </Modal>

            <Modal
                className="todo-create-modal"
                title="新建待办"
                open={creating}
                footer={null}
                width={520}
                centered
                onCancel={() => setCreating(false)}
            >
                {creating && (
                    <TaskEditor
                        groups={groups}
                        initialGroupId={selectedGroup || ""}
                        initialTitle=""
                        initialNote=""
                        submitLabel="添加待办"
                        disabled={busy}
                        clearAfterSubmit
                        onCancel={() => setCreating(false)}
                        onSubmit={async (draft) => {
                            const saved = await runMutation(() => addTodoItem(draft));
                            if (saved) setCreating(false);
                            return saved;
                        }}
                    />
                )}
            </Modal>
        </div>
    );
}
