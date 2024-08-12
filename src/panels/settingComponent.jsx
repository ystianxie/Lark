import React, {useState, useRef, useEffect, useReducer} from 'react';
import styled, {createGlobalStyle} from 'styled-components';
import {distance} from "mathjs";
import {Button, Input, InputNumber, Checkbox, Flex} from 'antd';
import {invoke} from "@tauri-apps/api/tauri";
import {event} from "@tauri-apps/api";

const Wrapper = createGlobalStyle`
    a{
        font-size:20px;
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
}
`

const Component = () => {
    const [clipboardCount, setClipboardCount] = useState(100);
    const [clipboardText, setClipboardText] = useState(10);
    const [clipboardImage, setClipboardImage] = useState(5);
    const [clipboardFile, setClipboardFile] = useState(1);

    const [larkDisplayText, setLarkDisplayText] = useReducer(hotkeysFrameShow, {element: '',downKey:{
            alt: true,
            meta: false,
            ctrl: false,
            shift: false,
            key: "Space"
        }
    });
    const [cbdDisplayText, setCBDDisplayText] = useReducer(hotkeysFrameShow, {element: ''});


    const handleSettingReset = async () => {
        setClipboardCount(100);
        setClipboardText(10);
        setClipboardImage(5);
        setClipboardFile(1);

        await handleSettingSave()
    }
    const handleSettingSave = async () => {
        //TODO: 保存设置
        console.log("保存设置")
        let all_setting = {
            clipboardCount,
            clipboardText,
            clipboardImage,
            clipboardFile
        }
        await invoke("save_setting", {settingInfo: all_setting})
    }


    const handleHotkeysDown = (event, name) => {
        event.preventDefault();  // 防止默认行为
        let downKey = {
            alt: event.altKey,
            meta: event.metaKey,
            ctrl: event.ctrlKey,
            shift: event.shiftKey,
            key: event.key.length === 1 ? event.key : ""
        }
        if (event.code === "Space") {
            downKey.key = "Space"
        }
        if (name === "lark") {
            setLarkDisplayText({behavior: "down", data: downKey})
        } else if (name === "cbd") {
            setCBDDisplayText({behavior: "down", data: downKey})
        }
    }

    const handleHotkeysUp = (event, name) => {
        let keyMap = {
            Control: "ctrl",
            Alt: "alt",
            Shift: "shift",
            Meta: "meta",
        }
        let upKey = {
            key: keyMap[event.key] || event.key,
        }
        if (event.code === "Space") {
            upKey.key = "Space"
        }
        if (name === "lark") {
            setLarkDisplayText({behavior: "up", data: upKey})
        } else if (name === "cbd") {
            setCBDDisplayText({behavior: "up", data: upKey})
        }
    }

    function hotkeysFrameShow(stats, action) {
        const keyMap = {
            ctrl: '⌃',
            alt: '⌥',
            shift: '⇧',
            meta: '⌘',
        };
        let showText = ""
        let downKey = {}
        if (action.behavior === "down") {
            downKey = action.data

        } else if (action.behavior === "up") {
            downKey = stats.downKey
            if (!downKey.key || !(downKey.ctrl || downKey.alt || downKey.shift || downKey.meta)) {
                if (keyMap.hasOwnProperty(action.data.key)) {
                    downKey[action.data.key] = false
                }
                if (downKey.key === action.data.key) {
                    downKey.key = ""
                }
            } else {
                // todo 处理组合键
            }

        }
        for (let char of Object.keys(keyMap)) {
            if (downKey[char]) {
                showText += `<span class="highlight">${keyMap[char]}</span>`
            } else {
                showText += `<span class="lowlight">${keyMap[char]}</span>`
            }
        }
        if (downKey.key) {
            let showKey = downKey.key.toUpperCase()
            if (downKey.key === "Space") {
                showKey = "␣"
            }
            showText += `<span class="highlight">${showKey}</span>`
        }
        return {
            element: showText,
            downKey: downKey
        };
    }

    return (
        <>
            <Wrapper/>
            <div id="settingframe">
                <a style={{display: "flex", justifyContent: 'center'}}>设置</a>
                <a style={{marginLeft: "15px"}}>快捷键</a>
                <div className="hotkeysFrame">
                    <div className="hotkeys-item">
                        <a style={{fontSize: "17px", width: "70px"}}>百灵鸟</a>
                        <div
                            contentEditable
                            placeholder="⌃⌥⇧⌘"
                            dangerouslySetInnerHTML={{__html: larkDisplayText.element}}
                            className="hotkeys-input"
                            onKeyDown={(event) => handleHotkeysDown(event, 'lark')}
                            onKeyUp={(event) => handleHotkeysUp(event, 'lark')}
                        />
                    </div>
                    <div className="hotkeys-item">
                        <a style={{fontSize: "17px", width: "70px"}}>剪贴板</a>
                        <div
                            contentEditable
                            placeholder="⌃⌥⇧⌘"
                            dangerouslySetInnerHTML={{__html: cbdDisplayText.element}}
                            className="hotkeys-input"
                            onKeyDown={(event) => handleHotkeysDown(event, 'cbd')}
                            onKeyUp={(event) => handleHotkeysUp(event, 'cbd')}
                        />
                    </div>
                </div>

                <a style={{marginLeft: "15px"}}>剪贴板历史</a>
                <div style={{display: "flex", justifyContent: 'center', marginTop: '2px'}}>
                    <div className="settingSmallFrame">
                        <Checkbox defaultChecked={true}>数量(个)</Checkbox>
                        <InputNumber style={{fontSize: "10px"}} placeholder={clipboardCount} size="small" min={10}
                                     max={200} defaultValue={clipboardCount} changeOnWheel/>
                    </div>
                    <div className="settingSmallFrame">
                        <Checkbox>文本(天)</Checkbox>
                        <InputNumber style={{fontSize: "10px"}} placeholder={clipboardText} size="small" min={1}
                                     max={30}
                                     defaultValue={clipboardText} changeOnWheel/>
                    </div>
                    <div className="settingSmallFrame">
                        <Checkbox>图片(天)</Checkbox>
                        <InputNumber style={{fontSize: "10px"}} placeholder={clipboardImage} size="small" min={1}
                                     max={15} defaultValue={clipboardImage} changeOnWheel/>
                    </div>
                    <div className="settingSmallFrame">
                        <Checkbox>文件(天)</Checkbox>
                        <InputNumber style={{fontSize: "10px"}} placeholder={clipboardFile} size="small" min={1}
                                     max={10} defaultValue={clipboardFile} changeOnWheel/>
                    </div>
                </div>

                <div style={{display: "flex", justifyContent: "right", height: "35px"}}>
                    <Button style={{marginRight: "5px"}} onClick={handleSettingReset}>重置</Button>
                    <Button style={{marginRight: "10px"}} type="primary" onClick={handleSettingSave}>保存</Button>
                </div>
            </div>
        </>
    );
};

export default Component;