import React, {useCallback, useEffect, useRef, useState} from "react";
import fileImg from "./assets/file.svg";
import calcImg from "./assets/calc.svg";
import settingImg from "./assets/setting.svg";
import clipboardImg from "./assets/clipboard.svg";
import componentImg from "./assets/component.svg";
import rebuildImg from "./assets/rebuild.svg";
import {evaluate} from "mathjs";
import {appWindow, LogicalSize} from "@tauri-apps/api/window";
import {IndexDBCache} from "./indexedDB.jsx";
import throttle from "lodash/throttle.js";

const TemplateComponent = (components, selectedKey, setSelectedKey, confirmComponentSelected, fnDown) => {
    const scrollContainerRef = useRef(null);
    const [firstItemIndex, setFirstItemIndex] = useState(0);

    const handleMouseEnter = (index) => {
        setSelectedKey(index);
    };
    const handleMouseLeave = () => {
        setSelectedKey(-1);
    };
    const displayDesc = (component, selected) => {
        if (fnDown && selected && (component.type === "app" || component.type === "file")) {
            return "Reveal file in Finder";
        } else {
            return component.desc?.replace(/\n/g, " ");
        }
    };
    let size = components.length > 9 ? 9 : components.length;
    useEffect(() => {
        const scrollContainer = scrollContainerRef?.current;
        const selectedItem = scrollContainer?.querySelector(`[data-index="${selectedKey}"]`);
        if (selectedItem) {
            selectedItem.scrollIntoView({
                behavior: 'smooth',
                block: 'nearest'
            });
        }
    }, [selectedKey])

    const closeDefault = useCallback(throttle((deltaY) => {
            // 滚动事件限制触发频率，并固定滚动距离
            const scrollContainer = scrollContainerRef.current;
            if (deltaY > 5) {
                scrollContainer.scrollTop = scrollContainer.scrollTop + 50;
            } else if (deltaY < -5) {
                scrollContainer.scrollTop = scrollContainer.scrollTop - 50;
            }
        }, 100)
        , [selectedKey])

    const handleScroll = (e) => {
        // 滚动事件处理，禁用原有滚动行为，并触发自定义滚动逻辑
        e.preventDefault();
        e.stopPropagation();
        closeDefault(e.deltaY)
    }
    useEffect(() => {
        if (components && scrollContainerRef.current) {
            scrollContainerRef.current.addEventListener('wheel', handleScroll, {passive: false})
        } else {
            scrollContainerRef?.current?.removeEventListener("wheel", handleScroll)
        }
    }, [components])

    function get_first_item() {
        let list_items = document.getElementsByClassName("templateComponent")
        for (let i = 0; i < list_items.length; i++) {
            if (list_items[i].getBoundingClientRect().y === 60.5) {
                return i
            }
        }
    }

    return (
        <div style={{position: "relative"}}>
            {
                size !== 0 ? <>
                    <div style={{height: "450px", overflowY: "scroll"}}
                         ref={scrollContainerRef}
                    >
                        {
                            components.map((component, index) => (
                                <div className={`templateComponent ${selectedKey === index ? 'activate' : ''}`}
                                     key={index}
                                     data-index={index}
                                     onMouseEnter={() => handleMouseEnter(index)}
                                     onMouseLeave={handleMouseLeave}
                                     onClick={() => {
                                         confirmComponentSelected();
                                     }}
                                >
                                    {
                                        typeof (component.icon) == "string" ?
                                            <div className="templateBaseIcon">
                                                {component.icon.slice(0, 3)}
                                            </div> :
                                            <div className='templateImgIcon'>
                                                {component.icon}
                                            </div>
                                    }
                                    <div style={{
                                        display: "flex",
                                        justifyContent: "space-between",
                                        alignItems: "center",
                                        width: "100%"
                                    }}>
                                        <div className="templateContent">
                                            <div className="templateTitle">
                                                {component.title}
                                            </div>
                                            <div className="templateDesc">
                                                {displayDesc(component, selectedKey === index)}
                                            </div>
                                        </div>

                                    </div>

                                </div>
                            ))
                        }

                    </div>
                    <div>
                        {
                            new Array(size).fill('').map((component, index) => (
                                <div className='templateHint' key={"hint" + index}
                                     style={{top: 3.5 + 11.1 * index + "%"}}>
                                    {selectedKey - get_first_item() === index ? "⏎" : "⌘" + (index + 1)}
                                </div>
                            ))
                        }

                    </div>
                </> : <></>
            }
        </div>
    );
}

function SubpageComponent({component, keyDown}) {
    const [RenderComponent, setRenderComponent] = useState(false);
    useEffect(() => {
        const loadDynamicComponent = async () => {
            const module = await import(`./panels/${component.data}.jsx`);
            setRenderComponent(() => module.default);
        }
        if (component?.type === "subpage" && component.data) {
            console.log("更新子页面：", component.data)
            loadDynamicComponent()
        } else {
            setRenderComponent(false)
        }

    }, [component])

    let subpageStyle = {
        backgroundColor: "#d8d8d7",
        height: "100%",
        borderRadius: "10px",
        borderWidth: "0px",
        overflow: 'hidden'
    }
    return (
        <>
            <div id="subPageFrame"
                 style={component?.type === "subpage" ? {height: "calc(100vh - 75px)", marginTop: "5px"} : {}}>
                {RenderComponent ? <div style={subpageStyle}><RenderComponent onKeyDown={keyDown}/></div> : <div/>}
            </div>
        </>
    );
}

function webSearchComponent(inputValue) {
    return [
        {
            title: `谷歌："${inputValue}"`,
            icon: <img src='/Google.svg' style={{width: "100%"}}></img>,
            data: "https://www.google.com/search?q=" + inputValue,
            type: "url",
        },
        {

            title: `百度："${inputValue}"`,
            icon: <img src='/baidu.svg' style={{width: "100%"}}></img>,
            data: "https://www.baidu.com/s?wd=" + inputValue,
            type: "url",
        },
        {
            title: `必应："${inputValue}"`,
            icon: <img src="/bing.svg" style={{width: "100%"}}></img>,
            data: "https://cn.bing.com/search?q=" + inputValue,
            type: "url",
        },
        {
            title: `哔哩哔哩："${inputValue}"`,
            icon: <img src='/bilibili.svg' style={{width: "100%"}}></img>,
            data: "https://search.bilibili.com/all?keyword=" + inputValue,
            type: "url",
        },
        {
            title: `淘宝:"${inputValue}"`,
            icon: <img src='/taobao.svg' style={{width: "100%"}}></img>,
            data: "https://s.taobao.com/search?q=" + inputValue,
            type: "url",
        },
        {
            title: `京东:"${inputValue}"`,
            icon: <img src='/jd.svg' style={{width: "100%"}}></img>,
            data: "https://search.jd.com/Search?keyword=" + inputValue,
            type: "url",
        }

    ];
}


function activeStyle(ss) {
    if (ss) {
        return ""
    } else {
        return "activateComponent"
    }
}

const searchFileComponent = {
    icon: <img src={fileImg} alt="file" className='activateComponent' data-tauri-drag-region/>,
    title: '文件搜索',
    desc: 'search file',
    type: "component",
};
const showPluginComponent = {
    icon: <img src={componentImg} alt="component" className='activateComponent' data-tauri-drag-region/>,
    title: '组件库',
    desc: 'show component',
    type: "subpage"
};
const settingPluginComponent = {
    icon: <img src={settingImg} alt="setting" className='activateComponent' data-tauri-drag-region/>,
    title: '应用设置',
    desc: 'app setting',
    type: "subpage",
    data: "settingComponent"
};
const clipboardPluginComponent = {
    icon: <img src={clipboardImg} alt="clipboard" className='activateComponent' data-tauri-drag-region/>,
    title: '剪贴板',
    desc: 'clipboard',
    type: "subpage",
    data: "clipboardComponent"
};

const FileIndexComponent = {
    icon: <img src={rebuildImg} alt="file" className='activateComponent' data-tauri-drag-region/>,
    title: '重建索引',
    desc: 'Rebuild Index',
    type: "action",
    action: "rebuildIndex",
};

const calcComponent = (result, input) => {
    return {
        title: result || "计算器",
        type: "result",
        icon: <img src={calcImg}></img>,
        data: result || "0",
        desc: input.replace(/\n/g, "")
    };
};
const pluginsComponent = {
    searchFileComponent,
    showPluginComponent,
    settingPluginComponent,
    clipboardPluginComponent,
    FileIndexComponent
}

function calculateExpression(expression) {
    try {
        const result = evaluate(expression);
        if (parseInt(result) || parseFloat(result)) {
            return result;
        }
        return false;
    } catch (error) {
        return false;
    }

}


const db_app_params = {
    dbName: "lark",
    cacheTableName: "appCache",
    keyPath: "title",
    indexs: [
        {name: 'title', unique: true},
        {name: 'icon', unique: false},
        {name: 'iconPath', unique: false},
        {name: 'desc', unique: false},
        {name: 'data', unique: false},
        {name: 'type', unique: false},
    ]
};
const db_app_habit_params = {
    dbName: "lark",
    cacheTableName: "appHabit",
    keyPath: "keyword",
    indexs: [
        {name: 'keyword', unique: true},
        {name: 'habitData', unique: false},
    ]
};

const modifyWindowSize = async (size) => {
    if (size === "big") {
        size = new LogicalSize(718, 600);
        document.getElementById("mainDiv").style.height = (size.height * 0.97) + "px";
    } else if (size === "small") {
        size = new LogicalSize(718, 71);
        document.getElementById("mainDiv").style.height = (size.height * 0.74) + "px";
    } else {
        size > 9 ? size = 9 : size < 1 ? size = 1 : size;
        let amend = [0, 5, 7, 8, 8, 8.5, 8.5, 9, 9]
        document.getElementById("mainDiv").style.height = ((1 + size) * 50) + "px";
        size = new LogicalSize(718, 78 + size * (40 + amend[size - 1]));

    }
    try {
        await appWindow.setSize(size);
    } catch (e) {
        console.error('Failed to resize window:', e);
    }
};
const getWindowPosition = async () => {
    const factor = await appWindow.scaleFactor();
    const position = await appWindow.innerPosition();
    const logical = position.toLogical(factor);
    return {x: logical.x, y: logical.y};
};

// 读取本地组件库，查看注册状态
const loadCustomComponent = async () => {
    const files = {};
    const importAll = import.meta.glob('./components/*.json');
    for (const path in importAll) {
        const module = await importAll[path]();
        const fileName = path.replace('./components/', '').replace(".json", "");
        files[fileName] = module;
    }
    return files;
};

const initIndexDB = async (db, version, dbList, setDbList) => {
    await db.initDB(version).then(res => {
        if (res.type === 'upgradeneeded') {
            console.log(db._cacheTableName + ' 数据库创建或更新成功!', res.target.result.objectStoreNames !== dbList);
            if (Object.values(res.target.result.objectStoreNames) !== dbList) {
                setDbList(Object.values(res.target.result.objectStoreNames));
            }
        } else {
            console.log(db._cacheTableName + ' 数据库初始化成功!', res);
        }
    }).catch((err) => {
        console.log(db._cacheTableName + ' 数据库初始化失败! ', err.target.error);
    });
};

function initAppDB(appDB, setAppDB, appHabitDB, setAppHabitDB, dbList, setDbList) {
    if (appDB && !appDB._db) {
        let version = dbList.includes(appDB._cacheTableName) ? dbList.length : dbList.length + 1;
        initIndexDB(appDB, version, dbList, setDbList).then(() => {
            if (!appHabitDB) {
                setTimeout(() => {
                    setAppHabitDB(new IndexDBCache(db_app_habit_params));
                }, 1000);
            }
        });
    } else if (!appDB) {
        setAppDB(new IndexDBCache(db_app_params));
    }
}

function initAppHabitDB(appDB, setAppDB, appHabitDB, dbList, setDbList) {
    if (appHabitDB && !appHabitDB._db) {
        let version = dbList.includes(appHabitDB._cacheTableName) ? dbList.length : dbList.length + 1;
        if (appDB && appDB._dbversion < version) {
            appDB.closeDB();
        }
        initIndexDB(appHabitDB, version, dbList, setDbList).then(() => {
            if (!appDB) {
                setTimeout(() => {
                    setAppDB(new IndexDBCache(db_app_params));
                }, 1000);
            }
        });
    }
}


export {
    TemplateComponent,
    SubpageComponent,
    pluginsComponent,
    calcComponent,
    webSearchComponent,
    calculateExpression,
    modifyWindowSize,
    getWindowPosition,
    loadCustomComponent,
    initAppDB,
    initAppHabitDB
};