import React, {useEffect, useMemo, useRef, useState} from "react";
import {Empty, Popconfirm, Spin} from "antd";
import {
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
        return right.createdAt - left.createdAt;
    });
}

function getErrorMessage(error) {
    return error instanceof Error ? error.message : String(error || "操作失败，请稍后重试");
}

function TaskEditor({initialTitle, initialNote, submitLabel, onSubmit, onCancel, onEscape, disabled, clearAfterSubmit = false}) {
    const [title, setTitle] = useState(initialTitle);
    const [note, setNote] = useState(initialNote);
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
        const saved = await onSubmit({title: cleanTitle, note});
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

function TodoTaskRow({task, onToggle, onEdit, onDelete, busy}) {
    const [editing, setEditing] = useState(false);

    if (editing) {
        return (
            <li className="todo-task todo-task-editing">
                <TaskEditor
                    initialTitle={task.title}
                    initialNote={task.note}
                    submitLabel="保存"
                    disabled={busy}
                    onCancel={() => setEditing(false)}
                    onSubmit={async (draft) => {
                        const saved = await onEdit(task, draft);
                        if (saved) setEditing(false);
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
                <button type="button" className="todo-icon-button" onClick={() => setEditing(true)} disabled={busy}>
                    编辑
                </button>
                <Popconfirm
                    title="删除这个待办？"
                    description="删除后无法恢复。"
                    okText="删除"
                    cancelText="取消"
                    onConfirm={() => onDelete(task)}
                >
                    <button type="button" className="todo-icon-button todo-delete-button" disabled={busy}>
                        删除
                    </button>
                </Popconfirm>
            </div>
        </li>
    );
}

export default function TodoComponent({onClose}) {
    const [tasks, setTasks] = useState([]);
    const [filter, setFilter] = useState("all");
    const [loading, setLoading] = useState(true);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState("");

    const loadTasks = async () => {
        setLoading(true);
        setError("");
        try {
            setTasks(sortTasks(await listTodoItems()));
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
        setBusy(true);
        setError("");
        try {
            await mutation();
            setTasks(sortTasks(await listTodoItems()));
            return true;
        } catch (mutationError) {
            setError(getErrorMessage(mutationError));
            return false;
        } finally {
            setBusy(false);
        }
    };

    const visibleTasks = useMemo(() => tasks.filter((task) => {
        if (filter === "active") return !task.completed;
        if (filter === "completed") return task.completed;
        return true;
    }), [tasks, filter]);

    const activeCount = tasks.filter((task) => !task.completed).length;

    return (
        <div className="todo-page">
            <div className="todo-header">
                <div>
                    <div className="todo-heading-row">
                        <h1>待办</h1>
                        <span className="todo-count">{activeCount} 项未完成</span>
                    </div>
                    <p>把要做的事情记下来，逐项完成。</p>
                </div>
                {onClose && <button type="button" className="todo-close-button" onClick={onClose}>关闭</button>}
            </div>

            <TaskEditor
                initialTitle=""
                initialNote=""
                submitLabel="添加待办"
                disabled={busy}
                clearAfterSubmit
                onEscape={onClose}
                onSubmit={(draft) => runMutation(async () => {
                    await addTodoItem(draft);
                })}
            />

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
                <span className="todo-total-count">共 {tasks.length} 项</span>
            </div>

            {error && (
                <div className="todo-error" role="alert">
                    <span>{error}</span>
                    <button type="button" onClick={() => void loadTasks()}>重试</button>
                </div>
            )}

            {loading ? (
                <div className="todo-state"><Spin size="small" /><span>正在加载待办…</span></div>
            ) : visibleTasks.length === 0 ? (
                <div className="todo-empty">
                    <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={filter === "completed" ? "还没有已完成的待办" : "还没有待办"} />
                </div>
            ) : (
                <ul className="todo-list">
                    {visibleTasks.map((task) => (
                        <TodoTaskRow
                            key={task.id}
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
        </div>
    );
}
