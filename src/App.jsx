import "./app.css";
import React, { lazy, Suspense } from "react";
import { flushSync } from "react-dom";
import { LogicalPosition } from "@tauri-apps/api/window";
import {
  WebviewWindow,
  getCurrentWebviewWindow,
  getAllWebviewWindows
} from "@tauri-apps/api/webviewWindow";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect, useRef, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import webImg from "./assets/web.svg";
import baseComponent from "./baseComponent";
import { executePluginWorkflow, normalizePluginPath } from "./pluginRuntime";
import {normalizePluginResults} from "./pluginScaffold";
import {schedulePluginExecution} from "./pluginExecution";
import PluginView from "./PluginView";
import { useLocalStorage } from "react-use";
import {
  getMaterialFileIcon,
  getMaterialFolderIcon,
} from "file-extension-icon-js";
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
  initAppHabitDB,
} from "./template.jsx";

const SubpageComponent = React.lazy(() =>
  import("./template.jsx").then((mod) => ({ default: mod.SubpageComponent }))
);
const TemplateComponent = React.lazy(() =>
  import("./template.jsx").then((mod) => ({ default: mod.TemplateComponent }))
);

const App = () => {
  // 键入值
  const [inputValue, setInputValue] = useState("");
  const [searchOffset, setSearchOffset] = useState(0);
  // 组件结构
  const [component, setComponent] = useState("");
  // 手枪组件
  const [pistol, setPistol] = useState("");
  // 组件信息
  const [componentInfo, setComponentInfo] = useState("");
  // 搜索组件结果数组
  const [keywordComponent, setKeywordComponent] = useState([]);
  // 当前选中组件索引
  const [selectedIndex, setSelectedIndex] = useState(-1);
  // 输入状态判断
  const [isComposing, setIsComposing] = useState({ status: false, ppos: 0 });
  // 输入框组件
  const inputBox = useRef(null);
  const searchRequestId = useRef(0);
  const windowPosition = useRef(null);
  // 功能键状态
  const [fnDown, setFnDown] = useState(false);
  // 按下按键
  const [keyDown, setKeyDown] = useState(false);
  // 自制插件管理
  const [pluginStatus, setPluginStatus] = useLocalStorage("pluginStatus", {});
  // 自制插件列表
  const [pluginList, setPluginList] = useState({});
  const [pluginsLoading, setPluginsLoading] = useState(true);
  const [pluginsError, setPluginsError] = useState("");
  const [pluginSettingsStatus, setPluginSettingsStatus] = useState({});
  const pluginRefreshId = useRef(0);

  // 只拉取「必填项是否齐全」的摘要，不把插件的配置值读进前端内存。
  const refreshPluginSettingsStatus = () => {
    invoke("get_plugin_settings_status")
      .then((status) => setPluginSettingsStatus(status && typeof status === "object" ? status : {}))
      .catch(() => setPluginSettingsStatus({}));
  };

  const refreshPlugins = async () => {
    const requestId = ++pluginRefreshId.current;
    setPluginsLoading(true);
    setPluginsError("");
    try {
      const result = await loadCustomComponent();
      if (requestId === pluginRefreshId.current) {
        setPluginList(result);
        refreshPluginSettingsStatus();
      }
      return {ok: true};
    } catch (error) {
      if (requestId === pluginRefreshId.current) setPluginsError(String(error));
      return {ok: false, error: String(error)};
    } finally {
      if (requestId === pluginRefreshId.current) setPluginsLoading(false);
    }
  };

  const togglePlugin = (pluginId, enable) => {
    const nextStatus = { ...pluginStatus, [pluginId]: { ...pluginStatus?.[pluginId], enable } };
    localStorage.setItem("pluginStatus", JSON.stringify(nextStatus));
    searchRequestId.current += 1;
    setPluginStatus(nextStatus);
  };

  const [dbList, setDbList] = useLocalStorage("dbList", []);

  const [appDirectory, setAppDirectory] = useState({});

  const [insidePluginList, setInsidePluginList] = useState(pluginsComponent);

  // 带目标插件打开组件库，并由 showComponent 的 pluginConfigId 自动进入该插件的配置页。
  const openPluginConfig = async (plugin) => {
    setKeywordComponent([]);
    setSelectedIndex(0);
    setInputValue("");
    setComponentInfo({ ...insidePluginList.showPluginComponent, pluginConfigId: plugin.id });
    await modifyWindowSize("expanded");
  };

  const [appHabitDB, setAppHabitDB] = useState(null);

  const [actionParent, setActionParent] = useState({});
  const [activePluginWorkflow, setActivePluginWorkflow] = useState(null);

  // 全局事件监听器只注册一次，使用 ref 读取最新的面板状态，避免闭包持有旧值。
  const componentInfoRef = useRef(componentInfo);
  const initStatusRef = useRef(null);
  // 当前面板（如插件创建向导）需要接管窗口拖放时，在此注册 (path) => void 处理器；
  // 为空则拖入的文件按原路径交给 pistol（插件入参 / 文件搜索 / .py 执行）。
  // Windows 上 WebView2 的原生拖放已被 Tauri 的 drop handler 接管，面板内收不到 HTML5 drop 事件。
  const panelDropHandlerRef = useRef(null);

  const appWindow = getCurrentWebviewWindow();

  function initStatus(components) {
    let resizePromise;
    // 立即废弃隐藏前尚未返回的搜索，避免它在窗口重新显示后回填旧结果。
    searchRequestId.current += 1;
    setPistol("");
    setSelectedIndex(-1);
    if (!components) {
      setInputValue("");
      // 清空当前面板/结果时，同时恢复主窗口的紧凑尺寸。
      resizePromise = modifyWindowSize("compact");
    } else {
      resizePromise = modifyWindowSize(components.length);
      setSelectedIndex(0);
    }
    setComponent(null);
    setComponentInfo("");
    setActivePluginWorkflow(null);
    setKeywordComponent(components);
    setIsComposing({ status: false, ppos: 0 });
    setFnDown(false);
    return resizePromise;
  }

  componentInfoRef.current = componentInfo;
  initStatusRef.current = initStatus;

  // 窗口拖放由 Rust 侧的 win-file-drop 插件接管：它注入脚本把 File 交给宿主解析成真实路径，
  // 再以 tauri://drag-drop 事件发回来（见下方 unListenFileDrop）。这里不再自己解析 dataTransfer。
  async function handleKeyDown(event) {
    // 处理键盘按下
    setKeyDown(event);
    if (event.nativeEvent.isComposing || event.keyCode === 229) return;
    if (componentInfoRef.current?.type === "panel") {
      // panel 仍由主输入框接收键盘事件，但不允许按键修改输入内容。
      if (event.key !== "Tab" || componentInfoRef.current?.data !== "showComponent") event.preventDefault();
      if (event.key === "Escape" && isComposing.ppos === 0) {
        // workspace 内部页面（例如插件创建/编辑器）自己处理 Esc，
        // 不要让宿主的全局处理器直接清空 panel。
        if (document.getElementById("mainDiv")?.dataset.windowMode === "workspace") return;
        initStatusRef.current?.();
        setTimeout(() => inputBox.current?.focus(), 50);
      }
      return;
    }
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
      setActivePluginWorkflow(null);
      if (inputValue) {
        setKeywordComponent([]);
        setInputValue("");
        if (componentInfo.type !== "panel") {
          await modifyWindowSize("compact");
        }
      } else if (pistol) {
        setPistol("");
        setKeywordComponent([]);
        await modifyWindowSize("compact");
      } else if (component) {
        setComponent(null);
        setComponentInfo("");
        await modifyWindowSize("compact");
      } else {
        setComponent(null);
        setComponentInfo("");
        await modifyWindowSize("compact");
      }
    } else if (
      event.code === "Tab" &&
      (event.target.value === "" || event.target.value === " ") &&
      !component
    ) {
      // 当Tab被按下时，如果输入框为空，则进入文件搜索模式
      event.preventDefault();
      event.target.value = "";
      let icon = createActiveIcon(insidePluginList.searchFileComponent.icon);
      setComponent(icon);
      setComponentInfo(insidePluginList.searchFileComponent);
    } else if (
      inputBox.current.value.length === 1 &&
      event.key === "Backspace"
    ) {
      // 交互式插件统一使用 Escape 退出；Backspace 只负责编辑输入内容。
      if (activePluginWorkflow) return;
      setKeywordComponent([]);
      if (componentInfo.type !== "panel") {
        await modifyWindowSize("compact");
      }
    } else if (
      inputBox.current.value.length === 0 &&
      isComposing.ppos === 0 &&
      event.key === "Backspace"
    ) {
      if (pistol) {
        setPistol("");
        setKeywordComponent([]);
        await modifyWindowSize("compact");
      } else {
        if (activePluginWorkflow) return;
        setComponent(null);
        setComponentInfo("");
        setKeywordComponent([]);
        await modifyWindowSize("compact");
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
    } else if (event.metaKey || event.altKey) {
      console.log(event.metaKey, event.key);
      if (event.key === "Meta" || event.code === "AltLeft") {
        setFnDown(true);
      } else if (event.key === "Enter") {
        await confirmComponentSelected();
        setInputValue("");
      } else {
        try {
          if (parseInt(event.key) <= 9 && parseInt(event.key) > 0) {
            let list_items =
              document.getElementsByClassName("templateComponent");
            let firstItem = 0;
            for (let i = 0; i < list_items.length; i++) {
              if (list_items[i].getBoundingClientRect().y === 60.5) {
                firstItem = i;
                break;
              }
            }
            await confirmComponentSelected(
              firstItem + parseInt(event.key) - 1,
              false
            );
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

  const getAppHabit = async (keyword) => {
    // 获取用户习惯热点度
    try {
      const res = await appHabitDB.getDataByKey(keyword);
      return JSON.parse(res[0]?.habitData || "{}");
    } catch (err) {
      return console.log("获取数据失败[appHabitDB]==>", err) || {};
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
      return appHabitDB.update({
        keyword,
        habitData: JSON.stringify(habitData),
      });
    } catch (err) {
      return console.log("更新数据失败[appHabitDB]==>", err);
    }
  };

  const initPoi = async () => {
    let window_position = await windowPosition.current;
    await appWindow.setPosition(
      new LogicalPosition(window_position.x, window_position.y)
    );
  };

  const runActivePluginWorkflow = async (active, text) => {
    const { plugin, workflow } = active;
    const response = await executePluginWorkflow(
      plugin,
      workflow,
      { text, file: pistol, ui: { close: () => initStatus() } },
      { text, file: pistol }
    );
    return normalizePluginResults(response, plugin.name).map((item) => ({
      ...item,
      icon: item.icon || workflow.icon || "JS",
    }));
  };

  const createActiveIcon = (icon) => {
    if (typeof icon === "string") {
      return <div className="activateComponent activateComponent--active" data-tauri-drag-region>{icon.slice(0, 4)}</div>;
    }
    if (!React.isValidElement(icon)) return icon;
    return React.cloneElement(icon, {
      className: `${icon.props?.className || "activateComponent"} activateComponent--active`,
      style: { ...(icon.props?.style || {}), height: "38px" },
    });
  };

  async function confirmComponentSelected(index, metaStatus) {
    // 组件确认选择后
    let currentComponent =
      keywordComponent[index !== undefined ? index : selectedIndex];
    setSelectedIndex(0);
    console.log(currentComponent);
    if (!currentComponent) return;
    const plugin = currentComponent.pluginId && pluginList[currentComponent.pluginId];
    if (currentComponent.pluginId && (!plugin || plugin.__error || pluginStatus?.[currentComponent.pluginId]?.enable === false)) return;
    // 必填配置缺失时不执行插件，直接带用户去该插件的配置页，避免被误判成插件故障。
    if (plugin && ["action", "python"].includes(currentComponent.type) &&
        pluginSettingsStatus?.[plugin.id]?.configured === false) {
      await openPluginConfig(plugin);
      return;
    }
    if (plugin && ["action", "python"].includes(currentComponent.type) && plugin.entry?.type === "js") {
      if (currentComponent.interactive) {
        searchRequestId.current += 1;
        setKeywordComponent([]);
        setSelectedIndex(-1);
        setActivePluginWorkflow({ plugin, workflow: currentComponent });
        setComponent(createActiveIcon(currentComponent.icon));
        setComponentInfo({ ...currentComponent, type: "input-panel" });
        setInputValue("");
        return;
      }
      try {
        const result = await executePluginWorkflow(plugin, currentComponent,
          { text: inputValue, file: pistol, ui: { close: () => initStatus() } },
          { text: inputValue, file: pistol });
        const results = normalizePluginResults(result, plugin.name);
        if (results.length) initStatus(results); else initStatus();
      } catch (error) {
        initStatus([{ type: "result", title: "插件执行失败", desc: String(error), icon: "ERROR", data: String(error) }]);
      }
      return;
    }
    if (plugin && currentComponent.type === "panel") {
      await updateAppHabit(inputValue, currentComponent.title || plugin.name);
      setComponentInfo({ ...currentComponent, pluginManifest: plugin });
      setInputValue("");
      await modifyWindowSize("expanded");
      return;
    }
    if (currentComponent.type === "input-panel") {
      await updateAppHabit(inputValue, currentComponent.title);
      if (typeof currentComponent.icon == "string") {
        setComponent(
          <div className="activateComponent" data-tauri-drag-region>
            {currentComponent.icon.slice(0, 4)}
          </div>
        );
      } else {
        setComponent(createActiveIcon(currentComponent.icon));
      }
      setComponentInfo(currentComponent);
      setInputValue("");
      inputBox.current.focus();
    } else if (currentComponent.type === "panel") {
      await updateAppHabit(inputValue, currentComponent.title);
      // 设置子页面的图标
      if (typeof currentComponent.icon == "string") {
        setComponent(
          <div className="activateComponent" data-tauri-drag-region>
            {currentComponent.icon.slice(0, 4)}
          </div>
        );
      } else {
        setComponent(createActiveIcon(currentComponent.icon));
      }
      setComponentInfo(currentComponent);
      setInputValue("");
      await modifyWindowSize("expanded");
    } else if (currentComponent.type === "result") {
      await appWindow.hide();
      await invoke("clipboard_control", {
        text: currentComponent.data.toString(),
        control: "write",
        paste: true,
        dataType: "text",
      });
    } else if (currentComponent.type === "app") {
      if (!fnDown || metaStatus === false) {
        await updateAppHabit(inputValue, currentComponent.title);
        console.log(currentComponent);
        await invoke("open_app", {
          appPath: currentComponent.data,
          appName: currentComponent.title,
        });
        await appWindow.hide();
        initStatus();
      } else {
        await invoke("open_explorer", {
          path: keywordComponent[selectedIndex].data,
        });
        await appWindow.hide();
      }
    } else if (currentComponent.type === "url") {
      console.log("打开网页");
      await invoke("open_url", { url: currentComponent.data });
      await appWindow.hide();
    } else if (currentComponent.type === "search") {
      await invoke("open_url", { url: currentComponent.data });
      await appWindow.hide();
    } else if (currentComponent.type === "action") {
      const handle = async (component) => {
        let scriptPath = component.data,
          scriptParams = "",
          result;
        if (component.script) {
          scriptPath = component.script.data;
          scriptParams = component.data || "";
          component.action = component.script.action;
          component.params = component.script.params || {};
        }
        if (
          scriptPath &&
          (scriptPath?.startsWith("./") || scriptPath?.[1] !== ":")
        ) {
          scriptPath = scriptPath.replace("./", "");
          scriptPath =
            scriptPath[0] === "/"
              ? scriptPath.substring(1, scriptPath.length)
              : scriptPath;
          scriptPath = `${component.pluginRoot || appDirectory["plugins"] + "/" + component.pluginName}/${scriptPath}`;
        }
        try {
          if (scriptPath) {
            result = await baseComponent["action_" + component.action](
              scriptPath,
              scriptParams || inputValue.split(" ")
            );
          } else {
            let input_value = scriptParams;
            if (!input_value) {
              let kg_index =
                input_value.indexOf(" ") !== -1 ? input_value.indexOf(" ") : 0;
              input_value = inputValue.substring(kg_index + 1);
            }
            result = await baseComponent["action_" + component.action](
              input_value,
              component.params
            );
          }
        } catch (e) {
          let res = {
            type: "result",
            title: e.toString(),
            desc: e.toString().slice(0, 100),
            icon: "ERROR",
            data: e.toString(),
          };
          return initStatus([res]);
        }
        if (!result) {
          await appWindow.hide();
          return initStatus();
        }
        result.data = JSON.parse(result.data);
        let items = [];
        for (let info of result.data.items) {
          let item = {
            type: component.next?.type || "result",
            title: info.title,
            desc: info.subtitle,
            icon: component.icon,
            data: info.arg,
            script: component.next,
          };
          items.push(item);
        }
        initStatus(items);
        console.log(items);
      };
      handle(currentComponent);
    } else if (currentComponent.type === "file") {
      if (!fnDown) {
        await invoke("open_file", { filePath: currentComponent.data });
      } else {
        await invoke("open_explorer", {
          path: keywordComponent[selectedIndex].data,
        });
      }
      await appWindow.hide();
      initStatus();
    } else {
      setKeywordComponent([]);
    }
  }

  useEffect(() => {
    // panel 的键盘事件仍由主输入框接收，再通过 keyDown 传给面板组件。
    if (componentInfo.type === "panel") {
      setTimeout(() => inputBox.current?.focus({ preventScroll: true }), 50);
    }
  }, [componentInfo]);

  useEffect(() => {
    // 文件拖放识别
    const fetchData = async () => {
      if (pistol.split(".").pop() === "py") {
        let result = await invoke("run_python_script", { scriptPath: pistol });
        if (result.success === "true") {
          try {
            let data = JSON.parse(result.data);
            data = data.items;
            await modifyWindowSize(data.length);
            setKeywordComponent(data);
            setSelectedIndex(0);
          } catch (e) {
            console.error(e);
            setKeywordComponent([
              {
                title: "Error",
                type: "result",
                icon: "E",
                data: e,
                desc: e.replace(/\n/g, ""),
              },
            ]);
            await modifyWindowSize(1);
            setSelectedIndex(0);
          }
        } else {
          setKeywordComponent([
            {
              title: "Error",
              type: "result",
              icon: "E",
              data: result.data,
              desc: result.data.replace(/\n/g, ""),
            },
          ]);
          await modifyWindowSize(1);
          setSelectedIndex(0);
        }
      }
    };
    fetchData();
  }, [pistol]);

  useEffect(() => {
    // 输入框内容提交
    const requestId = ++searchRequestId.current;
    const isCurrent = () => requestId === searchRequestId.current;
    function calculator() {
      let calc_result = calculateExpression(inputValue);
      if (calc_result !== false) {
        return calcComponent(calc_result, inputValue);
      }
    }

    function isValidURL(url) {
      const urlPattern =
        /^(https?:\/\/)?(www\.)?((([0-9]{1,3}\.){3}[0-9]{1,3})|([a-zA-Z0-9-]+\.[a-zA-Z]{2,}))([a-zA-Z0-9\-._~:/?#[\]@!$&'()*+,;=%]*)$/;
      return urlPattern.test(url);
    }

    const fetchData = async () => {
      if (!isCurrent()) return;
      if (inputBox.current) {
        inputBox.current.value = inputValue;
      }
      if (inputValue === "-" && !activePluginWorkflow && !componentInfo?.type) {
        function deleteIndexedDB(dbName) {
          return new Promise((resolve, reject) => {
            const request = indexedDB.deleteDatabase(dbName);

            request.onsuccess = () => {
              console.log(`Database ${dbName}   successfully`);
              resolve();
            };

            request.onerror = (event) => {
              console.error(
                `Error deleting database ${dbName}:`,
                event.target.error
              );
              reject(event.target.error);
            };

            request.onblocked = () => {
              console.warn(`Database ${dbName} delete blocked`);
            };
          });
        }

        setDbList([]);
        return deleteIndexedDB("lark").then(() => {});
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
      if (activePluginWorkflow) {
        try {
          const {plugin, workflow} = activePluginWorkflow;
          if (pluginStatus?.[plugin.id]?.enable === false || !pluginList[plugin.id]) return;
          if (workflow.skipEmptyInput && !inputValue.trim()) {
            result = [];
          } else {
            result = await schedulePluginExecution(`${plugin.id}:${workflow.id}`,
              () => runActivePluginWorkflow(activePluginWorkflow, workflow.skipEmptyInput ? inputValue : inputValue.trim()), isCurrent);
          }
        } catch (error) {
          result = [{
            type: "result",
            title: "插件执行失败",
            desc: String(error),
            icon: "ERROR",
            data: String(error),
          }];
        }
        if (requestId !== searchRequestId.current) return;
        await modifyWindowSize(result.length || "compact");
        if (!isCurrent()) return;
        setKeywordComponent(result);
        setSelectedIndex(0);
        return;
      }
      // 输入有值且不在输入状态时,进行搜索
      if (inputValue.trim() && !isComposing.status) {
        if (inputValue.trim() === "reIndex") {
          return await invoke("create_file_index", {});
        }
        if (inputValue.trim() === "reApp") {
          return await invoke("create_app_index", {});
        }
        // 计算器组件，在没有选择组件时，尝试计算
        if (searchType === "app") {
          let calc_result = calculator();
          if (calc_result) {
            result.push(calc_result);
          }
          // 判断输入的是不是网址
          if (isValidURL(inputValue)) {
            result.push({
              title: inputValue,
              type: "url",
              icon: <img alt={"web"} src={webImg}></img>,
              data: inputValue,
              desc: "使用默认浏览器打开url",
            });
            await modifyWindowSize(result.length || "compact");
          }
        }
        // 如果是搜索app时，尝试获取缓存
        let query_result;

        if (
          searchType === "file" &&
          Date.now() - (window.searchFileCache[inputValue]?.time || 0) < 10000
        ) {
          query_result = window.searchFileCache[inputValue]?.data || [];
        } else {
          query_result = await invoke("search_keyword", {
            componentName: componentInfo?.title || "",
            inputValue,
            offset: searchOffset,
            params: {},
          });
        }
        // const pinyinMatches = [];
        // const otherMatches = [];

        //* 匹配内部插件
        if (searchType === "app") {
          for (let pluginName in insidePluginList) {
            let plugin = insidePluginList[pluginName];
            if (
              plugin.title.startsWith(inputValue) ||
              plugin.desc.startsWith(inputValue)
            ) {
              result.push(plugin);
            }
          }
          // 匹配自定义插件组件
          for (let pluginName in pluginList) {
            if (pluginStatus?.[pluginName]?.enable === false || pluginList[pluginName].__error) {
              continue;
            }
            let plugin = pluginList[pluginName];
            let workflows = plugin.workflows || plugin.workflow || [];
            for (let workflow of workflows) {
              const keywords = workflow.keywords || (workflow.keyword ? [workflow.keyword] : []);
              if (keywords.some((keyword) => keyword?.startsWith(inputValue))) {
                workflow = { ...workflow };
                workflow.type = workflow.type || "action";
                workflow.action = workflow.action || workflow.handler;
                workflow.pluginName = pluginName;
                workflow.pluginId = plugin.__pluginId || plugin.id || pluginName;
                workflow.pluginRoot = plugin.__root;
                workflow.pluginTitle = plugin.name || plugin.title || pluginName;
                if (typeof workflow.icon !== "object") {
                  let icon, src;
                  if (workflow.icon?.startsWith("./")) {
                    icon = workflow.icon.replace("./", "");
                  } else {
                    icon = workflow.icon || "";
                    icon =
                      icon[0] === "/" ? icon.substring(1, icon.length) : icon;
                  }
                  if (icon[1] === ":") {
                    src = convertFileSrc(icon);
                  } else {
                    src = convertFileSrc(
                      `${normalizePluginPath(plugin.__root)}/${icon}`
                    );
                  }
                  workflow.icon = (
                    <img
                      alt={"i"}
                      src={src}
                      className={"activateComponent"}
                      data-tauri-drag-region
                    />
                  );
                  workflow.parent = pluginName;
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
                  item.icon = (
                    <img
                      src={`data:image/png;base64,${item.icon}`}
                      style={{ width: "100%" }}
                    ></img>
                  );
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
                  item.icon = (
                    <img
                      src={getMaterialFolderIcon(item.file_type)}
                      style={{ width: "100%" }}
                    ></img>
                  );
                } else {
                  item.icon = (
                    <img
                      src={getMaterialFileIcon(item.file_type)}
                      style={{ width: "100%" }}
                    ></img>
                  );
                }
                result.push(item);
              }
            }
          }

          // 对匹配项进行排序
          // 排序拼音匹配项
          // pinyinMatches.sort((a, b) => a.index - b.index);
          // 排序其他匹配项
          // otherMatches.sort((a, b) => a.index - b.index);
          // 合并结果
          // result = [...result, ...pinyinMatches.map(match => match.item), ...otherMatches.map(match => match.item)].slice(0, 9);

          if (result.length === 0 && searchType === "app") {
            // 没有结果则进行web搜索
            result = webSearchComponent(inputValue);
          } else {
            // 有结果 则判断关键词在appHabit中的热度，根据热度再排序。其中在这个关键词下每启动一次该app，则热度+1
            if (searchType === "app") {
              let habit = await getAppHabit(inputValue);
              result.sort(
                (a, b) => (habit[b.title] || 0) - (habit[a.title] || 0)
              );
            }
          }
        } catch (e) {
          console.log("错误:", e);
        }
        // 当前组件类型不为小窗组件时改变窗口大小
        if (componentInfo.type !== "panel") {
          await modifyWindowSize(result.length || "compact");
        }
      } else if (componentInfo.type !== "panel") {
        await modifyWindowSize("compact");
      }
      if (requestId !== searchRequestId.current) return;
      console.log(result);
      setKeywordComponent(result);
      setSelectedIndex(0);
    };

    // 仅保留很短的防抖；查询本身在 Rust 后台线程执行，避免阻塞输入事件。
    const timer = setTimeout(() => {
      fetchData();
    }, activePluginWorkflow?.workflow.debounceMs === 200 ? 200 : 80);

    return () => { clearTimeout(timer); searchRequestId.current += 1; };
  }, [inputValue, activePluginWorkflow, pluginList, pluginStatus]);

  useEffect(() => {
    // 监听窗口失去焦点 隐藏窗口
    let updateCacheTime;
    const unListenAutoHide = appWindow.onFocusChanged((event) => {
      console.log("当前组件的信息", componentInfoRef.current);
      if (event.payload === false && componentInfoRef.current?.type !== "panel") {
        const hideWindow = async () => {
          await appWindow.hide();
          await modifyWindowSize("compact");
        };
        // ? 正式启用
        hideWindow().then();
      }
    });
    // const appWindow = getCurrent();
    // console.log(appWindow);

    // if (appWindow) {
    //   appWindow.listen("tauri://focus", () => {
    //     console.log("窗口获取焦点");
    //   });

    //   appWindow.listen("tauri://blur", () => {
    //     console.log("窗口失去焦点");
    //   });
    // }
    const webview = getCurrentWebview();
    const focusInput = async () => {
      if (componentInfoRef.current?.type === "panel") return;
      await webview.setFocus();
      setTimeout(() => {
        inputBox.current?.focus();
      }, 50);
    };
    const focusPanelInputAfterWake = async () => {
      // Alt+Space 恢复 native 窗口后，WebView 需要显式恢复一次键盘焦点。
      // 只从自定义唤醒事件调用，不能放进 onFocusChanged，否则会形成焦点循环。
      await webview.setFocus();
      setTimeout(() => {
        inputBox.current?.focus({ preventScroll: true });
      }, 50);
    };
    const unListenWindowFocus = appWindow.onFocusChanged(({ payload: focused }) => {
      if (!focused) return;
      setTimeout(() => inputBox.current?.focus({ preventScroll: true }), 50);
    });
    // window.onVisibleChanged(({ payload }) => {
    //   if (payload === true) {
    //     console.log("窗口变为可见，尝试聚焦输入框");
    //     setTimeout(() => {
    //       inputBox.current?.focus();
    //     }, 50);
    //   }
    // });

    const unListenShowRequest = listen("window-show-request", async () => {
      if (componentInfoRef.current?.type === "panel") {
        await appWindow.show();
        await appWindow.setFocus();
        await focusPanelInputAfterWake();
      } else {
        let resizePromise;
        flushSync(() => {
          resizePromise = initStatusRef.current?.();
        });
        if (inputBox.current) inputBox.current.value = "";
        await resizePromise;
        await appWindow.show();
        await appWindow.setFocus();
        await focusInput();
      }
    });
    const unListenClipboardShowRequest = listen("clipboard-show-request", async () => {
      flushSync(() => initStatusRef.current?.());
      const clipboard = insidePluginList.clipboardPluginComponent;
      setComponent(createActiveIcon(clipboard.icon));
      setComponentInfo(clipboard);
      await modifyWindowSize("expanded");
      await appWindow.show();
      await appWindow.setFocus();
      await focusPanelInputAfterWake();
    });

    const showTrayPanel = async (panelKey) => {
      flushSync(() => initStatusRef.current?.());
      const panel = insidePluginList[panelKey];
      if (!panel) return;
      setComponent(createActiveIcon(panel.icon));
      setComponentInfo(panel);
      await modifyWindowSize("expanded");
      await appWindow.show();
      await appWindow.setFocus();
      await focusPanelInputAfterWake();
    };
    const unListenSettingsShowRequest = listen("settings-show-request", () => showTrayPanel("settingPluginComponent"));
    const unListenComponentsShowRequest = listen("components-show-request", () => showTrayPanel("showPluginComponent"));
    const handleGlobalKeyDown = (event) => {
      if (componentInfoRef.current?.type === "panel") {
        if (event.key === "Escape" && isComposing.ppos === 0) {
          event.preventDefault();
          if (document.getElementById("mainDiv")?.dataset.windowMode === "workspace") return;
          initStatusRef.current?.();
          setTimeout(() => inputBox.current?.focus(), 50);
        }
        return;
      }
      inputBox.current?.focus();
    };

    document.addEventListener("keydown", handleGlobalKeyDown);

    // Tauri v2 的窗口拖放事件是 tauri://drag-drop，payload 形如 { paths: [...], position }；
    // v1 的 tauri://file-drop 在 v2 中已不再发出，监听它永远不会触发。
    const unListenFileDrop = listen("tauri://drag-drop", (event) => {
      const [path] = event.payload?.paths ?? [];
      if (!path) return;
      // 面板（如插件创建向导）接管拖放时优先交给它，避免误设 pistol。
      const panelHandler = panelDropHandlerRef.current;
      if (panelHandler) {
        panelHandler(path);
        return;
      }
      setPistol(path);
      if (componentInfoRef.current?.type !== "panel") {
        inputBox.current?.focus();
      }
    });

    const unListenFileIndex = listen("file_index_count", (event) => {
      const { payload } = event;
      console.log(payload);
    });

    // 输入框获取焦点
    inputBox.current.focus();

    initAppHabitDB(appHabitDB, setAppHabitDB, dbList, setDbList);

    // 读取本地组件库，查看注册状态
    refreshPlugins();
    if (!windowPosition.current) {
      windowPosition.current = getWindowPosition();
    }

    window.searchFileCache = {};
    invoke("get_app_dir", {}).then((result) => {
      setAppDirectory(result);
    });

    return () => {
      unListenShowRequest.then((f) => f());
      unListenClipboardShowRequest.then((f) => f());
      unListenSettingsShowRequest.then((f) => f());
      unListenComponentsShowRequest.then((f) => f());
      unListenAutoHide.then((f) => f());
      unListenWindowFocus.then((f) => f());
      unListenFileDrop.then((f) => f());
      unListenFileIndex.then((f) => f());
      document.removeEventListener("keydown", handleGlobalKeyDown);
    };
  }, []);

  useEffect(() => {
    initAppHabitDB(appHabitDB, setAppHabitDB, dbList, setDbList);
  }, [appHabitDB]);

  useEffect(() => {
    console.log("新值：", keywordComponent);
  }, [keywordComponent]);
  return (
    <div id="mainDiv" data-tauri-drag-region>
      <div style={{ width: "100%", height: "51.5px", margin_bottom: "5px" }}>
        <div
          style={{
            width: "100%",
            height: "100%",
            display: "flex",
            justifyContent: "colum",
            alignItems: "center",
          }}
        >
          {!component ? <div /> : component}
          {!pistol ? (
            <div />
          ) : (
            <div className="pistol" title={pistol}
              onDoubleClick={() => setPistol("")}>
              {/* 拖入的是 Windows 反斜杠路径，两种分隔符都要切，否则会显示整条路径 */}
              <span className="pistolText">{pistol.split(/[\\/]/).pop()}</span>
              <span className="pistolRemove" role="button" aria-label="移除文件"
                onClick={() => { setPistol(""); inputBox.current?.focus(); }}>×</span>
            </div>
          )}
          <input
            ref={inputBox}
            type="text"
            id="mainInput"
            placeholder={activePluginWorkflow?.workflow.skipEmptyInput ? "输入文本后自动执行，Esc 返回" : ""}
            autoComplete="off"
            autoCorrect="off"
            spellCheck="false"
            autoFocus
            tabIndex={componentInfo?.type === "panel" ? -1 : 0}
            onBeforeInput={(event) => {
              if (componentInfoRef.current?.type === "panel") event.preventDefault();
            }}
            onChange={(event) => {
              if (!isComposing.status) {
                searchRequestId.current += 1;
                if (activePluginWorkflow) { setKeywordComponent([]); setSelectedIndex(-1); }
                setInputValue(event.target.value);
              }
            }}
            onKeyDown={handleKeyDown}
            onKeyUp={() => {
              setFnDown(false);
            }}
            onCompositionStart={() => {
              searchRequestId.current += 1;
              if (activePluginWorkflow) { setKeywordComponent([]); setSelectedIndex(-1); }
              setIsComposing({ status: true, ppos: 0 });
            }}
            onCompositionEnd={(event) => {
              setIsComposing({ status: false, ppos: 1 });
              setInputValue(event.target.value);
            }}
          />
        </div>
        {/*<button onClick={() => inputBox.current.focus()}>aa</button>*/}
        {/*{keywordComponent ? TemplateComponent(keywordComponent, selectedIndex, setSelectedIndex, confirmComponentSelected, fnDown) : null}*/}
        {keywordComponent ? (
          <Suspense fallback={<div>Loading...</div>}>
            <TemplateComponent
              {...{
                components: keywordComponent,
                selectedKey: selectedIndex,
                setSelectedKey: setSelectedIndex,
                confirmSelected: confirmComponentSelected,
                fnDown,
              }}
            />
          </Suspense>
        ) : null}
        {componentInfo?.type === "panel" && componentInfo.pluginManifest ? (
          <PluginView
            manifest={componentInfo.pluginManifest}
            input={componentInfo.input || ""}
            onClose={() => initStatus()}
          />
        ) : componentInfo ? (
          <SubpageComponent component={componentInfo} keyDown={keyDown}
            pluginLibraryProps={componentInfo.data === "showComponent" ? {
              panelDropHandlerRef,
              plugins: pluginList,
              pluginStatus,
              loading: pluginsLoading,
              error: pluginsError,
              onRefresh: refreshPlugins,
              onToggle: togglePlugin,
              onClose: () => { initStatus(); inputBox.current?.focus(); },
              pluginConfigId: componentInfo.pluginConfigId,
            } : undefined}
          />
        ) : null}
      </div>
    </div>
  );
};

export default App;

