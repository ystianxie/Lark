const DB_NAME = "larkTodo";
const STORE_NAME = "todoItems";
let databasePromise;

function openDatabase() {
    if (databasePromise) return databasePromise;
    databasePromise = new Promise((resolve, reject) => {
        const request = indexedDB.open(DB_NAME);
        request.onupgradeneeded = () => {
            const db = request.result;
            if (!db.objectStoreNames.contains(STORE_NAME)) {
                const store = db.createObjectStore(STORE_NAME, {keyPath: "id"});
                store.createIndex("createdAt", "createdAt", {unique: false});
                store.createIndex("completed", "completed", {unique: false});
            }
        };
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error || new Error("待办数据库初始化失败"));
        request.onblocked = () => reject(new Error("待办数据库被其他页面占用，请关闭其他 Lark 窗口后重试"));
    }).catch((error) => {
        databasePromise = undefined;
        throw error;
    });
    return databasePromise;
}

function normalizeTask(task) {
    return {
        id: String(task?.id || ""),
        title: String(task?.title || "").trim(),
        note: String(task?.note || ""),
        completed: task?.completed === true,
        createdAt: Number(task?.createdAt) || 0,
        updatedAt: Number(task?.updatedAt) || Number(task?.createdAt) || 0,
        completedAt: task?.completedAt == null ? null : Number(task.completedAt) || null,
    };
}

function transactionRequest(mode, callback) {
    return openDatabase().then((db) => new Promise((resolve, reject) => {
        const transaction = db.transaction(STORE_NAME, mode);
        const request = callback(transaction.objectStore(STORE_NAME));
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error || new Error("待办数据操作失败"));
        transaction.onerror = () => reject(transaction.error || new Error("待办数据操作失败"));
    }));
}

export async function listTodoItems() {
    const items = await transactionRequest("readonly", (store) => store.getAll());
    return items.map(normalizeTask);
}

export async function addTodoItem({title, note}) {
    const now = Date.now();
    const task = normalizeTask({
        id: globalThis.crypto?.randomUUID?.() || `${now}-${Math.random().toString(36).slice(2)}`,
        title,
        note,
        completed: false,
        createdAt: now,
        updatedAt: now,
        completedAt: null,
    });
    if (!task.title) throw new Error("标题不能为空");
    await transactionRequest("readwrite", (store) => store.add(task));
    return task;
}

export async function updateTodoItem(task) {
    const normalized = normalizeTask({...task, updatedAt: Date.now()});
    if (!normalized.id || !normalized.title) throw new Error("标题不能为空");
    await transactionRequest("readwrite", (store) => store.put(normalized));
    return normalized;
}

export async function deleteTodoItem(id) {
    return transactionRequest("readwrite", (store) => store.delete(id));
}
