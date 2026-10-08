const DB_NAME = "larkTodo";
const STORE_NAME = "todoItems";
const GROUP_STORE_NAME = "todoGroups";
let databasePromise;

function openDatabase() {
    if (databasePromise) return databasePromise;
    databasePromise = new Promise((resolve, reject) => {
        const request = indexedDB.open(DB_NAME, 2);
        request.onupgradeneeded = () => {
            const db = request.result;
            if (!db.objectStoreNames.contains(GROUP_STORE_NAME)) {
                db.createObjectStore(GROUP_STORE_NAME, {keyPath: "id"});
            }
            if (!db.objectStoreNames.contains(STORE_NAME)) {
                const store = db.createObjectStore(STORE_NAME, {keyPath: "id"});
                store.createIndex("createdAt", "createdAt", {unique: false});
                store.createIndex("completed", "completed", {unique: false});
            }
        };
        request.onsuccess = () => {
            const db = request.result;
            db.onversionchange = () => {
                db.close();
                databasePromise = undefined;
            };
            resolve(db);
        };
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
        groupId: String(task?.groupId || ""),
        completed: task?.completed === true,
        createdAt: Number(task?.createdAt) || 0,
        updatedAt: Number(task?.updatedAt) || Number(task?.createdAt) || 0,
        completedAt: task?.completedAt == null ? null : Number(task.completedAt) || null,
    };
}

function transactionRequest(mode, callback, storeName = STORE_NAME) {
    return openDatabase().then((db) => new Promise((resolve, reject) => {
        const transaction = db.transaction(storeName, mode);
        const request = callback(transaction.objectStore(storeName));
        transaction.oncomplete = () => resolve(request.result);
        transaction.onabort = () => reject(transaction.error || new Error("待办数据操作失败"));
        request.onerror = () => reject(request.error || new Error("待办数据操作失败"));
        transaction.onerror = () => reject(transaction.error || new Error("待办数据操作失败"));
    }));
}

export async function listTodoItems() {
    const items = await transactionRequest("readonly", (store) => store.getAll());
    return items.map(normalizeTask);
}

export async function addTodoItem({title, note, groupId = ""}) {
    const now = Date.now();
    const task = normalizeTask({
        id: globalThis.crypto?.randomUUID?.() || `${now}-${Math.random().toString(36).slice(2)}`,
        title,
        note,
        groupId,
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

export async function listTodoGroups() {
    const groups = await transactionRequest("readonly", (store) => store.getAll(), GROUP_STORE_NAME);
    return groups.sort((left, right) => left.createdAt - right.createdAt);
}

export async function addTodoGroup(name) {
    const cleanName = name.trim();
    if (!cleanName) throw new Error("请输入分组名称");
    const now = Date.now();
    const group = {
        id: globalThis.crypto?.randomUUID?.() || `${now}-${Math.random().toString(36).slice(2)}`,
        name: cleanName,
        createdAt: now,
    };
    await openDatabase().then((db) => new Promise((resolve, reject) => {
        const transaction = db.transaction(GROUP_STORE_NAME, "readwrite");
        const store = transaction.objectStore(GROUP_STORE_NAME);
        let duplicate = false;
        const request = store.getAll();
        request.onsuccess = () => {
            duplicate = request.result.some((item) => item.name === cleanName);
            if (duplicate) transaction.abort();
            else store.add(group);
        };
        transaction.oncomplete = resolve;
        transaction.onabort = () => reject(duplicate ? new Error("分组名称已存在") : transaction.error || new Error("分组保存失败"));
    }));
    return group;
}

export async function deleteTodoGroup(id) {
    await openDatabase().then((db) => new Promise((resolve, reject) => {
        const transaction = db.transaction([STORE_NAME, GROUP_STORE_NAME], "readwrite");
        const store = transaction.objectStore(STORE_NAME);
        const request = store.openCursor();
        request.onsuccess = () => {
            const cursor = request.result;
            if (!cursor) return;
            if (cursor.value.groupId === id) cursor.update({...cursor.value, groupId: ""});
            cursor.continue();
        };
        transaction.objectStore(GROUP_STORE_NAME).delete(id);
        transaction.oncomplete = resolve;
        transaction.onabort = () => reject(transaction.error || new Error("分组删除失败"));
    }));
}
