import React from 'react';
import styled, {createGlobalStyle} from 'styled-components';
import {distance} from "mathjs";
import {Button, Input, InputNumber, Checkbox, Flex} from 'antd';

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
        background-color: red;
        display: flex;
        justify-content: center;
        align-items: center;
        margin-top: 2px;
        flex-direction: column;
        border-radius: 10px;
    }
}
`

const Component = () => {
    const [clipboardCount, setClipboardCount] = React.useState(30);
    const [clipboardText, setClipboardText] = React.useState(7);
    const [clipboardImage, setClipboardImage] = React.useState(2);
    const [clipboardFile, setClipboardFile] = React.useState(1);

    const handleSettingReset = (value) => {
        setClipboardCount(30);
        setClipboardText(7);
        setClipboardImage(2);
        setClipboardFile(1);

        handleSettingSave()
    }
    const handleSettingSave = (value) => {
        //TODO: 保存设置
    }
    return (
        <>
            <Wrapper/>
            <div id="settingframe">
                <a style={{display: "flex", justifyContent: 'center'}}>设置</a>
                <div>
                    <a style={{marginLeft: "15px"}}>剪贴板历史</a>
                    <div className="settingSmallFrame">
                        <Checkbox>数量(个)</Checkbox>
                        <InputNumber style={{fontSize: "10px"}} placeholder={clipboardCount} size="small" min={10}
                                     max={200} defaultValue={clipboardCount} changeOnWheel/>
                    </div>

                    <div style={{display: 'flex', justifyContent: 'space-around', marginTop: '2px'}}>
                        <a>内容保存数量</a>
                        <InputNumber className="settingInput" placeholder={clipboardCount} min={10} max={200}
                                     defaultValue={clipboardCount} changeOnWheel/>
                    </div>

                    <div style={{display: 'flex', justifyContent: 'space-around', marginTop: '2px'}}>
                        <a>文本保存时长</a>
                        <InputNumber className="settingInput" placeholder={clipboardText} min={1} max={30}
                                     defaultValue={clipboardText} changeOnWheel/>
                    </div>

                    <div style={{display: 'flex', justifyContent: 'space-around', marginTop: '2px'}}>
                        <a>图片保存时长</a>
                        <InputNumber className="settingInput" placeholder={clipboardImage} min={1} max={10}
                                     defaultValue={clipboardImage} changeOnWheel/>
                    </div>
                    <div style={{
                        display: 'flex',
                        justifyContent: 'space-around',
                        marginTop: '2px',
                        marginBottom: "20px"
                    }}>
                        <a>文件保存时长</a>
                        <InputNumber className="settingInput" placeholder={clipboardFile} min={1} max={10}
                                     defaultValue={clipboardFile} changeOnWheel/>
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