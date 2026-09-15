import { invoke, path } from "@tauri-apps/api";
import { match } from "pinyin-pro";

// 生成 SHA-1 摘要
const digest = async plaintext => {
    const uint8 = new TextEncoder().encode(plaintext);
    const buffer = await window.crypto.subtle.digest("SHA-1", uint8);
    const array = Array.from(new Uint8Array(buffer));
    const hex = array.map(b => b.toString(16).padStart(2, "0")).join("");
    return hex;
};

// 扫描应用
export const scan = async () => {
    const apps = [];

    for (const app of await invoke('scan_apps')) {
        apps.push({
            name: app.name,
            path: app.path,
            target: app.target,
            description: app.target,
            type: 'app',
            icon: await invoke('get_icon', { path: app.target })
        });
    }

    return apps;
};

// 从配置文件中读取所有的插件并分解出功能模块
export const read = async () => {
    // 获取配置文件路径
    console.log("Reading config file...")
    const conf_path = await path.resolve(
        await path.documentDir(), "Gizmo", "config.json"
    );
    // 从配置文件中读取插件列表
    const plugin_list = JSON.parse(
        await invoke("read_text_file", {
            path: conf_path
        })
    );

    // 存储功能列表
    const features = [];

    // 遍历插件分解功能模块
    for (const plugin of plugin_list.plugins) {
        // 读取插件 manifest.json 文件
        const manifest = JSON.parse(
            await invoke("read_text_file", {
                path: await path.resolve(
                    plugin, "manifest.json"
                )
            })
        );

        const pid = await digest(`${manifest.title}###${manifest.description}`);

        // 解析功能
        for (const feature of manifest.features) {
            const fid = await digest(`${feature.name}###${feature.description}`);

            // 构造功能数据解构
            const item = {
                pid: pid,
                fid: fid,
                path: plugin,
                name: feature.name,
                description: feature.description,
                type: feature.type,
                mode: feature.mode,
                icon: feature.icon,
                params: feature.params,
            };

            // 解析图标
            if (typeof feature.icon === "string") {
                if (/^(https?:\/\/|data:image)/.test(feature.icon)) {
                    item.icon = feature.icon;
                } else {
                    item.icon = await path.resolve(plugin, feature.icon);
                };
            };

            // 解析静态页
            if (typeof feature.index === "string") {
                if (/https?:\/\//.test(feature.index)) {
                    item.index = feature.index;
                } else {
                    item.index = await path.resolve(plugin, feature.index);
                };
            };

            // 解析脚本
            // let script_list = [];
            // if (typeof feature.script === "string") {
            //     script_list = [feature.script];
            // } else if (Array.isArray(feature.script)) {
            //     script_list = [...feature.script];
            // };
            // for (const index in script_list) {
            //     if (!/https?:\/\//.test(script_list[index])) {
            //         script_list[index] = await path.resolve(plugin.path, script_list[index]);
            //     };
            // };
            // item.script = script_list;
            if (typeof feature.script === "string") {
                if (/https?:\/\//.test(feature.script)) {
                    item.script = feature.script;
                } else {
                    item.script = await path.resolve(plugin, feature.script);
                };
            };

            // 解析样式
            // let style_list = [];
            // if (typeof feature.style === "string") {
            //     style_list = [feature.style];
            // } else if (Array.isArray(feature.style)) {
            //     style_list = [...feature.style];
            // };
            // for (const index in style_list) {
            //     if (!/https?:\/\//.test(style_list[index])) {
            //         style_list[index] = await path.resolve(plugin.path, style_list[index]);
            //     };
            // };
            // item.style = style_list;
            if (typeof feature.style === "string") {
                if (/https?:\/\//.test(feature.style)) {
                    item.style = feature.style;
                } else {
                    item.style = await path.resolve(plugin, feature.style);
                };
            };

            features.push(item);
        };
    };

    return features;
};

// 过滤功能模块
export function* filter(text, file, features) {
    // 输入区域没有文件和文本内容时，不返回任何功能
    if (!text && !file) {
        return [];
    };

    // 根据 mode 参数，在输入区域有文件的情况下过滤出支持文件的功能列表，不然则过滤出 mode 中不含文件模式的功能列表
    if (file) {
        features = features.filter(feature => feature.mode?.includes('f'));
    } else {
        features = features.filter(feature => !feature.mode?.includes('f'));
    };

    for (const feature of features) {
        // 输入区域有文件并且文本内容为空时返回当前所有的功能过滤列表
        if (file && !text) {
            yield Object.assign({ file: file }, feature);
            continue;
        };

        // 从当前过滤列表中返回文本命中标题的功能列表
        if (match(feature.name, text)) {
            yield Object.assign({ file: file }, feature);
            continue;
        };
    };
};