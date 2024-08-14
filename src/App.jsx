import './app.css';
import React, {Suspense} from 'react';
import {appWindow, LogicalPosition, LogicalSize} from '@tauri-apps/api/window';
import {useEffect, useRef, useState} from "react";
import {invoke} from "@tauri-apps/api/tauri";
import {listen} from '@tauri-apps/api/event';
import webImg from './assets/web.svg';
import systemAppName from "./assets/system_app_name.json";
import {match} from 'pinyin-pro';
import baseComponent from './baseComponent';
import {useLocalStorage} from 'react-use';
import {getMaterialFileIcon, getMaterialFolderIcon} from "file-extension-icon-js";
import {
    calcComponent,
    pluginsComponent,
    SubpageComponent,
    TemplateComponent,
    webSearchComponent,
    calculateExpression,
    modifyWindowSize,
    getWindowPosition,
    loadCustomComponent,
    initAppDB, initAppHabitDB
} from "./template.jsx";
import {divide} from "mathjs";


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
    const isListening = useRef(false);
    const windowPosition = useRef(null);
    // 组件缓存
    const [componentCache, setComponentCache] = useState({});
    // app缓存
    const [appCache, setAppCache] = useState([]);
    // 功能键状态
    const [fnDown, setFnDown] = useState(false);
    // 按下按键
    const [keyDown, setKeyDown] = useState(false);

    // 自制插件管理
    const [pluginStatus, setPluginStatus] = useLocalStorage("pluginStatus", {});
    // 自制插件列表
    const [pluginList, setPluginList] = useLocalStorage("pluginList", []);
    const [dbList, setDbList] = useLocalStorage("dbList", []);


    const [insidePluginList, setInsidePluginList] = useState(pluginsComponent);


    const [appDB, setAppDB] = useState(null);
    const [appHabitDB, setAppHabitDB] = useState(null);


    function initStatus() {
        setPistol("");
        setInputValue("");
        setComponent(null);
        setComponentInfo("");
        setKeywordComponent([]);
        setSelectedIndex(-1);
        setIsComposing({status: false, ppos: 0});
        setFnDown(false);
    }


    async function handleKeyDown(event) {
        // 处理
        setKeyDown(event)
        if (!event.metaKey && event.key === "Enter") {
            if (keywordComponent) {
                await confirmComponentSelected();
            }
        } else if (event.key === "Tab") {
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
        } else if (event.code === "Space" && (event.target.value === "" || event.target.value === " ") && !component) {
            // 当空格键被按下时，如果输入框为空，则进入文件搜索模式
            console.log("space");
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
        } else if ((event.metaKey || event.ctrlKey)) {
            console.log(event.metaKey, event.key);
            if (event.key === "Meta") {
                setFnDown(true);
            } else if (event.key === "Enter") {
                await confirmComponentSelected();
                setInputValue("");
            } else {
                try {
                    if (parseInt(event.key) <= keywordComponent.length && parseInt(event.key) > 0) {
                        await confirmComponentSelected(parseInt(event.key) - 1);
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
        await appWindow.setPosition(new LogicalPosition(windowPosition.current.x, windowPosition.current.y));
    };

    async function confirmComponentSelected(index) {
        // 组件确认选择后
        let currentComponent = keywordComponent[index !== undefined ? index : selectedIndex];
        setSelectedIndex(0);
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
            if (typeof currentComponent.icon == "string") {
                setComponent(<div className='activateComponent'
                                  data-tauri-drag-region>{currentComponent.icon.slice(0, 4)}</div>);
            } else {
                let icon = <img style={{height: "38px"}} {...currentComponent.icon.props}/>
                setComponent(icon);
            }
            setComponentInfo(currentComponent);
            setInputValue("");
            await modifyWindowSize("big");
        } else if (currentComponent.type === "result") {
            await invoke("set_window_hide_macos", {});
            await invoke("clipboard_control", {text: currentComponent.data.toString(), control: "copy", paste: true});
        } else if (currentComponent.type === "app") {
            if (!fnDown) {
                await updateAppHabit(inputValue, currentComponent.title);
                await invoke("open_app", {appPath: currentComponent.data});
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
                let result = await baseComponent['action_' + component.action](component.data);
                if (Object.prototype.toString.call(component.next) === '[object Object]') {
                    component.next = [component.next];
                }
                for (let child of component.next) {
                    if (child.resolve === result.resolve) {
                        await handle(child);
                    }
                }
            };
            handle(currentComponent);
            initStatus();
        } else if (currentComponent.type === "file") {
            if (!fnDown) {
                await invoke("open_file", {filePath: currentComponent.data});
            } else {
                await invoke("open_explorer", {path: keywordComponent[selectedIndex].data});
            }

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

        const getDbAppByName = async (appName) => {
            try {
                const res = await appDB.getDataByKey(appName);
                return res[0];
            } catch (err) {
                console.log('获取数据失败[appCacheDb]==>', err);
                return null;
            }
        };
        const addDbAppItem = (appItem) => {
            try {
                appDB.update(appItem);
            } catch (err) {
                console.log('更新数据失败[appCacheDb]==>', err);
            }
        };
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
                console.log(window.searchFileCache);
                let query_result;
                if (searchType === "app" && appCache.length !== 0) {
                    query_result = appCache;
                } else if (searchType === "file" && Date.now() - (window.searchFileCache[inputValue]?.time || 0) < 10000) {
                    query_result = window.searchFileCache[inputValue]?.data || [];
                } else {
                    query_result = await invoke("search_keyword", {
                        componentName: componentInfo?.title || "",
                        inputValue,
                        offset: searchOffset,
                        params:{}
                    });
                    if (searchType === "app") {
                        setAppCache(query_result);
                    } else if (searchType === "file") {
                        window.searchFileCache[inputValue] = {
                            time: Date.now(),
                            data: query_result
                        };
                        console.log(window.searchFileCache);

                    }
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
                        let plugin = pluginList[pluginName];
                        let workflows = plugin.workflow;
                        for (let workflow of workflows) {
                            if (workflow.keyword.startsWith(inputValue)) {
                                workflow.type = "action";
                                result.push(workflow);
                            }
                        }

                    }
                }
                //* 匹配搜索结果
                try {
                    for (let item of query_result) {
                        if (item.title !== "") {
                            if (searchType === "app") {
                                // 对于图标为icns的组件进行base64编码
                                if (systemAppName[item.title]) {
                                    item.title = systemAppName[item.title];
                                }
                                let match_key = "";
                                let is_match = match(item.title, inputValue, {precision: 'start'});
                                if (!is_match) {
                                    let have_index = item.title.indexOf(inputValue);
                                    if (have_index != -1) {
                                        match_key = have_index;
                                        is_match = true;
                                    }
                                } else {
                                    match_key = "py";
                                }
                                if (!is_match) {
                                    let have_index = item.data.toLowerCase().indexOf(inputValue.toLowerCase());
                                    if (have_index != -1) {
                                        match_key = have_index;
                                        is_match = true;
                                    }
                                }

                                if (is_match) {
                                    let item_ = await getDbAppByName(item.title);
                                    if (componentCache[item.title] && componentCache[item.title].data === item.data) {
                                        item = componentCache[item.title];
                                    } else if (searchType === "app" && item_?.data === item.data) {
                                        item = item_;
                                        item.icon = <img src={`data:image/png;base64,${item.icon}`}
                                                         style={{width: "100%"}}></img>;
                                    } else if (item.type === "app" && item.icon) {
                                        await invoke("append_txt", {
                                            filePath: "/Users/starsxu/Documents/test2.txt",
                                            text: item.icon + "\n"
                                        });
                                        let icon_ = await invoke("read_app_info", {appPath: item.icon});
                                        if (icon_ !== "文件不存在！") {
                                            item.icon = icon_;
                                            item.iconPath = icon_;
                                        } else {
                                            icon_ = await invoke("get_file_icon", {filePath: item.data});
                                            if (icon_ !== "文件不存在！") {
                                                item.icon = icon_;
                                                addDbAppItem(item);
                                                item.icon = <img src={`data:image/png;base64,${icon_}`}
                                                                 style={{width: "100%"}}></img>;
                                            }
                                        }
                                        if (typeof item.icon == "string" && item.icon.indexOf(".icns") !== -1) {
                                            invoke('read_icns_to_base64', {path: item.icon})
                                                .then((base64) => {
                                                    item.icon = base64;
                                                    addDbAppItem(item);
                                                    item.icon = <img src={`data:image/png;base64,${base64}`}
                                                                     style={{width: "100%"}}></img>;
                                                })
                                                .catch((error) => {
                                                    console.error('Error reading file:', item.icon);
                                                });
                                        }
                                    }

                                    if (match_key === "py") {
                                        pinyinMatches.push({item, index: is_match[0]});
                                    } else {
                                        otherMatches.push({item, index: match_key});
                                    }
                                    if (!componentCache[item.title]) {
                                        setComponentCache(prevCache => ({
                                            ...prevCache,
                                            [item.title]: item,
                                        }));
                                    }
                                }
                                if (result.length >= 10) {
                                    break;
                                }
                            }
                            if (searchType === "file") {
                                item = item.File;
                                item.data = item.path;
                                item.desc = item.path;
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
                    console.log(pinyinMatches, otherMatches);
                    // 合并结果
                    result = [...result, ...pinyinMatches.map(match => match.item), ...otherMatches.map(match => match.item)].slice(0, 10);

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
                    console.log(result);
                } catch (e) {
                    await invoke("append_txt", {
                        filePath: "/Users/starsxu/Documents/test.txt",
                        text: JSON.stringify(e) + "\n"
                    });
                }
                // 当前组件类型不为小窗组件时改变窗口大小
                if (componentInfo.type !== "subpage") {
                    await modifyWindowSize(result.length || "small");
                }

            } else if (componentInfo.type !== "subpage") {
                await modifyWindowSize('small');
            }
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

        initAppDB(appDB, setAppDB)

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
        // 初始化AppCache
        const initCache = async () => {
            if (!updateCacheTime) {
                updateCacheTime = setTimeout(async () => {
                    let query_result = await invoke("search_keyword", {componentName: "", inputValue,offset:searchOffset});
                    setAppCache(query_result);
                    updateCacheTime = null;
                }, 3000);
            }
        };
        initCache();


        // 读取本地组件库，查看注册状态
        loadCustomComponent().then((result) => {
                setPluginList(result);
            }
        )
        if (!windowPosition.current) {
            windowPosition.current = getWindowPosition();
        }

        window.searchFileCache = {};


        return () => {
            unListenFocusChanged.then((f) => f());
            unListenFileDrop.then((f) => f());
            unListenFileIndex.then((f) => f());
        };

    }, []);


    useEffect(() => {
        // 初始化本地缓存数据库链接
        initAppDB(appDB, setAppDB, appHabitDB, setAppHabitDB, dbList, setDbList);
    }, [appDB]);

    useEffect(() => {
        initAppHabitDB(appDB, setAppDB, appHabitDB, dbList, setDbList);
    }, [appHabitDB]);

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
                {TemplateComponent(keywordComponent, selectedIndex, setSelectedIndex, confirmComponentSelected, fnDown)}
                <SubpageComponent component={componentInfo} keyDown={keyDown}/>
            </div>
        </div>
    );
};

export default App;
