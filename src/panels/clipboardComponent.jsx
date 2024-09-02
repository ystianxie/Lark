import React, {useState, useRef, useEffect, useCallback} from 'react';
import {createGlobalStyle} from 'styled-components';
import {List, Avatar} from 'antd';
import {invoke} from "@tauri-apps/api/tauri";
import InfiniteScroll from 'react-infinite-scroll-component';
import throttle from 'lodash/throttle';
import {getMaterialFileIcon, getMaterialFolderIcon} from "file-extension-icon-js";
import baseComponent from "../baseComponent.jsx";
import {debounce} from "lodash/function.js";

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
    .clipboard-item{
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        -webkit-user-select: none;
        -moz-user-select: none;
        -ms-user-select: none;
        cursor: default;
        width: 70%;
    }
    .activate-item{
        background-color: #3a7f87;
    }
    .ant-list-item{
        justify-content: start !important;
        height: 35px !important;
        flex: none !important;
        width: 100%;
    }
    .ant-list-item-meta{
        flex: none !important;
        
    }
   .clipboard-item-wrapper{
       display: flex;
       justify-content: space-between;
       width: 100%;
       overflow: hidden;
   }
   #showCopyContent {
        display: flex;
        justify-content: space-between;
        flex-direction: column;
        width: 50%;
        background-color: rgb(180, 176, 176);
   }
}
`

const ClipboardComponent = ({onKeyDown}) => {
    const [initLoading, setInitLoading] = useState(true);
    const [loading, setLoading] = useState(false);
    const [data, setData] = useState([]);
    const [list, setList] = useState([]);
    const [offset, setOffset] = useState(0);
    const [frameHeight, setFrameHeight] = useState(0);
    const [selectIndex, setSelectIndex] = useState(0);
    const scrollContainerRef = useRef(null);
    const selectIndexRef = useRef(selectIndex);
    const [firstItemIndex, setFirstItemIndex] = useState(0);
    useEffect(() => {
        selectIndexRef.current = selectIndex;
    }, [selectIndex]);

    const get_first_item = () => {
        let list_items = document.getElementsByClassName("ant-list-item")
        let cache = -1
        for (let i = 0; i < list_items.length; i++) {
            if (list_items[i].getBoundingClientRect().y > 32) {
                return [i, -1]
            } else if (list_items[i].getBoundingClientRect().y > 0 && cache === -1) {
                cache = i
            }
        }
        return [-1, cache !== -1 ? cache : 0]
    }
    const closeDefault = useCallback(throttle((deltaY) => {
            // 滚动事件限制触发频率，并固定滚动距离
            const scrollContainer = scrollContainerRef.current;
            if (deltaY > 5) {
                scrollContainer.scrollTop = scrollContainer.scrollTop + 35;
            } else if (deltaY < -5) {
                scrollContainer.scrollTop = scrollContainer.scrollTop - 35;
            }
            let index = get_first_item()
            if (index[0] !== -1) {
                setFirstItemIndex(index[0])
            }
        }, 100)
        , [selectIndex])

    const handleScroll = useCallback((e) => {
        // 滚动事件处理，禁用原有滚动行为，并触发自定义滚动逻辑
        e.preventDefault();
        e.stopPropagation();
        closeDefault(e.deltaY)
    }, [closeDefault])

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
                scrollContainerRef.current.addEventListener('wheel', handleScroll, {passive: false})
            });

    }, []);

    function changeFirstItemIndex(additional) {
        // 按键处理 可视区域内第一项，找到其索引 用作快捷键提示
        let list_items = document.getElementsByClassName("ant-list-item")
        for (let i = 0; i < list_items.length; i++) {
            if (additional < 0) {
                //  上移
                if (list_items[i].getBoundingClientRect().y === 30.5) {
                    if (list_items[selectIndex].getBoundingClientRect().y !== 65.5) {
                        return setFirstItemIndex(i + 1)
                    }
                    return setFirstItemIndex(i)
                } else if (selectIndex === 0) {
                    return setFirstItemIndex(list_items.length > 15 ? list_items.length - 15 : 0);
                }
            } else {
                // 下移
                if (list_items[i].getBoundingClientRect().y === 65.5) {
                    if (i + 14 === selectIndex) {
                        return setFirstItemIndex(i + 1)
                    } else {
                        return setFirstItemIndex(i)
                    }
                } else if (selectIndex === list.length - 1) {
                    return setFirstItemIndex(0) || 0;
                }
            }
        }
    }

    function confirmClipboardContent() {
        // 确认剪贴板内容
        if (data) {
            invoke("clipboard_control", {
                text: data[selectIndex]?.content || "",
                control: "copy",
                paste: true,
                dataType: ""
            })
                .then((res) => {
                    console.log('确认剪贴板内容', res)
                });
        }
    }

    useEffect(() => {
        // 当按下键盘时，处理内容
        if (onKeyDown.key === "ArrowUp") {
            setSelectIndex(selectIndex > 0 ? selectIndex - 1 : list.length - 1)
            changeFirstItemIndex(-1)
        } else if (onKeyDown.key === "ArrowDown") {
            setSelectIndex(selectIndex < list.length - 1 ? selectIndex + 1 : 0)
            changeFirstItemIndex(1)
        } else if (onKeyDown.key === "Enter" && !initLoading) {
            confirmClipboardContent()
        }
    }, [onKeyDown]);

    useEffect(() => {
        //  更改当前选择项时 将其滚动到可视区域
        const scrollContainer = scrollContainerRef.current;
        const selectedItem = scrollContainer.querySelector(`[data-index="${selectIndex}"]`);
        if (selectedItem) {
            selectedItem.scrollIntoView({
                behavior: 'smooth',
                block: 'nearest'
            });
        }

        // 防止因快速切换导致行元素对齐出现偏移
        function standardizedDisplay() {
            let index = get_first_item()
            let item = document.getElementsByClassName("ant-list-item")[index[1]]
            if (item && item.getBoundingClientRect().y !== 32) {
                scrollContainerRef.current.scrollTop -= 32 - item.getBoundingClientRect().y
            }
        }

        const debouncedStandardizedDisplay = debounce(standardizedDisplay, 100)
        debouncedStandardizedDisplay()
        return () => {
            debouncedStandardizedDisplay.cancel()
        }
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
        // 快捷键提示文本
        if (selectIndex === firstItemIndex + index) {
            return "⏎"
        }
        if (index < 9) {
            return "⌘" + (index + 1)
        }
        return ""
    }

    function showHotkeysColor(index) {
        // 快捷键提示颜色
        if (selectIndex === firstItemIndex + index) {
            return "#fff"
        }
        if (index === 0) {
            return "rgb(74,73,73)"
        } else if (index > 7) {
            return "rgb(200,199,199)"
        } else if (index > 5) {
            return "rgb(151,150,150)"
        } else if (index > 0) {
            return "rgb(120,120,120)"

        }
    }

    const handleMouseEnter = (e, index) => {
        if (e.movementX !== 0 || e.movementY !== 0) {
            setSelectIndex(index);
        }
    };

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
            return <div style={{overflow: "hidden", textOverflow: "ellipsis"}}>{content.content}</div>
        } else if (content.data_type === "image") {
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

            return (content_.map(item => (
                <div key={item[1]} style={{display: "flex"}}>
                    <img src={item[1] !== "folder" ? getMaterialFileIcon(item[1]) : getMaterialFolderIcon(item[1])}
                         style={{overflow: "hidden", width: fontSize, marginRight: "10px"}}></img>
                    <div style={{overflow: "hidden", fontSize}}>{item[0]}</div>
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
                <div id="scrollableDiv"
                     style={{
                         height: frameHeight,
                         width: "50%",
                         overflow: 'auto',
                         borderTopLeftRadius: "10px",
                         borderBottomLeftRadius: "10px"
                     }}
                     ref={scrollContainerRef}
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
                                               data-index={index}>
                                        <List.Item.Meta
                                            avatar={
                                                <Avatar
                                                    src={`data:image/png;base64,${item.app_icon}`}/>
                                            }
                                        />
                                        <div className={"clipboard-item-wrapper"}
                                             onMouseEnter={(event) => handleMouseEnter(event, index)}
                                             onClick={confirmClipboardContent}>
                                            <div className="clipboard-item"> {handleContentOption(item)}</div>


                                        </div>
                                    </List.Item>

                                )
                            }
                        />
                        <div style={{
                            position: "absolute",
                            right: "52%",
                            top: "12.5%",
                            marginLeft: "10px auto",
                            fontSize: "16px"
                        }}>
                            {

                                Array.from({length: 15}).map(
                                    (item, index) => (
                                        <div style={{
                                            flexShrink: 0,
                                            minWidth: "20px",
                                            height: "35px",
                                            color: showHotkeysColor(index)
                                        }} key={index}>
                                            {showHotkeys(index)}
                                        </div>
                                    )
                                )
                            }
                        </div>


                    </InfiniteScroll>
                </div>

                <div id={"showCopyContent"} style={data[selectIndex] ? {height: frameHeight} : {
                    height: frameHeight,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center"
                }}>
                    <div style={{
                        fontSize: "15px",
                        margin: "5px 5px 5px 5px",
                        whiteSpace: "pre-wrap", height: '90%', overflow: "hidden"
                    }}>
                        {handleContentPreview(data[selectIndex])}
                    </div>
                    <div style={{
                        fontSize: "15px",
                        display: 'flex',
                        flexDirection: "column",
                        alignItems: "center",
                        overflow: "hidden",
                        "flex": "1"
                    }}>
                        <div style={{overflow: "hidden"}}>
                            {data[selectIndex] ? "time：" + timestampToTime(data[selectIndex].create_time) : ""}
                        </div>
                        <div>
                            {data[selectIndex] ? "row：" + data[selectIndex].content.split("\n").length + " char：" + data[selectIndex].content.length : ""}
                        </div>
                    </div>
                </div>
            </div>
        </>
    )

}

export default ClipboardComponent;