import { invoke } from "@tauri-apps/api";

async function action_openApp(app_path) {
    await invoke("open_app", { appPath: app_path });
}


async function action_openUrl(url) {
    await invoke("open_url", { url });
}

async function action_runScript(script_path, params) {
    if (script_path.endsWitch("py")) {
        return await invoke("run_python_script", { script_path, params });
    }
}


async function action_readClipboard() {
    return await invoke("clipboard_control", { text: "", control: "read", paste: false });
}


async function action_writeClipboard(text, paste) {
    await invoke("clipboard_control", { text, control: "write", paste });
}

async function action_result(text, paste) {
    await action_writeClipboard(text, paste);
}

async function action_writeFile(file_path, text) {
    console.log("write file", file_path, text);
    await invoke("write_txt", { filePath: file_path, text });
}

async function action_readFile(file_path) {
    return await invoke("read_txt", { filePath: file_path });
}

export default {
    action_openApp,
    action_openUrl,
    action_runScript,
    action_readClipboard,
    action_writeClipboard,
    action_writeFile,
    action_readFile,
    action_result
};