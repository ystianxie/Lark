import React, {useState, useRef, useEffect} from 'react';
import {createGlobalStyle} from 'styled-components';
import {List, Avatar} from 'antd';
import {invoke} from "@tauri-apps/api/core";
import InfiniteScroll from 'react-infinite-scroll-component';
import {getMaterialFileIcon, getMaterialFolderIcon} from "file-extension-icon-js";
import baseComponent from "../baseComponent.jsx";
import {getCurrentWindow} from "@tauri-apps/api/window";
import {modifyWindowSize} from "../template.jsx";

const Wrapper = createGlobalStyle`
    a{
        font-size:13px;
    }
    
    #settingframe{
        display: "flex";
        justify-content: "center";
        align-content:"colum";
    }
    
    .settingInput{
        font-size:15px;
        height:35px;
        width:200px;
 
    }
   
    .settingInput:focus {
        outline: none;
        box-shadow: none;
    }
   .settingSmallFrame {
        height: 80px;
        width: 150px;
        background-color: #cbcbcb;
        display: flex;
        justify-content: center;
        align-items: center;
        margin: 4px 4px 2px 4px;
        flex-direction: column;
        border-radius: 10px;
    }
    .hotkeys-input {
      border: 1px solid #d9d9d9;
      padding: 4px 11px;
      border-radius: 2px;
      outline: none;
      white-space: pre-wrap;
      background-color:white;
      width:250px;
      border-radius:6px;
      font-size:18px;
      height:20px
    }
    
    .hotkeys-input:empty:before {
      content: attr(placeholder);
      color: #bfbfbf;
    }
    .hotkeys-input:focus {
      outline: none;
      border-color: #1890ff; /* 选中时的边框颜色 */
    }
    
    .highlight {
      color: black; 
    }
    .lowlight {
        color: #bfbfbf;
    }
    
    .hotkeysFrame {
        display: flex;
        justify-content: center;
        flex-direction: column;
        align-items: center;
        margin-top: 2px;
    }
    .hotkeys-item {
        display: flex;
        justify-content: center;
        align-items: center;
        margin-top: 2px;
        height: 40px;
    }
    .clipboard-list-surface {
        position: relative;
        width: 50%;
        height: 100%;
        min-width: 0;
        overflow: hidden;
        border-top-left-radius: 10px;
        border-bottom-left-radius: 10px;
        background: #242424;
    }
    .clipboard-list-surface #scrollableDiv {
        position: relative;
        box-sizing: border-box;
        width: 100% !important;
        background: #242424;
    }
    .panelSubPage .clipboard-list-surface #scrollableDiv {
        scrollbar-color: rgba(0, 0, 0, 0.42) transparent;
        scrollbar-width: thin;
    }
    .panelSubPage .clipboard-list-surface #scrollableDiv::-webkit-scrollbar {
        width: 4px;
        height: 4px;
    }
    .panelSubPage .clipboard-list-surface #scrollableDiv::-webkit-scrollbar-track {
        background: transparent;
    }
    .panelSubPage .clipboard-list-surface #scrollableDiv::-webkit-scrollbar-thumb {
        background: rgba(0, 0, 0, 0.32);
        border-radius: 6px;
    }
    .panelSubPage .clipboard-list-surface #scrollableDiv::-webkit-scrollbar-thumb:hover {
        background: rgba(0, 0, 0, 0.52);
    }
    .clipboard-list-surface .ant-list,
    .clipboard-list-surface .ant-list-items {
        background: #242424;
    }
    .clipboard-list-surface .ant-list-item {
        padding-right: 48px !important;
        background-color: #242424;
    }
    .clipboard-list-surface .ant-list-item:hover:not(.activate) {
        background-color: #2d2d2d;
    }
    .clipboard-list-surface .ant-list-item.activate {
        background-color: #31575b;
        box-shadow: inset 2px 0 #70b9bf;
    }
    .clipboard-list-surface .clipboard-item {
        width: 100%;
        color: #c9c9cc;
    }
    .clipboard-list-surface .ant-list-item.activate .clipboard-item {
        color: #f0f3f3;
    }
    .clipboard-shortcuts {
        position: absolute;
        z-index: 2;
        top: 0;
        right: 12px;
        width: 34px;
        pointer-events: none;
        font-family: "Segoe UI Symbol", "Apple Symbols", sans-serif;
        font-size: 13px;
        font-variant-numeric: tabular-nums;
    }
    .clipboard-shortcut {
        display: flex;
        box-sizing: border-box;
        width: 100%;
        height: 35px;
        align-items: center;
        justify-content: center;
        line-height: 1;
        color: #777b82;
    }
    .clipboard-shortcut.activate {
        color: #e8f2f2;
        font-size: 14px;
    }
    .clipboard-list-surface .ant-list-item {
        cursor: pointer;
    }
    .clipboard-item{
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        -webkit-user-select: none;
        -moz-user-select: none;
        -ms-user-select: none;
        cursor: default;
        width: 70%;
        color: #d5d5d6;
    }
    .ant-list-item{
        box-sizing: border-box;
        justify-content: start !important;
        height: 35px !important;
        flex: none !important;
        width: 100%;
        padding: 0 10px !important;
        border-bottom: 1px solid rgba(255, 255, 255, 0.035) !important;
        color: #d5d5d6;
        transition: background-color 120ms ease;
    }
    .ant-list-item:hover:not(.activate){
        background-color: rgba(255, 255, 255, 0.045);
    }
    .ant-list-item-meta{
        flex: none !important;
        margin-inline-end: 10px !important;
    }
    .ant-list-item-meta-avatar{
        display: flex;
        align-items: center;
    }
    .ant-list-item-meta-avatar .ant-avatar{
        border-radius: 8px;
        background: rgba(255, 255, 255, 0.08);
    }
    .ant-list-item.activate{
        background-color: rgba(58, 127, 135, 0.38);
        box-shadow: inset 2px 0 #62aeb4;
    }
    .clipboard-item-wrapper{
        display: flex;
        justify-content: space-between;
        width: 100%;
        overflow: hidden;
        align-items: center;
    }
    #showCopyContent {
        box-sizing: border-box;
        display: flex;
        justify-content: space-between;
        flex-direction: column;
        width: 50%;
        padding: 14px 16px 12px;
        background-color: #242424;
        color: #d5d5d6;
        border-left: 1px solid rgba(255, 255, 255, 0.08);
    }
    .panelSubPage .clipboard-preview {
        scrollbar-color: rgba(0, 0, 0, 0.42) transparent;
        scrollbar-width: thin;
    }
    .panelSubPage .clipboard-preview::-webkit-scrollbar {
        width: 6px;
        height: 6px;
    }
    .panelSubPage .clipboard-preview::-webkit-scrollbar-track {
        background: transparent;
    }
    .panelSubPage .clipboard-preview::-webkit-scrollbar-thumb {
        background: rgba(0, 0, 0, 0.42);
        border-radius: 6px;
    }
    .panelSubPage .clipboard-preview::-webkit-scrollbar-thumb:hover {
        background: rgba(0, 0, 0, 0.62);
    }
    .clipboard-preview{
        box-sizing: border-box;
        width: 100%;
        min-height: 0;
        flex: 1;
        overflow: auto;
        padding: 12px;
        border-radius: 10px;
        background-color: rgba(255, 255, 255, 0.045);
        color: #dedee0;
        font-size: 14px;
        line-height: 1.55;
        overflow-wrap: anywhere;
    }
    .clipboard-preview img{
        display: block;
        object-fit: contain;
        margin: auto;
    }
    .clipboard-preview-file{
        display: flex;
        align-items: center;
        gap: 9px;
        min-height: 28px;
        color: #dedee0;
    }
    .clipboard-preview-file img{
        flex: none;
        width: 18px !important;
        height: 18px;
        object-fit: contain;
        margin: 0 !important;
    }
    .clipboard-preview-file span{
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        flex: 1;
        min-width: 0;
    }
    .clipboard-preview-empty{
        width: 100%;
        height: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
        color: #8f9095;
        font-size: 14px;
    }
    .clipboard-metadata{
        display: flex;
        justify-content: space-between;
        gap: 10px;
        padding-top: 10px;
        color: #929399;
        font-size: 11px;
        font-variant-numeric: tabular-nums;
        white-space: nowrap;
    }
`

const ClipboardComponent = ({onKeyDown, onClose}) => {
    const [initLoading, setInitLoading] = useState(true);
    const [loading, setLoading] = useState(false);
    const [data, setData] = useState([]);
    const [list, setList] = useState([]);
    const [offset, setOffset] = useState(0);
    const [frameHeight, setFrameHeight] = useState(0);
    const [selectIndex, setSelectIndex] = useState(0);
    const scrollContainerRef = useRef(null);
    const selectIndexRef = useRef(0);
    const firstItemIndexRef = useRef(0);
    const dataRef = useRef(data);
    const listRef = useRef(list);
    const [shortcutViewport, setShortcutViewport] = useState({firstIndex: 0, offset: 0});
    dataRef.current = data;
    listRef.current = list;

    const selectClipboardItem = (index) => {
        selectIndexRef.current = index;
        setSelectIndex(index);
    };

    const syncShortcutViewport = (scrollTop) => {
        const firstIndex = Math.floor(scrollTop / 35);
        const offset = scrollTop % 35;
        firstItemIndexRef.current = firstIndex;
        setShortcutViewport((current) =>
            current.firstIndex === firstIndex && current.offset === offset
                ? current
                : {firstIndex, offset}
        );
    };

    useEffect(() => {
        // 初始化剪贴板内容
        invoke("get_history_part", {limit: 30, offset: 0})
            .then((res) => {
                console.log('初始化剪贴板内容', res)
                setInitLoading(false);
                setData(res);
                setList(res);
                setOffset(30)
                let frame = document.getElementById("subPageFrame")
                setFrameHeight(frame.clientHeight)
            });

    }, []);

    async function confirmClipboardContent(itemIndex = selectIndexRef.current, waitForModifierRelease = false) {
        // 确认剪贴板内容
        const item = dataRef.current?.[itemIndex];
        if (item && !item.loading) {
            selectClipboardItem(itemIndex);
            await getCurrentWindow().hide();
            // await modifyWindowSize("compact");


            // Alt+数字触发时，等待修饰键释放，避免模拟粘贴仍处于 Alt 状态而被系统当成菜单快捷键。
            if (waitForModifierRelease) {
                await new Promise((resolve) => setTimeout(resolve, 150));
            }

            // 历史列表使用 content_preview；粘贴时必须按 id 读取完整 content。
            const fullItem = await invoke("get_history_id", {id: item.id});
            let content = fullItem?.content || "";
            const dataType = fullItem?.data_type || item.data_type || "";

            if (dataType === "image") {
                content = JSON.parse(content).base64
            } else if (dataType === "file") {
                // 传递完整文件列表，支持多文件/文件夹粘贴。
                content = JSON.parse(content).files
            }
            const result = await invoke("clipboard_control", {
                text: content,
                control: "write",
                paste: true,
                dataType
            });
            console.log('确认剪贴板内容', result);
            onClose?.();
        }
    }

    useEffect(() => {
        // 快捷提示和快捷操作都以当前滚动视口最上方的行作为序号起点。
        if (!onKeyDown || !onKeyDown.key) return;
        const items = listRef.current;
        const currentIndex = selectIndexRef.current;
        if (onKeyDown.key === "ArrowUp" && items.length) {
            selectClipboardItem(currentIndex > 0 ? currentIndex - 1 : items.length - 1);
        } else if (onKeyDown.key === "ArrowDown" && items.length) {
            selectClipboardItem(currentIndex < items.length - 1 ? currentIndex + 1 : 0);
        } else if (onKeyDown.key === "Enter" && !initLoading) {
            confirmClipboardContent(currentIndex);
        } else if (onKeyDown.metaKey || onKeyDown.altKey) {
            // Windows 下 Alt+数字的 key 在不同输入法/浏览器环境中可能不是纯数字，优先使用 code。
            const shortcutMatch = onKeyDown.code?.match(/^Digit([1-9])$/) || onKeyDown.key?.match(/^([1-9])$/);
            const shortcutNumber = shortcutMatch ? Number(shortcutMatch[1]) : 0;
            const targetIndex = firstItemIndexRef.current + shortcutNumber - 1;
            if (shortcutNumber && targetIndex >= 0 && targetIndex < dataRef.current.length) {
                void confirmClipboardContent(targetIndex, true);
            }
        }
    }, [onKeyDown]);

    useEffect(() => {
        // 只在选择项离开可视范围时滚动，避免平滑滚动期间快捷标记与行错位。
        const scrollContainer = scrollContainerRef.current;
        const selectedItem = scrollContainer?.querySelector(`[data-index="${selectIndex}"]`);
        if (!scrollContainer || !selectedItem) return;

        const itemTop = selectedItem.offsetTop;
        const itemBottom = itemTop + selectedItem.offsetHeight;
        if (itemTop < scrollContainer.scrollTop) {
            scrollContainer.scrollTop = itemTop;
        } else if (itemBottom > scrollContainer.scrollTop + scrollContainer.clientHeight) {
            scrollContainer.scrollTop = itemBottom - scrollContainer.clientHeight;
        }
        syncShortcutViewport(scrollContainer.scrollTop);
    }, [selectIndex]);

    const onLoadMore = () => {
        setLoading(true);
        setList(
            list.concat(
                Array.from({length: 1}).map(() => ({name: '', loading: true, content: 'Loading...'})),
            ),
        );
        invoke("get_history_part", {limit: 20, offset: offset})
            .then((res) => {
                console.log('获取后20条数据', res)
                if (res) {
                    setData(data.concat(res));
                    setOffset(offset + 20);
                    setList(data.concat(res));
                    setLoading(false);
                }
            });
    }

    function showHotkeys(index) {
        const itemIndex = shortcutViewport.firstIndex + index;
        if (selectIndex === itemIndex) return "⏎";
        return index < 9 ? "⌘" + (index + 1) : "";
    }

    function timestampToTime(timestamp) {
        timestamp = timestamp ? timestamp : null;
        let date = new Date(timestamp);//时间戳为10位需*1000，时间戳为13位的话不需乘1000
        let Y = date.getFullYear() + '-';
        let M = (date.getMonth() + 1 < 10 ? '0' + (date.getMonth() + 1) : date.getMonth() + 1) + '-';
        let D = (date.getDate() < 10 ? '0' + date.getDate() : date.getDate()) + ' ';
        let h = (date.getHours() < 10 ? '0' + date.getHours() : date.getHours()) + ':';
        let m = (date.getMinutes() < 10 ? '0' + date.getMinutes() : date.getMinutes()) + ':';
        let s = date.getSeconds() < 10 ? '0' + date.getSeconds() : date.getSeconds();
        return Y + M + D + h + m + s;
    }

    function handleContentPreview(content) {
        if (!content) return <div>Choose to view more</div>
        if (content.data_type === "text") {
            return <div>{content.content}</div>
        } else if (content.data_type === "image") {
            console.log(content)
            return (<img src={"data:image/jpeg;base64," + JSON.parse(content.content)?.base64}
                         style={{maxWidth: "100%", maxHeight: "100%"}}></img>)
        } else if (content.data_type === "file") {
            let content_ = JSON.parse(JSON.parse(content.content)?.files)
            console.log("content-", content_)
            let max_length = 5
            let fontSize = "16px"
            for (let file of content_) {
                if (file[0].length > max_length) {
                    max_length = file[0].length
                }
            }
            if (max_length > 50) {
                fontSize = "13px"
            }

            return (content_.map((item, index) => (
                <div className="clipboard-preview-file" key={`${item[0]}-${item[1]}-${index}`}>
                    <img src={item[1] !== "folder" ? getMaterialFileIcon(item[1]) : getMaterialFolderIcon(item[1])}/>
                    <span style={{fontSize}}>{item[0]}</span>
                </div>
            )))
        }
    }

    function handleContentOption(content) {
        if (!content) return ""
        if (content.data_type === "text") {
            return content.content
        } else if (content.data_type === "image") {
            return JSON.parse(content.content)?.title
        } else if (content.data_type === "file") {
            return JSON.parse(content.content)?.title
        }
    }

    return (
        <>
            <Wrapper/>
            <div style={{display: "flex", justifyContent: "center", flexDirection: "row", overflow: "hidden"}}>
                <div className="clipboard-list-surface">
                    <div id="scrollableDiv"
                     style={{
                         height: frameHeight,
                         width: "100%",
                         overflow: 'auto',
                         borderTopLeftRadius: "10px",
                         borderBottomLeftRadius: "10px",
                     }}
                     ref={scrollContainerRef}
                     onScroll={(event) => syncShortcutViewport(event.currentTarget.scrollTop)}
                >
                    <InfiniteScroll
                        dataLength={list.length}
                        next={onLoadMore}
                        hasMore={!loading}
                        scrollableTarget="scrollableDiv"
                    >
                        <List
                            className="demo-loadmore-list"
                            itemLayout="horizontal"
                            size="small"
                            dataSource={list}
                            renderItem={
                                (item, index) => (
                                    <List.Item className={selectIndex === index ? "activate" : ""}
                                               data-index={index}
                                               onMouseEnter={() => selectClipboardItem(index)}
                                               onClick={() => confirmClipboardContent(index)}
                                               role="option"
                                               aria-selected={selectIndex === index}>
                                        <List.Item.Meta
                                            avatar={
                                                <Avatar
                                                    src={`data:image/png;base64,${item.app_icon}`}/>
                                            }
                                        />
                                        <div className={"clipboard-item-wrapper"}>
                                            <div className="clipboard-item"> {handleContentOption(item)}</div>


                                        </div>
                                    </List.Item>

                                )
                            }
                        />
                    </InfiniteScroll>
                    </div>
                    <div className="clipboard-shortcuts" style={{transform: `translateY(-${shortcutViewport.offset}px)`}}>
                        {Array.from({length: 15}).map((item, index) => (
                            <div className={`clipboard-shortcut${selectIndex === shortcutViewport.firstIndex + index ? " activate" : ""}`} key={index}>
                                {showHotkeys(index)}
                            </div>
                        ))}
                    </div>
                </div>

                <div id={"showCopyContent"} style={{height: frameHeight}}>
                    {data[selectIndex] ? <>
                        <div className="clipboard-preview">
                            {handleContentPreview(data[selectIndex])}
                        </div>
                        <div className="clipboard-metadata">
                            <span>{timestampToTime(data[selectIndex].create_time)}</span>
                            <span>{data[selectIndex].content.split("\n").length} 行 · {data[selectIndex].content.length} 字符</span>
                        </div>
                    </> : <div className="clipboard-preview-empty">选择一条记录查看内容</div>}
                </div>
            </div>
        </>
    )

}

export default ClipboardComponent;
