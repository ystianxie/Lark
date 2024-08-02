import React, {useEffect, useState} from "react";
import fileImg from "./assets/file.svg";
import calcImg from "./assets/calc.svg";
import {evaluate} from "mathjs";
import {appWindow, LogicalSize} from "@tauri-apps/api/window";
import {IndexDBCache} from "./indexedDB.jsx";

const TemplateComponent = (components, selectedKey, setSelectedKey, confirmComponentSelected, fnDown) => {
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
    return (
        <>
            {
                components.map((component, index) => (
                    <div className={`templateComponent ${selectedKey === index ? 'activate' : ''}`}
                         key={index}
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
                            <div className='templateHint'>
                                {selectedKey === index ? "⏎" : "⌘" + (index + 1)}
                            </div>
                        </div>

                    </div>
                ))
            }
        </>
    );
}

function SubpageComponent({component}) {
    const [RenderComponent, setRenderComponent] = useState(false);
    useEffect(() => {
        const loadDynamicComponent = async () => {
            const module = await import(`./panels/${component.data}.jsx`);
            setRenderComponent(() => module.default);
        }
        if (component?.type === "subpage") {
            console.log("更新子页面：", component.data)
            loadDynamicComponent()
        }

    }, [component])
    let subpageStyle = {
        backgroundColor: "#d8d8d7",
        height: "100%",
        borderRadius: "10px",
        borderWidth: "0px"
    }
    return (
        <>
            <div>

                {RenderComponent ? <div style={subpageStyle}><RenderComponent/></div> :
                    <div/>}
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


const searchFileComponent = {
    icon: <img src={fileImg} alt="file" className='activateComponent' data-tauri-drag-region/>,
    title: '搜索文件',
    desc: 'search file',
    type: "component"
};
const showPluginComponent = {
    icon: <img src={fileImg} alt="file" className='activateComponent' data-tauri-drag-region/>,
    title: '显示组件',
    desc: 'show component',
    type: "subpage"
};
const settingPluginComponent = {
    icon: <img src={fileImg} alt="file" className='activateComponent' data-tauri-drag-region/>,
    title: '应用设置',
    desc: 'app setting',
    type: "subpage",
    data: "settingComponent"
};
const clipboardPluginComponent = {
    icon: <img src={fileImg} alt="file" className='activateComponent' data-tauri-drag-region/>,
    title: '剪贴板',
    desc: 'clipboard',
    type: "subpage"
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
    clipboardPluginComponent
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
        document.getElementById("mainDiv").style.height = ((1 + size) * 50) + "px";
        size = new LogicalSize(718, 78 + size * 49);
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