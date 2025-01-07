import './app.css';
import React, {lazy, Suspense} from 'react';
import {appWindow, LogicalPosition, LogicalSize} from '@tauri-apps/api/window';
import {useEffect, useRef, useState} from "react";
import {convertFileSrc, invoke} from "@tauri-apps/api/tauri";
import {listen} from '@tauri-apps/api/event';
import webImg from './assets/web.svg';
import baseComponent from './baseComponent';
import {useLocalStorage} from 'react-use';
import {getMaterialFileIcon, getMaterialFolderIcon} from "file-extension-icon-js";
import {
    calcComponent,
    pluginsComponent,
    // SubpageComponent,
    // TemplateComponent,
    webSearchComponent,
    calculateExpression,
    modifyWindowSize,
    getWindowPosition,
    loadCustomComponent,
    initAppHabitDB
} from "./template.jsx";

const SubpageComponent = React.lazy(() =>
    import("./template.jsx").then((mod) => ({default: mod.SubpageComponent}))
);
const TemplateComponent = React.lazy(() =>
    import("./template.jsx").then((mod) => ({default: mod.TemplateComponent}))
);

const App = () => {
    // 键入值
    const [inputValue, setInputValue] = useState('');
    const [searchOffset, setSearchOffset] = useState(0);
    // 组件结构
    const [component, setComponent] = useState('');
    // 手枪组件
    const [pistol, setPistol] = useState('');
    // 组件信息
    const [componentInfo, setComponentInfo] = useState('');
    // 搜索组件结果数组
    const [keywordComponent, setKeywordComponent] = useState([]);
    // 当前选中组件索引
    const [selectedIndex, setSelectedIndex] = useState(-1);
    // 输入状态判断
    const [isComposing, setIsComposing] = useState({status: false, ppos: 0});
    // 输入框组件
    const inputBox = useRef(null);
    const windowPosition = useRef(null);
    // 功能键状态
    const [fnDown, setFnDown] = useState(false);
    // 按下按键
    const [keyDown, setKeyDown] = useState(false);

    // 自制插件管理
    const [pluginStatus, setPluginStatus] = useLocalStorage("pluginStatus", {});
    // 自制插件列表
    const [pluginList, setPluginList] = useLocalStorage("pluginList", []);
    const [dbList, setDbList] = useLocalStorage("dbList", []);

    const [appDirectory, setAppDirectory] = useState({})

    const [insidePluginList, setInsidePluginList] = useState(pluginsComponent);

    const [appHabitDB, setAppHabitDB] = useState(null);

    const [actionParent, setActionParent] = useState({})

    function initStatus(components) {
        setPistol("");
        setSelectedIndex(-1);
        if (!components) {
            setInputValue("");
        } else {
            modifyWindowSize(components.length)
            setSelectedIndex(0)
        }
        setComponent(null);
        setComponentInfo("");
        setKeywordComponent(components || []);
        setIsComposing({status: false, ppos: 0});
        setFnDown(false);
    }


    async function handleKeyDown(event) {
        // 处理键盘按下
        setKeyDown(event)
        if (!event.metaKey && event.key === "Enter") {
            if (keywordComponent) {
                await confirmComponentSelected();
            }
        } else if (event.key === "Tab" && component) {
            // 当按下TAB键时，将焦点移动到下一个输入框
            event.preventDefault();
            let inputs = document.querySelectorAll("input");
            for (var i = 0; i < inputBox.length; i++) {
                if (document.activeElement === inputs[i]) {
                    break;
                }
            }
            if (i === inputBox.length - 1) {
                i = 0;
            } else {
                i++;
            }
            console.log(i);
            inputs[i].focus();
        } else if (event.key === "Escape" && isComposing.ppos === 0) {
            // 当按下ESC键时，清空输入框和组件
            if (inputValue) {
                setKeywordComponent([]);
                setInputValue("");
                if (!componentInfo.type === "subpage") {
                    await modifyWindowSize('small');
                }
            } else if (pistol) {
                setPistol("");
                setKeywordComponent([]);
                await modifyWindowSize('small');
            } else if (component) {
                setComponent(null);
                setComponentInfo("");
                await modifyWindowSize('small');
            } else {
                setComponent(null);
                setComponentInfo("");
                await modifyWindowSize('small');
            }
        } else if (event.code === "Tab" && (event.target.value === "" || event.target.value === " ") && !component) {
            // 当Tab被按下时，如果输入框为空，则进入文件搜索模式
            event.preventDefault();
            event.target.value = "";
            let icon = <img style={{height: "38px"}} {...insidePluginList.searchFileComponent.icon.props}/>
            setComponent(icon)
            setComponentInfo(insidePluginList.searchFileComponent);
        } else if (inputBox.current.value.length === 1 && event.key === "Backspace") {
            setKeywordComponent([]);
            if (!componentInfo.type === "subpage") {
                await modifyWindowSize('small');
            }
        } else if (inputBox.current.value.length === 0 && isComposing.ppos === 0 && event.key === "Backspace") {
            if (pistol) {
                setPistol("");
                setKeywordComponent([]);
                await modifyWindowSize('small');
            } else {
                setComponent(null);
                setComponentInfo("");
                setKeywordComponent([]);
                await modifyWindowSize('small');
            }
        } else if (event.key === "ArrowUp") {
            event.preventDefault();
            if (selectedIndex > 0) {
                setSelectedIndex(selectedIndex - 1);
            } else {
                setSelectedIndex(keywordComponent.length - 1);
            }
        } else if (event.key === "ArrowDown") {
            event.preventDefault();
            if (selectedIndex >= keywordComponent.length - 1) {
                setSelectedIndex(0);
            } else {
                setSelectedIndex(selectedIndex + 1);
            }
        } else if ((event.metaKey || event.altKey)) {
            console.log(event.metaKey, event.key);
            if (event.key === "Meta" || event.code === "AltLeft") {
                setFnDown(true);
            } else if (event.key === "Enter") {
                await confirmComponentSelected();
                setInputValue("");
            } else {
                try {
                    if (parseInt(event.key) <= 9 && parseInt(event.key) > 0) {
                        let list_items = document.getElementsByClassName("templateComponent")
                        let firstItem = 0;
                        for (let i = 0; i < list_items.length; i++) {
                            if (list_items[i].getBoundingClientRect().y === 60.5) {
                                firstItem = i
                                break
                            }
                        }
                        await confirmComponentSelected(firstItem + parseInt(event.key) - 1, false);
                    }
                } catch (e) {
                    console.log(e);
                }
            }
        }
        if (!isComposing.status && isComposing.ppos !== 0) {
            setIsComposing({status: false, ppos: 0});
        }
    }


    const getAppHabit = async (keyword) => {
        // 获取用户习惯热点度
        try {
            const res = await appHabitDB.getDataByKey(keyword);
            return JSON.parse(res[0]?.habitData || "{}");
        } catch (err) {
            return console.log('获取数据失败[appHabitDB]==>', err) || {};
        }
    };
    const updateAppHabit = async (keyword, appName) => {
        // 更新用户习惯热点度
        try {
            let habitData = await getAppHabit(keyword);
            if (!habitData[appName]) {
                habitData[appName] = 0;
            }
            habitData[appName] += 1;
            return appHabitDB.update({keyword, habitData: JSON.stringify(habitData)});
        } catch (err) {
            return console.log('更新数据失败[appHabitDB]==>', err);
        }
    };
    const initPoi = async () => {
        let window_position = await windowPosition.current
        await appWindow.setPosition(new LogicalPosition(window_position.x, window_position.y));
    };

    async function confirmComponentSelected(index, metaStatus) {
        // 组件确认选择后
        let currentComponent = keywordComponent[index !== undefined ? index : selectedIndex];
        setSelectedIndex(0);
        console.log(currentComponent)
        if (!currentComponent) return
        if (currentComponent.type === "component") {
            await updateAppHabit(inputValue, currentComponent.title);
            if (typeof currentComponent.icon == "string") {
                setComponent(<div className='activateComponent'
                                  data-tauri-drag-region>{currentComponent.icon.slice(0, 4)}</div>);
            } else {
                let icon = <img style={{height: "38px"}} {...currentComponent.icon.props}/>
                setComponent(icon);
            }
            setComponentInfo(currentComponent);
            setInputValue("");
            inputBox.current.focus();
        } else if (currentComponent.type === "subpage") {
            await updateAppHabit(inputValue, currentComponent.title);
            // 设置子页面的图标
            if (typeof currentComponent.icon == "string") {
                setComponent(<div className='activateComponent'
                                  data-tauri-drag-region>{currentComponent.icon.slice(0, 4)}</div>);
            } else {
                let icon = <img style={{height: "38px", width: "38px"}} {...currentComponent.icon.props}/>
                setComponent(icon);
            }
            setComponentInfo(currentComponent);
            setInputValue("");
            await modifyWindowSize("big");
        } else if (currentComponent.type === "result") {
            await appWindow.hide();
            await invoke("clipboard_control", {
                text: currentComponent.data.toString(),
                control: "write",
                paste: true,
                dataType: "text"
            });
        } else if (currentComponent.type === "app") {
            if (!fnDown || metaStatus === false) {
                await updateAppHabit(inputValue, currentComponent.title);
                console.log(currentComponent)
                await invoke("open_app", {appPath: currentComponent.data, appName: currentComponent.title});
                initStatus();
            } else {
                await invoke("open_explorer", {path: keywordComponent[selectedIndex].data});
            }
        } else if (currentComponent.type === "url") {
            console.log("打开网页");
            await invoke("open_url", {url: currentComponent.data});
        } else if (currentComponent.type === "search") {
            await invoke("open_url", {url: currentComponent.data});
        } else if (currentComponent.type === "action") {
            const handle = async (component) => {
                let scriptPath = component.data, scriptParams, result
                if (component.script) {
                    scriptPath = component.script.data
                    scriptParams = component.data || ""
                    component.action = component.script.action
                    component.params = component.script.params || {}
                }
                if (scriptPath && (scriptPath?.startsWith("./") || scriptPath?.[1] !== ":")) {
                    scriptPath = scriptPath.replace("./", "")
                    scriptPath = scriptPath[0] === "/" ? scriptPath.substring(1, scriptPath.length) : scriptPath
                    scriptPath = `${appDirectory['plugins']}/${component.pluginName}/${scriptPath}`;
                }
                try {
                    if (scriptPath) {
                        result = await baseComponent['action_' + component.action](scriptPath, scriptParams || inputValue.split(" "));
                    } else {
                        let input_value = scriptParams
                        if (!input_value) {
                            let kg_index = input_value.indexOf(" ") !== -1 ? input_value.indexOf(" ") : 0
                            input_value = inputValue.substring(kg_index + 1)
                        }
                        result = await baseComponent['action_' + component.action](input_value, component.params);
                    }
                } catch (e) {
                    let res = {
                        type: "result",
                        title: e.toString(),
                        desc: e,
                        icon: "ERROR",
                        data: e,
                    }
                    return initStatus([res])
                }
                if (!result) {
                    return initStatus()
                }
                result.data = JSON.parse(result.data)
                let items = []
                for (let info of result.data.items) {
                    let item = {
                        type: component.next?.type || "result",
                        title: info.title,
                        desc: info.subtitle,
                        icon: component.icon,
                        data: info.arg,
                        script: component.next
                    }
                    items.push(item)
                }
                initStatus(items);
                console.log(items)

            };
            handle(currentComponent)
        } else if (currentComponent.type === "file") {
            if (!fnDown) {
                await invoke("open_file", {filePath: currentComponent.data});
            } else {
                await invoke("open_explorer", {path: keywordComponent[selectedIndex].data});
            }
            initStatus();
        }
        setKeywordComponent([])
    }

    useEffect(() => {
        // 当组件信息改变时，重新聚焦输入框
        // 如果为小组件则将焦点聚焦在子页面输入框上
        if (componentInfo.type === "subpage") {
            let inputs = document.querySelectorAll("input");
            if (inputs[1]) {
                inputs[1].focus();
            } else {
                inputs[0].focus();
            }
        }
    }, [componentInfo]);

    useEffect(() => {
        // 文件拖放识别
        const fetchData = async () => {
            if (pistol.split(".").pop() === "py") {
                let result = await invoke("run_python_script", {scriptPath: pistol});
                if (result.success === "true") {
                    try {
                        let data = JSON.parse(result.data);
                        data = data.items;
                        await modifyWindowSize(data.length);
                        setKeywordComponent(data);
                        setSelectedIndex(0);
                    } catch (e) {
                        console.error(e);
                        setKeywordComponent([{
                            title: "Error",
                            type: "result",
                            icon: "E",
                            "data": e,
                            "desc": e.replace(/\n/g, "")
                        }]);
                        await modifyWindowSize(1);
                        setSelectedIndex(0);
                    }
                } else {
                    setKeywordComponent([{
                        title: "Error",
                        type: "result",
                        icon: "E",
                        "data": result.data,
                        "desc": result.data.replace(/\n/g, "")
                    }]);
                    await modifyWindowSize(1);
                    setSelectedIndex(0);
                }
            }
        };
        fetchData();
    }, [pistol]);

    useEffect(() => {
        // 输入框内容提交
        function calculator() {
            let calc_result = calculateExpression(inputValue);
            if (calc_result !== false) {
                return calcComponent(calc_result, inputValue);
            }
        }

        function isValidURL(url) {
            const urlPattern = /^(https?:\/\/)?(www\.)?((([0-9]{1,3}\.){3}[0-9]{1,3})|([a-zA-Z0-9-]+\.[a-zA-Z]{2,}))([a-zA-Z0-9\-._~:/?#[\]@!$&'()*+,;=%]*)$/;
            return urlPattern.test(url);
        }

        const fetchData = async () => {
            if (inputBox.current) {
                inputBox.current.value = inputValue;
            }
            if (inputValue === "-") {
                function deleteIndexedDB(dbName) {
                    return new Promise((resolve, reject) => {
                        const request = indexedDB.deleteDatabase(dbName);

                        request.onsuccess = () => {
                            console.log(`Database ${dbName} deleted successfully`);
                            resolve();
                        };

                        request.onerror = (event) => {
                            console.error(`Error deleting database ${dbName}:`, event.target.error);
                            reject(event.target.error);
                        };

                        request.onblocked = () => {
                            console.warn(`Database ${dbName} delete blocked`);
                        };
                    });
                }

                setDbList([]);
                return deleteIndexedDB("lark").then(() => {
                });
            }
            let searchType = "app";
            if (componentInfo?.title === "文件搜索") {
                searchType = "file";
            } else if (componentInfo?.type) {
                searchType = componentInfo.title;
            } else if (!componentInfo && pistol) {
                searchType = "pistol";
            }
            console.log("搜索类型", searchType);
            let result = [];
            // 输入有值且不在输入状态时,进行搜索
            if (inputValue.trim() && !isComposing.status) {
                if (inputValue.trim() === "reIndex") {
                    await invoke("create_file_index", {})
                    return
                }
                if (inputValue.trim() === "reApp") {
                    await invoke("create_app_index", {})
                    return
                }
                // 计算器组件，在没有选择组件时，尝试计算
                if (searchType === "app") {
                    let calc_result = calculator();
                    if (calc_result) {
                        result.push(calc_result);
                    }
                    // 判断输入的是不是网址
                    if (isValidURL(inputValue)) {
                        result.push(
                            {
                                title: inputValue,
                                type: "url",
                                icon: <img src={webImg}></img>,
                                "data": inputValue,
                                "desc": "使用默认浏览器打开url"
                            }
                        );
                        await modifyWindowSize(result.length || "small");
                    }
                }
                // 如果是搜索app时，尝试获取缓存
                let query_result;

                if (searchType === "file" && Date.now() - (window.searchFileCache[inputValue]?.time || 0) < 10000) {
                    query_result = window.searchFileCache[inputValue]?.data || [];
                } else {
                    query_result = await invoke("search_keyword", {
                        componentName: componentInfo?.title || "",
                        inputValue,
                        offset: searchOffset,
                        params: {}
                    });

                }
                const pinyinMatches = [];
                const otherMatches = [];

                //* 匹配内部插件
                if (searchType === "app") {
                    for (let pluginName in insidePluginList) {
                        let plugin = insidePluginList[pluginName];
                        if (plugin.title.startsWith(inputValue) || plugin.desc.startsWith(inputValue)) {
                            result.push(plugin);
                        }
                    }
                    // 匹配自定义插件组件
                    for (let pluginName in pluginList) {
                        if (pluginStatus[pluginName]?.enable === false) {
                            continue;
                        }
                        let plugin = pluginList[pluginName];
                        let workflows = plugin.workflow;
                        for (let workflow of workflows) {
                            if (workflow.keyword?.startsWith(inputValue)) {
                                // workflow.type = "action";
                                workflow.pluginName = plugin.title
                                if (typeof workflow.icon !== "object") {
                                    let icon, src
                                    if (workflow.icon?.startsWith("./")) {
                                        icon = workflow.icon.replace("./", "")
                                    } else {
                                        icon = workflow.icon || ""
                                        icon = icon[0] === "/" ? icon.substring(1, icon.length) : icon
                                    }
                                    if (icon[1] === ":") {
                                        src = convertFileSrc(icon)
                                    } else {
                                        src = convertFileSrc(`${appDirectory['plugins']}/${pluginName}/${icon}`)
                                    }
                                    workflow.icon =
                                        <img alt={"i"} src={src} className={"activateComponent"}
                                             data-tauri-drag-region/>
                                    workflow.parent = pluginName
                                }
                                result.push(workflow);
                            }
                        }

                    }
                }
                //* 匹配搜索结果
                try {
                    for (let item of query_result) {
                        if (item.title !== "" || item.File !== undefined) {
                            if (searchType === "app") {
                                item = item.File;
                                if (typeof item.icon === "string") {
                                    item.icon =
                                        <img src={`data:image/png;base64,${item.icon}`} style={{width: "100%"}}></img>;
                                }
                                item.data = item.path;
                                item.type = "app";
                                result.push(item);
                            }
                            if (searchType === "file") {
                                item = item.File;
                                item.data = item.path;
                                item.desc = item.path;
                                item.type = "file";
                                if (item.file_type === "folder") {
                                    item.icon =
                                        <img src={getMaterialFolderIcon(item.file_type)} style={{width: "100%"}}></img>;
                                } else {
                                    item.icon =
                                        <img src={getMaterialFileIcon(item.file_type)} style={{width: "100%"}}></img>;
                                }
                                result.push(item);
                            }
                        }
                    }

                    // 对匹配项进行排序
                    // 排序拼音匹配项
                    pinyinMatches.sort((a, b) => a.index - b.index);
                    // 排序其他匹配项
                    otherMatches.sort((a, b) => a.index - b.index);
                    // 合并结果
                    // result = [...result, ...pinyinMatches.map(match => match.item), ...otherMatches.map(match => match.item)].slice(0, 9);

                    if (result.length === 0 && searchType === "app") {
                        // 没有结果则进行web搜索
                        result = webSearchComponent(inputValue);
                    } else {
                        // 有结果 则判断关键词在appHabit中的热度，根据热度再排序。其中在这个关键词下每启动一次该app，则热度+1
                        if (searchType === "app") {
                            let habit = await getAppHabit(inputValue);
                            result.sort((a, b) => (habit[b.title] || 0) - (habit[a.title] || 0));
                        }
                    }

                } catch (e) {
                    console.log("错误:", e)
                }
                // 当前组件类型不为小窗组件时改变窗口大小
                if (componentInfo.type !== "subpage") {
                    await modifyWindowSize(result.length || "small");
                }

            } else if (componentInfo.type !== "subpage") {
                await modifyWindowSize('small');
            }
            console.log(result)
            setKeywordComponent(result);
            setSelectedIndex(0);
        };

        fetchData();

    }, [inputValue]);

    useEffect(() => {
        // 监听窗口失去焦点 隐藏窗口
        let updateCacheTime;
        appWindow.onFocusChanged(event => {
            console.log("当前组件的信息", componentInfo);
            if (event.payload === false && componentInfo.type !== "subpage") {
                const hideWindow = async () => {
                    await appWindow.hide();
                    await modifyWindowSize("small");
                };
                // ? 正式启用
                // hideWindow().then(initCache);
            }
        });

        const unListenFocusChanged = listen('window-focus', event => {
            if (event.payload === true) {
                initStatus();
                initPoi();
                appWindow.show();
                appWindow.setFocus();
                inputBox.current.focus();
            }

        });


        const unListenFileDrop = listen('tauri://file-drop', event => {
            const {payload} = event;
            if (Array.isArray(payload) && payload.length > 0) {
                setPistol(payload[0]);
                if (!componentInfo || componentInfo.type != "subpage") {
                    inputBox.current.focus();
                }
            }
        });

        const unListenFileIndex = listen('file_index_count', event => {
            const {payload} = event;
            console.log(payload)
        });


        // 输入框获取焦点
        inputBox.current.focus();

        initAppHabitDB(appHabitDB, setAppHabitDB, dbList, setDbList);

        // 读取本地组件库，查看注册状态
        loadCustomComponent().then((result) => {
                setPluginList(result);
            }
        )
        if (!windowPosition.current) {
            windowPosition.current = getWindowPosition();
        }


        window.searchFileCache = {};
        invoke("get_app_dir", {}).then((result) => {
            setAppDirectory(result)
        })

        return () => {
            unListenFocusChanged.then((f) => f());
            unListenFileDrop.then((f) => f());
            unListenFileIndex.then((f) => f());
        };

    }, []);

    useEffect(() => {
        initAppHabitDB(appHabitDB, setAppHabitDB, dbList, setDbList);
    }, [appHabitDB])
    return (
        <div id="mainDiv" data-tauri-drag-region>
            <div style={{width: "100%", height: "51.5px", margin_bottom: "5px"}}>
                <div style={{
                    width: "100%",
                    height: "100%",
                    display: "flex",
                    justifyContent: "colum",
                    alignItems: "center"
                }}>
                    {!component ? <div/> : component}
                    {!pistol ? <div/> : <div className='pistol' onDoubleClick={() => {
                        setPistol("");
                    }}><p className='pistolText'>{pistol.split("/").pop()}</p></div>}
                    <input ref={inputBox} type="text" id='mainInput' autoCorrect="off" spellCheck="false"
                           onChange={(event) => {
                               if (!isComposing.status) {
                                   setInputValue(event.target.value);
                               }
                           }}
                           onKeyDown={handleKeyDown}
                           onKeyUp={() => {
                               setFnDown(false)
                           }}
                           onCompositionStart={() => {
                               setIsComposing({status: true, ppos: 0})
                           }}
                           onCompositionEnd={(event) => {
                               setIsComposing({status: false, ppos: 1});
                               setInputValue(event.target.value);
                           }}/>
                </div>
                {/*{keywordComponent ? TemplateComponent(keywordComponent, selectedIndex, setSelectedIndex, confirmComponentSelected, fnDown) : null}*/}
                {keywordComponent ?
                    <Suspense fallback={<div>Loading...</div>}>
                        <TemplateComponent {...{
                            components: keywordComponent,
                            selectedKey: selectedIndex,
                            setSelectedKey: setSelectedIndex,
                            confirmComponentSelected,
                            fnDown
                        }}/>
                    </Suspense>
                    : null}
                {componentInfo ? <SubpageComponent component={componentInfo} keyDown={keyDown}/> : null}
            </div>
        </div>
    );
};

export default App;
