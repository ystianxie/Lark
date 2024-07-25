import './App.css';
import {appWindow, LogicalPosition, LogicalSize} from '@tauri-apps/api/window';
import {useEffect, useRef, useState} from "react";
import {invoke} from "@tauri-apps/api/tauri";
import {listen} from '@tauri-apps/api/event';
import fileImg from './assets/file.svg';
import calcImg from './assets/calc.svg';
import webImg from './assets/web.svg';
import systemAppName from "./assets/system_app_name.json";
import {evaluate} from 'mathjs';
import {match} from 'pinyin-pro';
import {IndexDBCache} from "./indexedDB";
import baseComponent from './baseComponent';
import {useLocalStorage} from 'react-use';
import {getMaterialFileIcon} from "file-extension-icon-js";


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


function templateComponent(components, selectedKey, setSelectedKey, confirmComponentSelected, fnDown) {

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
                <div className='templateImgIcon' >
                  {component.icon}
                </div>
            }
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", width: "100%" }}>
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

function subpageComponent(component) {
  return (
    <>
      {
        !component || component?.type !== "subpage" ? <div></div> : <div className="subpageComponent">
          <div className="subpageContent">
            <input type='text'></input>

          </div>
        </div>
      }
    </>
  );
}



const App = () => {
  // 键入值
  const [inputValue, setInputValue] = useState('');
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
  const [isComposing, setIsComposing] = useState({ status: false, ppos: 0 });
  // 输入框组件
  const inputBox = useRef(null);
  const isListening = useRef(false);
  const windowPosition = useRef(null);
  // 组件缓存 
  const [componentCache, setComponentCache] = useState({});
  // app缓存
  const [appCache, setAppCache] = useState([]);
  // 功能键状态
  const [fnDown, setfnDown] = useState(false);
  // 自制插件管理
  const [pluginStatus, setPluginStatus] = useLocalStorage("pluginStatus", {});
  // 自制插件列表
  const [pluginList, setPluginList] = useLocalStorage("pluginList", []);
  const [dbList, setDbList] = useLocalStorage("dbList", []);

  const searchFileComponent = {
    icon: <img src={fileImg} alt="file" className='activateComponent' data-tauri-drag-region />,
    title: '搜索文件',
    desc: 'search file',
    type: "component"
  };
  const showPluginComponent = {
    icon: <img src={fileImg} alt="file" className='activateComponent' data-tauri-drag-region />,
    title: '显示组件',
    desc: 'show component',
    type: "subpage"
  };


  const [insidePluginList, setInsidePluginList] = useState({
    searchFileComponent,
    showPluginComponent
  });

  const calcComponent = (result) => {
    return {
      title: result || "计算器",
      type: "result",
      icon: <img src={calcImg}></img>,
      data: result || "0",
      desc: inputValue.replace(/\n/g, "")
    };
  };
  function webSearchComponent() {
    return [{
      title: `谷歌："${inputValue}"`,
      icon: <img src='/Google.svg' style={{ width: "100%" }}></img>,
      data: "https://www.google.com/search?q=" + inputValue,
      type: "url",
    },
    {

      title: `百度："${inputValue}"`,
      icon: <img src='/baidu.svg' style={{ width: "100%" }}></img>,
      data: "https://www.baidu.com/s?wd=" + inputValue,
      type: "url",
    },
    {
      title: `必应："${inputValue}"`,
      icon: <img src="/bing.svg" style={{ width: "100%" }}></img>,
      data: "https://cn.bing.com/search?q=" + inputValue,
      type: "url",
    },
    {
      title: `哔哩哔哩："${inputValue}"`,
      icon: <img src='/bilibili.svg' style={{ width: "100%" }}></img>,
      data: "https://search.bilibili.com/all?keyword=" + inputValue,
      type: "url",
    },
    {
      title: `淘宝:"${inputValue}"`,
      icon: <img src='/taobao.svg' style={{ width: "100%" }}></img>,
      data: "https://s.taobao.com/search?q=" + inputValue,
      type: "url",
    },
    {
      title: `京东:"${inputValue}"`,
      icon: <img src='/jd.svg' style={{ width: "100%" }}></img>,
      data: "https://search.jd.com/Search?keyword=" + inputValue,
      type: "url",
    }

    ];
  }

  const db_app_params = {
    dbName: "lark",
    cacheTableName: "appCache",
    keyPath: "title",
    indexs: [
      { name: 'title', unique: true },
      { name: 'icon', unique: false },
      { name: 'iconPath', unique: false },
      { name: 'desc', unique: false },
      { name: 'data', unique: false },
      { name: 'type', unique: false },
    ]
  };
  const db_app_habit_params = {
    dbName: "lark",
    cacheTableName: "appHabit",
    keyPath: "keyword",
    indexs: [
      { name: 'keyword', unique: true },
      { name: 'habitData', unique: false },
    ]
  };
  const [appDB, setAppDB] = useState(null);
  const [appHabitDB, setAppHabitDB] = useState(null);


  function initStatus() {
    setPistol("");
    setInputValue("");
    setComponent(null);
    setComponentInfo("");
    setKeywordComponent([]);
    setSelectedIndex(-1);
    setIsComposing({ status: false, ppos: 0 });
    setfnDown(false);
  }
  const getWindowPosition = async () => {
    const factor = await appWindow.scaleFactor();
    const position = await appWindow.innerPosition();
    const logical = position.toLogical(factor);
    windowPosition.current = {x: logical.x, y: logical.y};
    return logical;
  };
  async function handleChange(event) {

    if (!isComposing.status) {
      setInputValue(event.target.value);
    }
  }

  async function handleKeyDown(event) {
    // 处理
    console.log(inputBox.current.value.length, event.key);
    if (!event.metaKey && event.key === "Enter") {
      await confirmComponentSelected();
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
      setComponent(<div className='activateComponent' data-tauri-drag-region>{searchFileComponent.icon}</div>);
      setComponentInfo(searchFileComponent);
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
        setfnDown(true);
      } else if (event.key === "Enter") {
        // let file_file = keywordComponent[selectedIndex].data.split("/");
        // file_file = file_file.slice(0, -1).join("/");
        // await invoke("open_explorer", { path: keywordComponent[selectedIndex].data });
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
      setIsComposing({ status: false, ppos: 0 });
    }
  }

  async function handleKeyUp(event) {
    setfnDown(false);
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
      return appHabitDB.update({ keyword, habitData: JSON.stringify(habitData) });
    } catch (err) {
      return console.log('更新数据失败[appHabitDB]==>', err);
    }
  };
  const initPoi = async () => {
    await appWindow.setPosition(new LogicalPosition(windowPosition.current.x, windowPosition.current.y));
  };

  async function confirmComponentSelected(index) {
    // 组件确认选择后
    let currentComponent = keywordComponent[index != undefined ? index : selectedIndex];
    setSelectedIndex(0);

    if (currentComponent.type == "component") {
      if (typeof currentComponent.icon == "string") {
        setComponent(<div className='activateComponent' data-tauri-drag-region>{currentComponent.icon.slice(0, 4)}</div>);
      } else {
        setComponent(currentComponent.icon);
      }
      setComponentInfo(currentComponent);
      setInputValue("");
      inputBox.current.focus();
    } else if (currentComponent.type == "subpage") {
      if (typeof currentComponent.icon == "string") {
        setComponent(<div className='activateComponent' data-tauri-drag-region>{currentComponent.icon.slice(0, 4)}</div>);
      } else {
        setComponent(currentComponent.icon);
      }
      setComponentInfo(currentComponent);
      setInputValue("");
      await modifyWindowSize("big");
    } else if (currentComponent.type == "result") {
      await invoke("set_window_hide_macos", {});
      await invoke("clipboard_control", { text: currentComponent.data.toString(), control: "copy", paste: true });
    } else if (currentComponent.type == "app") {
      if (!fnDown) {
        await updateAppHabit(inputValue, keywordComponent[selectedIndex].title);
        await invoke("open_app", { appPath: currentComponent.data });
        initStatus();
      } else {
        await invoke("open_explorer", { path: keywordComponent[selectedIndex].data });
      }
    } else if (currentComponent.type == "url") {
      console.log("打开网页");
      await invoke("open_url", { url: currentComponent.data });
    } else if (currentComponent.type == "search") {
      await invoke("open_url", { url: currentComponent.data });
    } else if (currentComponent.type == "action") {
      const handle = async (component) => {
        let result = await baseComponent['action_' + component.action](component.data);
        if (Object.prototype.toString.call(component.next) == '[object Object]') {
          component.next = [component.next];
        }
        for (let child of component.next) {
          if (child.resolve == result.resolve) {
            await handle(child);
          }
        }
      };
      handle(currentComponent);
      await invoke("set_window_hide_macos", {});
    } else if (currentComponent.type == "file") {
      if (!fnDown) {
        await invoke("open_file", { filePath: currentComponent.data });
      } else {
        await invoke("open_explorer", { path: keywordComponent[selectedIndex].data });
      }

    }
  }

  useEffect(() => {
    // 当组件信息改变时，重新聚焦输入框
    // 如果为小组件则将焦点聚焦在子页面输入框上
    if (componentInfo.type == "subpage") {
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
      if (pistol.split(".").pop() == "py") {
        let result = await invoke("run_python_script", { scriptPath: pistol });
        if (result.success == "true") {
          try {
            let data = JSON.parse(result.data);
            data = data.items;
            await modifyWindowSize(data.length);
            setKeywordComponent(data);
            setSelectedIndex(0);
          } catch (e) {
            console.error(e);
            setKeywordComponent([{ title: "Error", type: "result", icon: "E", "data": e, "desc": e.replace(/\n/g, "") }]);
            await modifyWindowSize(1);
            setSelectedIndex(0);
          }
        } else {
          setKeywordComponent([{ title: "Error", type: "result", icon: "E", "data": result.data, "desc": result.data.replace(/\n/g, "") }]);
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
        return calcComponent(calc_result);
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
      if (inputValue == "-") {
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
        return deleteIndexedDB("lark").then(() => { });
      }
      let searchType = "app";
      if (componentInfo?.title == "搜索文件") {
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
        // 计算器组件，在没有选择组件时，尝试计算
        if (searchType == "app") {
          let calc_result = calculator();
          if (calc_result) {
            result.push(calc_result);
          }
          // 判断输入的是不是网址
          if (isValidURL(inputValue)) {
            result.push(
              { title: inputValue, type: "url", icon: <img src={webImg}></img>, "data": inputValue, "desc": "使用默认浏览器打开url" }
            );
            await modifyWindowSize(result.length || "small");
          }
        }
        // 如果是搜索app时，尝试获取缓存
        console.log(window.searchFileCache);
        let query_result;
        if (searchType == "app" && appCache.length != 0) {
          query_result = appCache;
        } else if (searchType == "file" && Date.now() - (window.searchFileCache[inputValue]?.time || 0) < 10000) {
          query_result = window.searchFileCache[inputValue]?.data || [];
        } else {
          query_result = await invoke("search_keyword", { componentName: componentInfo?.title || "", inputValue });
          if (searchType == "app") {
            setAppCache(query_result);
          } else if (searchType == "file") {
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
        if (searchType == "app") {
          for (let pluginName in insidePluginList) {
            console.log(pluginName);
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
            if (item.title != "") {
              if (searchType == "app") {
                // 对于图标为icns的组件进行base64编码
                if (systemAppName[item.title]) {
                  item.title = systemAppName[item.title];
                }
                let match_key = "";
                let is_match = match(item.title, inputValue, { precision: 'start' });
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
                  if (componentCache[item.title] && componentCache[item.title].data == item.data) {
                    item = componentCache[item.title];
                  } else if (searchType == "app" && item_?.data == item.data) {
                    item = item_;
                    item.icon = <img src={`data:image/png;base64,${item.icon}`} style={{ width: "100%" }}></img>;
                  } else if (item.type == "app" && item.icon) {
                    await invoke("append_txt", { filePath: "/Users/starsxu/Documents/test2.txt", text: item.icon + "\n" });
                    let icon_ = await invoke("read_app_info", { appPath: item.icon });
                    if (icon_ != "文件不存在！") {
                      item.icon = icon_;
                      item.iconPath = icon_;
                    } else {
                      icon_ = await invoke("get_file_icon", { filePath: item.data });
                      if (icon_ != "文件不存在！") {
                        item.icon = icon_;
                        addDbAppItem(item);
                        item.icon = <img src={`data:image/png;base64,${icon_}`} style={{ width: "100%" }}></img>;
                      }
                    }
                    if (typeof item.icon == "string" && item.icon.indexOf(".icns") != -1) {
                      invoke('read_icns_to_base64', { path: item.icon })
                        .then((base64) => {
                          item.icon = base64;
                          addDbAppItem(item);
                          item.icon = <img src={`data:image/png;base64,${base64}`} style={{ width: "100%" }}></img>;
                        })
                        .catch((error) => {
                          console.error('Error reading file:', item.icon);
                        });
                    }
                  }

                  if (match_key == "py") {
                    pinyinMatches.push({ item, index: is_match[0] });
                  } else {
                    otherMatches.push({ item, index: match_key });
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
              if (searchType == "file") {
                let ext = item.data.split("/").pop();
                item.icon = <img src={getMaterialFileIcon(ext)} style={{ width: "100%" }}></img>;
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

          if (result.length == 0 && searchType == "app") {
            // 没有结果则进行web搜索
            result = webSearchComponent();
          } else {
            // 有结果 则判断关键词在appHabit中的热度，根据热度再排序。其中在这个关键词下每启动一次该app，则热度+1
            if (searchType == "app") {
              let habit = await getAppHabit(inputValue);
              result.sort((a, b) => (habit[b.title] || 0) - (habit[a.title] || 0));
            }
          }
          console.log(result);
        } catch (e) {
          await invoke("append_txt", { filePath: "/Users/starsxu/Documents/test.txt", text: JSON.stringify(e) + "\n" });
        }
        // 当前组件类型不为小窗组件时改变窗口大小
        if (componentInfo.type != "subpage") {
          await modifyWindowSize(result.length || "small");
        }

      } else if (componentInfo.type != "subpage") {
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
      if (event.payload == false && componentInfo.type != "subpage") {
        const hideWindow = async () => {
          await appWindow.hide();
          await modifyWindowSize("small");
        };
        // ? 正式启用
        // hideWindow().then(initCache);
      }
    });

    const unListenFocusChanged = listen('window-focus', event => {
      if (event.payload == true) {
        initStatus();
        initPoi();
        appWindow.show();
        appWindow.setFocus();
        inputBox.current.focus();
      }

    });
    if (!appDB) {
      setAppDB(new IndexDBCache(db_app_params));
    }

    const unListenFileDrop = listen('tauri://file-drop', event => {
      const { payload } = event;
      if (Array.isArray(payload) && payload.length > 0) {
        setPistol(payload[0]);
        if (!componentInfo || componentInfo.type != "subpage") {
          inputBox.current.focus();
        }
      }
    });


    // 输入框获取焦点
    inputBox.current.focus();
    // 初始化AppCache
    const initCache = async () => {
      if (!updateCacheTime) {
        updateCacheTime = setTimeout(async () => {
          let query_result = await invoke("search_keyword", { componentName: "", inputValue });
          setAppCache(query_result);
          updateCacheTime = null;
        }, 3000);
      }
    };
    initCache();



    // 读取本地组件库，查看注册状态
    const loadCustomComponent = async () => {
      const files = {};
      const importAll = import.meta.glob('./components/*.json');
      for (const path in importAll) {
        const module = await importAll[path]();
        const fileName = path.replace('./components/', '').replace(".json", "");
        files[fileName] = module;
      };

      setPluginList(files);

    };

    loadCustomComponent();

    if (!windowPosition.current) {
      getWindowPosition();
    }

    window.searchFileCache = {};


    return () => {
      unListenFocusChanged.then((f) => f());
      unListenFileDrop.then((f) => f());

    };

  }, []);

  const initIndexDB = async (db, version) => {
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

  useEffect(() => {
    // 初始化本地缓存数据库链接
    if (appDB && !appDB._db) {
      let version = dbList.includes(appDB._cacheTableName) ? dbList.length : dbList.length + 1;
      initIndexDB(appDB, version).then(() => {
        if (!appHabitDB) {
          setTimeout(() => {
            setAppHabitDB(new IndexDBCache(db_app_habit_params));
          }, 1000);
        }
      });
    };


  }, [appDB]);

  useEffect(() => {

    if (appHabitDB && !appHabitDB._db) {
      let version = dbList.includes(appHabitDB._cacheTableName) ? dbList.length : dbList.length + 1;
      if (appDB && appDB._dbversion < version) {
        appDB.closeDB();
      }
      initIndexDB(appHabitDB, version).then(() => {
        if (!appDB) {
          setTimeout(() => {
            setAppDB(new IndexDBCache(db_app_params));
          }, 1000);
        }
      });
    }
  }, [appHabitDB]);
  const handleCompositionStart = () => {
    setIsComposing({ status: true, ppos: 0 });
  };

  const handleCompositionEnd = (event) => {
    setIsComposing({ status: false, ppos: 1 });
    setInputValue(event.target.value);
  };

  return (
    <div id="mainDiv" data-tauri-drag-region >
      <div style={{ width: "100%", height: "51.5px", margin_bottom: "5px" }}>
        <div style={{ width: "100%", height: "100%", display: "flex", justifyContent: "colum", alignItems: "center" }}>
          {!component ? <div /> : component}
          {!pistol ? <div /> : <div className='pistol' onDoubleClick={() => { setPistol(""); }}><p className='pistolText'>{pistol.split("/").pop()}</p></div>}
          <input ref={inputBox} type="text" id='mainInput' autoCorrect="off" spellCheck="false" onChange={handleChange} onKeyDown={handleKeyDown} onKeyUp={handleKeyUp} onCompositionStart={handleCompositionStart} onCompositionEnd={handleCompositionEnd} />
        </div>
        {templateComponent(keywordComponent, selectedIndex, setSelectedIndex, confirmComponentSelected, fnDown)}
        {subpageComponent(componentInfo)}
      </div>
    </div>
  );
};

export default App;
