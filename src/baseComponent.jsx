import {invoke} from "@tauri-apps/api";

async function open_app(app_path, app_name) {
    await invoke("open_app", {appPath: app_path, appName: app_name});
}


async function open_url(url) {
    await invoke("open_url", {url});
}

async function action_runScript(script_path, params) {
    console.log("run script", script_path, params)
    if (script_path.endsWith("py")) {
        return await invoke("run_python_script", {scriptPath: script_path, params: params || []});
    }
}


async function action_readClipboard() {
    return await invoke("clipboard_control", {text: "", control: "read", paste: false});
}


async function action_writeClipboard(content, kwargs) {
    if (!content) return
    await invoke("clipboard_control", {
        text: content,
        control: "write",
        paste: kwargs.paste || false,
        dataType: kwargs.type || "text"
    });
}

async function action_result(text, paste) {
    await action_writeClipboard(text, paste);
}

async function action_writeFile(file_path, text) {
    console.log("write file", file_path, text);
    await invoke("write_txt", {filePath: file_path, text});
}

export async function action_readFile(file_path) {
    return await invoke("read_txt", {filePath: file_path});
}

async function action_rebuildFileIndex() {
    return await invoke("rebuild_index", {});
}

async function action_rebuildAppIndex() {
    return await invoke("create_app_index", {})
}

export default {
    action_openApp: open_app,
    action_openUrl: open_url,
    action_runScript,
    action_readClipboard,
    action_writeClipboard,
    action_writeFile,
    action_readFile,
    action_rebuildFileIndex,
    action_rebuildAppIndex,
    action_result,
};