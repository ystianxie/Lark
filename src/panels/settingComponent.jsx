import React, {useState, useRef, useEffect, useReducer} from 'react';
import styled, {createGlobalStyle} from 'styled-components';
import {distance} from "mathjs";
import {Button, Input, InputNumber, Checkbox, Flex, Tabs, Tag, Popconfirm, Switch} from 'antd';
import {invoke} from "@tauri-apps/api/core";
import {open} from "@tauri-apps/plugin-dialog";
import {listen} from "@tauri-apps/api/event";

const Wrapper = createGlobalStyle`
    a{
        font-size:20px;
    }
    
    #settingframe{
        box-sizing: border-box;
        height: 100%;
        padding-bottom: 12px;
        overflow-y: auto;
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
        box-sizing: border-box;
        min-height: 78px;
        min-width: 0;
        background: #fafafa;
        border: 1px solid #ededed;
        display: flex;
        justify-content: center;
        align-items: center;
        flex-direction: column;
        gap: 8px;
        border-radius: 8px;
    }
    .hotkeys-input {
        box-sizing: border-box;
        display: flex;
        align-items: center;
        min-height: 42px;
        padding: 6px 10px;
        overflow: hidden;
        border: 1px solid #d9d9d9;
        border-radius: 8px;
        outline: none;
        background: #fff;
        cursor: text;
    }
    
    .hotkeys-input:empty:before {
      content: attr(placeholder);
      color: #bfbfbf;
    }
    .hotkeys-input:focus {
        border-color: #1677ff;
        box-shadow: 0 0 0 2px rgba(5, 145, 255, 0.1);
    }
    .hotkeyKeys {
        display: flex;
        align-items: center;
        gap: 6px;
        min-width: 0;
    }
    .hotkeyCap {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        min-width: 27px;
        height: 26px;
        padding: 0 7px;
        border: 1px solid #d6dbe3;
        border-bottom-width: 2px;
        border-radius: 6px;
        background: linear-gradient(#fff, #f7f8fa);
        color: #303846;
        font-family: "Segoe UI", sans-serif;
        font-size: 12px;
        font-weight: 600;
        line-height: 1;
        box-shadow: 0 1px 1px rgba(0, 0, 0, 0.04);
        white-space: nowrap;
    }
    .hotkeySeparator {
        color: #b8bec8;
        font-size: 12px;
        user-select: none;
    }
    .hotkeyPlaceholder {
        color: #bfbfbf;
        font-size: 13px;
    }
    .appSettingsPane {
        display: grid;
        gap: 12px;
        margin: 4px 15px 16px;
    }
    .appSettingCard {
        padding: 14px;
        border: 1px solid #e8e8e8;
        border-radius: 10px;
        background: #fff;
        box-shadow: 0 1px 2px rgba(0, 0, 0, 0.03);
    }
    .hotkeysFrame {
        display: grid;
        gap: 10px;
        margin-top: 12px;
    }
    .hotkeys-item {
        display: grid;
        grid-template-columns: minmax(110px, 0.32fr) minmax(220px, 1fr);
        align-items: center;
        gap: 14px;
    }
    .hotkeyName {
        color: #262626;
        font-size: 13px;
        font-weight: 600;
    }
    .hotkeyDescription {
        margin-top: 2px;
        color: #8c8c8c;
        font-size: 11px;
    }
    .clipboardRetentionGrid {
        display: grid;
        grid-template-columns: repeat(4, minmax(0, 1fr));
        gap: 8px;
        margin-top: 12px;
    }
    .appSettingFooter {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
    }
    .indexSettingsPane {
        display: grid;
        gap: 12px;
        margin: 4px 15px 16px;
    }
    .indexSettingsIntro {
        padding: 16px 18px;
        border: 1px solid #dce8ff;
        border-radius: 12px;
        background: linear-gradient(135deg, #f7faff 0%, #eef5ff 100%);
    }
    .indexSettingsTitle {
        margin: 0;
        color: #1f2937;
        font-size: 16px;
        font-weight: 650;
    }
    .indexSettingsDescription {
        margin: 5px 0 0;
        color: #64748b;
        font-size: 12px;
        line-height: 1.6;
    }
    .indexSettingsGrid {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 12px;
    }
    .indexSettingCard {
        min-width: 0;
        padding: 14px;
        border: 1px solid #e6eaf0;
        border-radius: 12px;
        background: #fff;
        box-shadow: 0 2px 8px rgba(31, 41, 55, 0.04);
    }
    .indexSettingHeader {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 12px;
        margin-bottom: 10px;
    }
    .indexSettingHeading {
        margin: 0;
        color: #262f3d;
        font-size: 13px;
        font-weight: 650;
    }
    .indexSettingHint {
        margin-top: 3px;
        color: #8a94a3;
        font-size: 11px;
        line-height: 1.45;
    }
    .indexSettingCount {
        flex: none;
        padding: 2px 8px;
        border-radius: 999px;
        background: #f0f5ff;
        color: #3b6edc;
        font-size: 11px;
        font-weight: 600;
    }
    .indexSettingList {
        box-sizing: border-box;
        min-height: 84px;
        max-height: 132px;
        padding: 9px;
        overflow-y: auto;
        border: 1px solid #e5e9ef;
        border-radius: 9px;
        background: #fafbfc;
    }
    .indexSettingList .ant-tag {
        max-width: 100%;
        margin: 0 6px 6px 0;
        overflow: hidden;
        border-color: #dbe5f5;
        border-radius: 6px;
        background: #fff;
        color: #46566c;
        text-overflow: ellipsis;
    }
    .indexSettingEmpty {
        display: flex;
        min-height: 64px;
        align-items: center;
        justify-content: center;
        color: #a0a8b4;
        font-size: 12px;
        text-align: center;
    }
    .indexSettingAdd {
        display: flex;
        gap: 8px;
        margin-top: 9px;
    }
    .indexSettingAdd .ant-input {
        min-width: 0;
    }
    .indexSettingsFooter {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        padding-top: 2px;
    }
    .indexSettingsFooterHint {
        color: #8c8c8c;
        font-size: 11px;
    }
    @media (max-width: 680px) {
        .indexSettingsGrid {
            grid-template-columns: 1fr;
        }
    }
    .customAppsPane {
        display: grid;
        gap: 12px;
        margin: 4px 15px 16px;
    }
    .customAppsIntro {
        padding: 16px 18px;
        border: 1px solid #dce8ff;
        border-radius: 12px;
        background: linear-gradient(135deg, #f7faff 0%, #eef5ff 100%);
    }
    .customAppsTitle {
        margin: 0;
        color: #1f2937;
        font-size: 16px;
        font-weight: 650;
    }
    .customAppsDescription {
        margin: 5px 0 0;
        color: #64748b;
        font-size: 12px;
        line-height: 1.6;
    }
    .customAppCard {
        padding: 14px;
        border: 1px solid #e6eaf0;
        border-radius: 12px;
        background: #fff;
        box-shadow: 0 2px 8px rgba(31, 41, 55, 0.04);
    }
    .customAppFormHeader,
    .customAppListHeader {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 12px;
        margin-bottom: 12px;
    }
    .customAppHeading {
        margin: 0;
        color: #262f3d;
        font-size: 13px;
        font-weight: 650;
    }
    .customAppHint {
        margin-top: 3px;
        color: #8a94a3;
        font-size: 11px;
        line-height: 1.45;
    }
    .customAppCount {
        flex: none;
        padding: 2px 8px;
        border-radius: 999px;
        background: #f0f5ff;
        color: #3b6edc;
        font-size: 11px;
        font-weight: 600;
    }
    .customAppForm {
        display: grid;
        grid-template-columns: minmax(140px, 0.35fr) minmax(220px, 1fr) auto;
        gap: 8px;
        align-items: center;
    }
    .customAppError {
        padding: 9px 11px;
        border: 1px solid #ffccc7;
        border-radius: 8px;
        background: #fff2f0;
        color: #cf1322;
        font-size: 12px;
    }
    .customAppList {
        max-height: 320px;
        overflow-y: auto;
        border: 1px solid #e5e9ef;
        border-radius: 9px;
        background: #fafbfc;
    }
    .customAppItem {
        display: flex;
        align-items: center;
        gap: 11px;
        padding: 10px 12px;
        border-bottom: 1px solid #e9edf2;
        background: #fff;
        transition: background-color 0.15s ease;
    }
    .customAppItem:hover {
        background: #f8fbff;
    }
    .customAppItem:last-child {
        border-bottom: 0;
    }
    .customAppIcon,
    .customAppIconFallback {
        box-sizing: border-box;
        width: 38px;
        height: 38px;
        flex: none;
        border-radius: 9px;
    }
    .customAppIcon {
        padding: 3px;
        border: 1px solid #edf0f4;
        object-fit: contain;
        background: #fff;
    }
    .customAppIconFallback {
        display: flex;
        align-items: center;
        justify-content: center;
        background: linear-gradient(135deg, #e8f1ff, #dce9ff);
        color: #3b6edc;
        font-size: 15px;
        font-weight: 700;
    }
    .customAppMeta {
        min-width: 0;
        flex: 1;
    }
    .customAppName {
        overflow: hidden;
        color: #273142;
        font-size: 13px;
        font-weight: 600;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    .customAppPath {
        margin-top: 3px;
        overflow: hidden;
        color: #8a94a3;
        font-family: Consolas, "SFMono-Regular", monospace;
        font-size: 11px;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    .customAppEmpty {
        display: flex;
        min-height: 112px;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        color: #a0a8b4;
        font-size: 12px;
        text-align: center;
    }
    .customAppEmptyTitle {
        margin-bottom: 4px;
        color: #7d8795;
        font-size: 13px;
        font-weight: 600;
    }
    @media (max-width: 680px) {
        .customAppForm {
            grid-template-columns: 1fr;
        }
        .customAppForm .ant-btn {
            width: 100%;
        }
    }
    .settingError {
        margin: 0 15px 8px;
        color: #d4380d;
    }
    .settingNotice {
        color: #389e0d;
        font-size: 13px;
    }
    .snippetPane {
        display: grid;
        gap: 12px;
        margin: 4px 15px 16px;
    }
    .snippetCard {
        padding: 14px;
        border: 1px solid #e8e8e8;
        border-radius: 10px;
        background: #fff;
        box-shadow: 0 1px 2px rgba(0, 0, 0, 0.03);
    }
    .snippetStatusRow {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 20px;
    }
    .snippetHeading {
        margin: 0;
        color: #1f1f1f;
        font-size: 15px;
        font-weight: 600;
    }
    .snippetHint {
        margin-top: 4px;
        color: #8c8c8c;
        font-size: 12px;
        line-height: 1.5;
    }
    .snippetStatusActions {
        display: flex;
        align-items: center;
        gap: 18px;
        flex: none;
    }
    .snippetSwitch {
        display: flex;
        align-items: center;
        gap: 8px;
        color: #595959;
        font-size: 13px;
        white-space: nowrap;
    }
    .snippetTriggerControl {
        display: flex;
        align-items: center;
        gap: 8px;
        color: #595959;
        font-size: 13px;
        white-space: nowrap;
    }
    .snippetTriggerInput {
        width: 46px;
        text-align: center;
        font-weight: 600;
    }
    .snippetForm {
        display: grid;
        grid-template-columns: minmax(130px, 0.38fr) minmax(220px, 1fr) auto;
        gap: 10px;
        align-items: end;
        margin-top: 12px;
    }
    .snippetField {
        display: grid;
        gap: 6px;
        min-width: 0;
    }
    .snippetLabel {
        color: #595959;
        font-size: 12px;
        font-weight: 500;
    }
    .snippetAddButton {
        min-width: 72px;
    }
    .snippetListHeader {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 8px;
    }
    .snippetCount {
        color: #8c8c8c;
        font-size: 12px;
    }
    .snippetList {
        overflow: hidden;
        border: 1px solid #f0f0f0;
        border-radius: 8px;
    }
    .snippetRow {
        display: grid;
        grid-template-columns: minmax(110px, 0.32fr) minmax(0, 1fr) auto;
        gap: 14px;
        align-items: center;
        min-height: 48px;
        padding: 9px 10px;
        border-bottom: 1px solid #f0f0f0;
    }
    .snippetRow:last-child {
        border-bottom: 0;
    }
    .snippetKeyword {
        overflow: hidden;
        padding: 4px 8px;
        border-radius: 6px;
        background: #f0f5ff;
        color: #2458a6;
        font-family: Consolas, monospace;
        font-size: 13px;
        font-weight: 600;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    .snippetPreview {
        display: -webkit-box;
        overflow: hidden;
        color: #595959;
        font-size: 13px;
        line-height: 1.45;
        overflow-wrap: anywhere;
        white-space: pre-wrap;
        -webkit-box-orient: vertical;
        -webkit-line-clamp: 2;
    }
    .snippetEmpty {
        padding: 26px 16px;
        color: #8c8c8c;
        text-align: center;
    }
    .snippetError {
        padding: 9px 12px;
        border: 1px solid #ffccc7;
        border-radius: 8px;
        background: #fff2f0;
        color: #cf1322;
        font-size: 13px;
    }
    .snippetFooter {
        display: flex;
        justify-content: flex-end;
    }
    @media (max-width: 640px) {
        .hotkeys-item {
            grid-template-columns: 1fr;
            gap: 6px;
        }
        .clipboardRetentionGrid {
            grid-template-columns: repeat(2, minmax(0, 1fr));
        }
        .snippetStatusRow,
        .snippetStatusActions {
            align-items: flex-start;
            flex-direction: column;
        }
        .snippetStatusActions {
            gap: 10px;
        }
        .snippetForm {
            grid-template-columns: 1fr;
        }
        .snippetAddButton {
            width: 100%;
        }
        .snippetRow {
            grid-template-columns: minmax(90px, 0.4fr) minmax(0, 1fr) auto;
            gap: 8px;
        }
    }
}
`

function hotkeyToDownKey(hotkey) {
    const downKey = {alt: false, meta: false, ctrl: false, shift: false, key: ""};
    for (const part of String(hotkey || '').split('+').map((value) => value.trim()).filter(Boolean)) {
        const normalized = part.toLowerCase();
        if (normalized === 'control' || normalized === 'ctrl') downKey.ctrl = true;
        else if (normalized === 'alt' || normalized === 'option') downKey.alt = true;
        else if (normalized === 'shift') downKey.shift = true;
        else if (normalized === 'meta' || normalized === 'super' || normalized === 'command') downKey.meta = true;
        else downKey.key = normalized === 'space' ? 'Space' : part;
    }
    return downKey;
}

function hotkeyDisplayParts(downKey = {}) {
    const parts = [];
    if (downKey.ctrl) parts.push({label: 'Ctrl', title: 'Control'});
    if (downKey.alt) parts.push({label: 'Alt', title: 'Alt'});
    if (downKey.shift) parts.push({label: '⇧', title: 'Shift'});
    if (downKey.meta) parts.push({label: '⊞', title: 'Windows'});
    if (downKey.key) {
        const namedKeys = {Space: 'Space', Enter: '⏎', Return: '⏎'};
        const label = namedKeys[downKey.key] || String(downKey.key).toUpperCase();
        parts.push({label, title: downKey.key});
    }
    return parts;
}

function HotkeyKeys({downKey}) {
    const parts = hotkeyDisplayParts(downKey);
    if (parts.length === 0) return <span className="hotkeyPlaceholder">点击后按下快捷键</span>;
    return <span className="hotkeyKeys">
        {parts.map((part, index) => <React.Fragment key={`${part.title}-${index}`}>
            {index > 0 && <span className="hotkeySeparator">+</span>}
            <span className="hotkeyCap" title={part.title}>{part.label}</span>
        </React.Fragment>)}
    </span>;
}

const modifierKeyMap = {
    Control: "ctrl",
    Alt: "alt",
    Shift: "shift",
    Meta: "meta",
};

const Component = () => {
    const [activeTab, setActiveTab] = useState('app');
    const [clipboardCount, setClipboardCount] = useState(100);
    const [clipboardText, setClipboardText] = useState(10);
    const [clipboardImage, setClipboardImage] = useState(5);
    const [clipboardFile, setClipboardFile] = useState(1);
    const [clipboardCountSwitch, setClipboardCountSwitch] = useState(true);
    const [clipboardTextSwitch, setClipboardTextSwitch] = useState(false);
    const [clipboardImageSwitch, setClipboardImageSwitch] = useState(false);
    const [clipboardFileSwitch, setClipboardFileSwitch] = useState(false);
    const [hotkeyAwaken, setHotkeyAwaken] = useState("Alt+Space");
    const [hotkeyClipboard, setHotkeyClipboard] = useState("Shift+Alt+V");
    const [hotkeyFileJump, setHotkeyFileJump] = useState("Ctrl+G");
    const [appSearchPaths, setAppSearchPaths] = useState([]);
    const [appExcludePaths, setAppExcludePaths] = useState([]);
    const [fileSearchPaths, setFileSearchPaths] = useState(null);
    const [excludePaths, setExcludePaths] = useState([]);
    const [excludeTypes, setExcludeTypes] = useState([]);
    const [newAppSearchPath, setNewAppSearchPath] = useState('');
    const [newAppExcludePath, setNewAppExcludePath] = useState('');
    const [newFileSearchPath, setNewFileSearchPath] = useState('');
    const [newExcludePath, setNewExcludePath] = useState('');
    const [newExcludeType, setNewExcludeType] = useState('');
    const [customApps, setCustomApps] = useState([]);
    const [customAppName, setCustomAppName] = useState('');
    const [customAppPath, setCustomAppPath] = useState('');
    const [customAppError, setCustomAppError] = useState('');
    const [snippetEnabled, setSnippetEnabled] = useState(false);
    const [snippetTrigger, setSnippetTrigger] = useState(';');
    const [snippets, setSnippets] = useState([]);
    const [newSnippetKeyword, setNewSnippetKeyword] = useState('');
    const [newSnippetText, setNewSnippetText] = useState('');
    const [snippetError, setSnippetError] = useState('');
    const [settingNotice, setSettingNotice] = useState({type: '', text: ''});
    const [pythonInterpreter, setPythonInterpreter] = useState(null);
    const [pythonProbe, setPythonProbe] = useState(null);
    const [pythonProbeError, setPythonProbeError] = useState('');
    const [autoLaunch, setAutoLaunch] = useState(false);
    const hotkeyCaptureActive = useRef(false);
    const activeHotkeyField = useRef(null);
    const hotkeyCaptureTransition = useRef(Promise.resolve());
    const hotkeyAwakenRef = useRef(hotkeyAwaken);
    const hotkeyClipboardRef = useRef(hotkeyClipboard);
    const hotkeyFileJumpRef = useRef(hotkeyFileJump);

    hotkeyAwakenRef.current = hotkeyAwaken;
    hotkeyClipboardRef.current = hotkeyClipboard;
    hotkeyFileJumpRef.current = hotkeyFileJump;

    useEffect(() => {
        invoke("get_app_settings").then((settings) => {
            setHotkeyAwaken(settings.hotkeyAwaken);
            setHotkeyClipboard(settings.hotkeyClipboard);
            setHotkeyFileJump(settings.hotkeyFileJump || "Ctrl+G");
            setClipboardCountSwitch(settings.clipboardCountSwitch ?? true);
            setClipboardCount(settings.clipboardCount ?? 100);
            setClipboardTextSwitch(settings.clipboardTextSwitch ?? false);
            setClipboardText(settings.clipboardText ?? 10);
            setClipboardImageSwitch(settings.clipboardImageSwitch ?? false);
            setClipboardImage(settings.clipboardImage ?? 5);
            setClipboardFileSwitch(settings.clipboardFileSwitch ?? false);
            setClipboardFile(settings.clipboardFile ?? 1);
            const interpreter = settings.pythonInterpreter ?? null;
            setPythonInterpreter(interpreter);
            probePythonInterpreter(interpreter);
        }).catch((error) => console.error("读取应用设置失败", error));
        invoke('get_auto_launch_enabled').then(setAutoLaunch).catch((error) => console.error('读取开机启动状态失败', error));
        invoke("get_index_settings").then((settings) => {
            setAppSearchPaths(settings.localAppSearchPaths || []);
            setAppExcludePaths(settings.localAppSearchExcludePaths || []);
            setFileSearchPaths(settings.localFileSearchPaths ?? null);
            setExcludePaths(settings.localFileSearchExcludePaths || []);
            setExcludeTypes(settings.localFileSearchExcludeTypes || []);
        }).catch((error) => console.error("读取索引设置失败", error));
        invoke("get_snippet_settings").then((settings) => {
            setSnippetEnabled(settings.enabled ?? false);
            setSnippetTrigger(settings.trigger || ';');
            setSnippets(settings.snippets || []);
        }).catch((error) => setSnippetError(String(error)));
        loadCustomApps();
    }, []);

    useEffect(() => () => {
        if (hotkeyCaptureActive.current) {
            invoke('set_hotkey_capture_active', {active: false}).catch(console.error);
        }
    }, []);

    useEffect(() => {
        let disposed = false;
        let unlisten;
        listen('hotkey-capture', async ({payload}) => {
            const name = activeHotkeyField.current;
            if (!name || typeof payload !== 'string') return;
            const nextAwaken = name === 'lark' ? payload : hotkeyAwakenRef.current;
            const nextClipboard = name === 'cbd' ? payload : hotkeyClipboardRef.current;
            const nextFileJump = name === 'fileJump' ? payload : hotkeyFileJumpRef.current;
            try {
                await invoke('reserve_hotkey_capture', {
                    awaken: nextAwaken,
                    clipboard: nextClipboard,
                    fileJump: nextFileJump,
                });
                if (name === 'lark') {
                    hotkeyAwakenRef.current = payload;
                    setHotkeyAwaken(payload);
                    setLarkDisplayText({behavior: 'set', data: hotkeyToDownKey(payload)});
                } else if (name === 'cbd') {
                    hotkeyClipboardRef.current = payload;
                    setHotkeyClipboard(payload);
                    setCBDDisplayText({behavior: 'set', data: hotkeyToDownKey(payload)});
                } else {
                    hotkeyFileJumpRef.current = payload;
                    setHotkeyFileJump(payload);
                    setFileJumpDisplayText({behavior: 'set', data: hotkeyToDownKey(payload)});
                }
                setSettingNotice({type: '', text: ''});
            } catch (error) {
                setSettingNotice({type: 'error', text: `快捷键暂时无法占用：${error}`});
            }
        }).then((stop) => {
            if (disposed) stop();
            else unlisten = stop;
        });
        return () => {
            disposed = true;
            unlisten?.();
        };
    }, []);

    async function loadCustomApps() {
        try {
            setCustomApps(await invoke("get_custom_app_indexes"));
            setCustomAppError('');
        } catch (error) {
            setCustomAppError(`读取自定义应用失败：${error}`);
        }
    }

    const [larkDisplayText, setLarkDisplayText] = useReducer(hotkeysFrameShow, {downKey:{
            alt: true,
            meta: false,
            ctrl: false,
            shift: false,
            key: "Space"
        }
    });
    const [cbdDisplayText, setCBDDisplayText] = useReducer(hotkeysFrameShow, {downKey: {}});
    const [fileJumpDisplayText, setFileJumpDisplayText] = useReducer(hotkeysFrameShow, {downKey: {}});

    useEffect(() => {
        setLarkDisplayText({behavior: "set", data: hotkeyToDownKey(hotkeyAwaken)});
        setCBDDisplayText({behavior: "set", data: hotkeyToDownKey(hotkeyClipboard)});
        setFileJumpDisplayText({behavior: "set", data: hotkeyToDownKey(hotkeyFileJump)});
    }, [hotkeyAwaken, hotkeyClipboard, hotkeyFileJump]);


    // 解释器探测：只在打开设置页、选择文件或输入框失焦时触发，结果仅用于提示，不阻断保存。
    function probePythonInterpreter(path) {
        const value = typeof path === 'string' && path.trim() ? path.trim() : null;
        invoke("probe_python_interpreter", {path: value}).then((result) => {
            setPythonProbe(result);
            setPythonProbeError('');
        }).catch((error) => {
            setPythonProbe(null);
            setPythonProbeError(String(error));
        });
    }

    async function choosePythonInterpreter() {
        try {
            // macOS/Linux 的解释器通常没有扩展名，加 filters 会导致选不中。
            const windows = (navigator.userAgent || '').includes('Windows');
            const selected = await open({
                multiple: false,
                directory: false,
                filters: windows ? [{name: 'Python 解释器', extensions: ['exe']}] : undefined,
            });
            if (typeof selected !== 'string') return;
            setPythonInterpreter(selected);
            probePythonInterpreter(selected);
        } catch (error) {
            setPythonProbeError(String(error));
        }
    }

    function pythonProbeSummary() {
        if (pythonProbeError) return `检测失败：${pythonProbeError}`;
        if (!pythonProbe) return '未检测';
        if (!pythonProbe.ok) {
            const hint = pythonProbe.kind === 'store-alias'
                ? '；这看起来是 Microsoft Store 的应用执行别名，请改选具体解释器（例如 <虚拟环境>\\Scripts\\python.exe）'
                : '';
            return `不可用：${pythonProbe.error || '未知错误'}${hint}`;
        }
        const label = pythonProbe.kind === 'venv' ? '虚拟环境' : '系统 Python';
        const version = pythonProbe.version ? ` ${pythonProbe.version}` : '';
        return pythonProbe.configured
            ? `${label}${version}（已配置）`
            : `未配置，使用系统默认：${pythonProbe.resolved}${version}`;
    }

    const handleSettingReset = () => {
        const defaults = {
            clipboardCountSwitch: true,
            clipboardCount: 100,
            clipboardTextSwitch: false,
            clipboardText: 10,
            clipboardImageSwitch: false,
            clipboardImage: 5,
            clipboardFileSwitch: false,
            clipboardFile: 1
        };
        setClipboardCountSwitch(defaults.clipboardCountSwitch);
        setClipboardCount(100);
        setClipboardTextSwitch(defaults.clipboardTextSwitch);
        setClipboardText(10);
        setClipboardImageSwitch(defaults.clipboardImageSwitch);
        setClipboardImage(5);
        setClipboardFileSwitch(defaults.clipboardFileSwitch);
        setClipboardFile(1);
        setSettingNotice({type: '', text: ''});
    }
    const handleSettingSave = async () => {
        let all_setting = {
            hotkeyAwaken,
            hotkeyClipboard,
            hotkeyFileJump,
            clipboardCountSwitch,
            clipboardCount,
            clipboardTextSwitch,
            clipboardText,
            clipboardImageSwitch,
            clipboardImage,
            clipboardFileSwitch,
            clipboardFile,
            // 清空必须显式传 null：undefined 会被 JSON.stringify 丢掉，宿主会当成「不更新」。
            pythonInterpreter: pythonInterpreter ?? null
        }
        try {
            await hotkeyCaptureTransition.current;
            await invoke('set_hotkey_capture_active', {active: false});
            hotkeyCaptureActive.current = false;
            await invoke("save_setting", {settingInfo: all_setting});
            setSettingNotice({type: 'success', text: '设置已保存'});
        } catch (error) {
            setSettingNotice({type: 'error', text: String(error)});
        }
    }

    const handleAutoLaunchChange = async (checked) => {
        const previous = autoLaunch;
        setAutoLaunch(checked);
        try {
            await invoke('set_auto_launch_enabled', {enabled: checked});
            setSettingNotice({type: 'success', text: checked ? '已开启开机启动' : '已关闭开机启动'});
        } catch (error) {
            setAutoLaunch(previous);
            setSettingNotice({type: 'error', text: `开机启动设置失败：${error}`});
        }
    };

    const handleHotkeyCapture = async (active) => {
        hotkeyCaptureActive.current = active;
        hotkeyCaptureTransition.current = hotkeyCaptureTransition.current
            .catch(() => {})
            .then(() => invoke('set_hotkey_capture_active', {active}));
        try {
            await hotkeyCaptureTransition.current;
        } catch (error) {
            hotkeyCaptureActive.current = false;
            setSettingNotice({type: 'error', text: `快捷键录制状态切换失败：${error}`});
            throw error;
        }
    }

    const handleIndexSettingSave = async () => {
        await invoke("save_index_settings", {settingInfo: {
            localAppSearchPaths: appSearchPaths,
            localAppSearchExcludePaths: appExcludePaths,
            localFileSearchPaths: fileSearchPaths,
            localFileSearchExcludePaths: excludePaths,
            localFileSearchExcludeTypes: excludeTypes
        }})
    }

    const addSnippet = () => {
        const keyword = newSnippetKeyword.trim();
        if (!/^[A-Za-z0-9_-]{1,32}$/.test(keyword)) {
            setSnippetError('关键词只能包含字母、数字、下划线或短横线，长度 1-32');
            return;
        }
        if (!newSnippetText) {
            setSnippetError('片段内容不能为空');
            return;
        }
        if (snippets.some((item) => item.keyword === keyword)) {
            setSnippetError('关键词不能重复');
            return;
        }
        if (snippets.some((item) => item.keyword.startsWith(keyword) || keyword.startsWith(item.keyword))) {
            setSnippetError('关键词不能互为前缀，否则无法判断何时展开');
            return;
        }
        setSnippets([...snippets, {keyword, text: newSnippetText}]);
        setNewSnippetKeyword('');
        setNewSnippetText('');
        setSnippetError('');
    };

    const saveSnippets = async () => {
        if ([...snippetTrigger].length !== 1 || /\s/.test(snippetTrigger)) {
            setSnippetError('触发符必须是一个非空白字符');
            return;
        }
        try {
            await invoke('save_snippet_settings', {settingInfo: {
                enabled: snippetEnabled,
                trigger: snippetTrigger,
                snippets,
            }});
            setSnippetError('');
        } catch (error) {
            setSnippetError(String(error));
        }
    };

    const addAppSearchPath = () => {
        const value = newAppSearchPath.trim();
        if (value && !appSearchPaths.includes(value)) setAppSearchPaths([...appSearchPaths, value]);
        setNewAppSearchPath('');
    }

    const addAppExcludePath = () => {
        const value = newAppExcludePath.trim();
        if (value && !appExcludePaths.includes(value)) setAppExcludePaths([...appExcludePaths, value]);
        setNewAppExcludePath('');
    }

    const addFileSearchPath = () => {
        const value = newFileSearchPath.trim();
        const paths = fileSearchPaths || [];
        if (value && !paths.includes(value)) setFileSearchPaths([...paths, value]);
        setNewFileSearchPath('');
    }

    const addExcludePath = () => {
        const value = newExcludePath.trim();
        if (value && !excludePaths.includes(value)) setExcludePaths([...excludePaths, value]);
        setNewExcludePath('');
    }

    const addExcludeType = () => {
        const value = newExcludeType.trim().replace(/^\./, '').toLowerCase();
        if (value && !excludeTypes.includes(value)) setExcludeTypes([...excludeTypes, value]);
        setNewExcludeType('');
    }

    const addCustomApp = async () => {
        const appName = customAppName.trim();
        const appPath = customAppPath.trim();
        if (!appName || !appPath) {
            setCustomAppError('请输入应用名称和 .exe 路径');
            return;
        }
        try {
            await invoke("add_custom_app_index", {appName, appPath});
            setCustomAppName('');
            setCustomAppPath('');
            await loadCustomApps();
        } catch (error) {
            setCustomAppError(String(error));
        }
    }

    const deleteCustomApp = async (id) => {
        try {
            await invoke("delete_custom_app_index", {id});
            await loadCustomApps();
        } catch (error) {
            setCustomAppError(String(error));
        }
    }


    const handleHotkeysDown = async (event, name) => {
        event.preventDefault();  // 防止默认行为
        try {
            await hotkeyCaptureTransition.current;
        } catch {
            return;
        }
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
        } else if (name === "fileJump") {
            setFileJumpDisplayText({behavior: "down", data: downKey})
        }
        const modifierOnly = ['Control', 'Alt', 'Shift', 'Meta'].includes(event.key);
        if (!modifierOnly) {
            const parts = [];
            if (event.ctrlKey) parts.push("Control");
            if (event.altKey) parts.push("Alt");
            if (event.shiftKey) parts.push("Shift");
            if (event.metaKey) parts.push("Super");
            parts.push(event.code === "Space" ? "Space" : event.key.length === 1 ? event.key.toUpperCase() : event.key);
            const candidate = parts.join("+");
            const nextAwaken = name === 'lark' ? candidate : hotkeyAwaken;
            const nextClipboard = name === 'cbd' ? candidate : hotkeyClipboard;
            const nextFileJump = name === 'fileJump' ? candidate : hotkeyFileJump;
            try {
                await invoke('reserve_hotkey_capture', {
                    awaken: nextAwaken,
                    clipboard: nextClipboard,
                    fileJump: nextFileJump,
                });
                if (name === "lark") {
                    hotkeyAwakenRef.current = candidate;
                    setHotkeyAwaken(candidate);
                } else if (name === "cbd") {
                    hotkeyClipboardRef.current = candidate;
                    setHotkeyClipboard(candidate);
                } else {
                    hotkeyFileJumpRef.current = candidate;
                    setHotkeyFileJump(candidate);
                }
                setSettingNotice({type: '', text: ''});
            } catch (error) {
                if (name === "lark") {
                    setLarkDisplayText({behavior: 'set', data: hotkeyToDownKey(hotkeyAwaken)});
                } else if (name === "cbd") {
                    setCBDDisplayText({behavior: 'set', data: hotkeyToDownKey(hotkeyClipboard)});
                } else {
                    setFileJumpDisplayText({behavior: 'set', data: hotkeyToDownKey(hotkeyFileJump)});
                }
                setSettingNotice({type: 'error', text: `快捷键暂时无法占用：${error}`});
            }
        }
    }

    const handleHotkeysUp = (event, name) => {
        let upKey = {
            key: modifierKeyMap[event.key] || event.key,
        }
        if (event.code === "Space") {
            upKey.key = "Space"
        }
        if (name === "lark") {
            setLarkDisplayText({behavior: "up", data: upKey})
        } else if (name === "cbd") {
            setCBDDisplayText({behavior: "up", data: upKey})
        } else if (name === "fileJump") {
            setFileJumpDisplayText({behavior: "up", data: upKey})
        }
    }

    function hotkeysFrameShow(stats, action) {
        let downKey = {}
        if (action.behavior === "set" || action.behavior === "down") {
            downKey = {...action.data}

        } else if (action.behavior === "up") {
            downKey = {...stats.downKey}
            if (!downKey.key || !(downKey.ctrl || downKey.alt || downKey.shift || downKey.meta)) {
                if (Object.hasOwn(modifierKeyMap, action.data.key)) {
                    downKey[action.data.key] = false
                }
                if (downKey.key === action.data.key) {
                    downKey.key = ""
                }
            } else {
                // todo 处理组合键
            }

        }
        return {
            downKey: downKey
        };
    }

    return (
        <>
            <Wrapper/>
            <div id="settingframe">
                <Tabs activeKey={activeTab} onChange={setActiveTab} centered items={[
                    {key: 'app', label: '应用设置'},
                    {key: 'index', label: '索引扫描'},
                    {key: 'custom', label: '手动应用'},
                    {key: 'snippets', label: '文本片段'}
                ]}/>
                {activeTab === 'snippets' && <div className="snippetPane">
                    <section className="snippetCard snippetStatusRow">
                        <div>
                            <h3 className="snippetHeading">自动展开</h3>
                            <div className="snippetHint">输入触发符和关键词后，自动替换为保存的文本。</div>
                        </div>
                        <div className="snippetStatusActions">
                            <label className="snippetSwitch">
                                <Switch size="small" checked={snippetEnabled} onChange={setSnippetEnabled}/>
                                <span>{snippetEnabled ? '已启用' : '已停用'}</span>
                            </label>
                            <label className="snippetTriggerControl">
                                <span>触发符</span>
                                <Input className="snippetTriggerInput" value={snippetTrigger} maxLength={1} size="small"
                                       aria-label="文本片段触发符"
                                       onChange={(event) => setSnippetTrigger(event.target.value.slice(-1))}/>
                            </label>
                        </div>
                    </section>

                    <section className="snippetCard">
                        <h3 className="snippetHeading">添加片段</h3>
                        <div className="snippetHint">关键词不需要包含触发符，例如填写 email，使用时输入 {snippetTrigger || ';'}email。</div>
                        <div className="snippetForm">
                            <label className="snippetField">
                                <span className="snippetLabel">关键词</span>
                                <Input value={newSnippetKeyword} placeholder="例如 email"
                                       prefix={<span style={{color: '#8c8c8c'}}>{snippetTrigger || ';'}</span>}
                                       onChange={(event) => setNewSnippetKeyword(event.target.value)}/>
                            </label>
                            <label className="snippetField">
                                <span className="snippetLabel">替换内容</span>
                                <Input.TextArea value={newSnippetText} placeholder="输入要自动写入的文本"
                                                autoSize={{minRows: 1, maxRows: 4}}
                                                onChange={(event) => setNewSnippetText(event.target.value)}/>
                            </label>
                            <Button className="snippetAddButton" type="primary" onClick={addSnippet}>添加</Button>
                        </div>
                    </section>

                    {snippetError && <div className="snippetError">{snippetError}</div>}

                    <section className="snippetCard">
                        <div className="snippetListHeader">
                            <h3 className="snippetHeading">已有片段</h3>
                            <span className="snippetCount">{snippets.length} 个</span>
                        </div>
                        <div className="snippetList">
                            {snippets.length === 0 && <div className="snippetEmpty">还没有文本片段，请先添加一个。</div>}
                            {snippets.map((item) => <div className="snippetRow" key={item.keyword}>
                                <div className="snippetKeyword" title={`${snippetTrigger}${item.keyword}`}>
                                    {snippetTrigger}{item.keyword}
                                </div>
                                <div className="snippetPreview" title={item.text}>{item.text}</div>
                                <Popconfirm title="删除这个文本片段？"
                                            onConfirm={() => setSnippets(snippets.filter((snippet) => snippet.keyword !== item.keyword))}
                                            okText="删除" cancelText="取消">
                                    <Button type="text" danger size="small">删除</Button>
                                </Popconfirm>
                            </div>)}
                        </div>
                    </section>

                    <div className="snippetFooter">
                        <Button type="primary" onClick={saveSnippets}>保存设置</Button>
                    </div>
                </div>}
                {activeTab === 'index' && <div className="indexSettingsPane">
                    <header className="indexSettingsIntro">
                        <h2 className="indexSettingsTitle">索引扫描</h2>
                        <p className="indexSettingsDescription">设置应用的扫描范围，并过滤不需要进入搜索结果的目录和文件类型。</p>
                    </header>

                    <div className="indexSettingsGrid">
                        <section className="indexSettingCard">
                            <div className="indexSettingHeader">
                                <div>
                                    <h3 className="indexSettingHeading">应用扫描路径</h3>
                                    <div className="indexSettingHint">添加需要发现应用程序的目录</div>
                                </div>
                                <span className="indexSettingCount">{appSearchPaths.length} 项</span>
                            </div>
                            <div className="indexSettingList">
                                {appSearchPaths.length === 0 && <div className="indexSettingEmpty">暂无扫描路径</div>}
                                {appSearchPaths.map((path) => <Tag title={path} key={path} closable onClose={() => setAppSearchPaths(appSearchPaths.filter((item) => item !== path))}>{path}</Tag>)}
                            </div>
                            <div className="indexSettingAdd">
                                <Input value={newAppSearchPath} placeholder="例如 D:\\Apps" onChange={(event) => setNewAppSearchPath(event.target.value)} onPressEnter={addAppSearchPath}/>
                                <Button type="primary" ghost onClick={addAppSearchPath}>添加</Button>
                            </div>
                        </section>

                        <section className="indexSettingCard">
                            <div className="indexSettingHeader">
                                <div>
                                    <h3 className="indexSettingHeading">应用排除目录</h3>
                                    <div className="indexSettingHint">忽略扫描路径中的指定子目录</div>
                                </div>
                                <span className="indexSettingCount">{appExcludePaths.length} 项</span>
                            </div>
                            <div className="indexSettingList">
                                {appExcludePaths.length === 0 && <div className="indexSettingEmpty">暂无排除目录</div>}
                                {appExcludePaths.map((path) => <Tag title={path} key={path} closable onClose={() => setAppExcludePaths(appExcludePaths.filter((item) => item !== path))}>{path}</Tag>)}
                            </div>
                            <div className="indexSettingAdd">
                                <Input value={newAppExcludePath} placeholder="例如 D:\\Apps\\不需要扫描的目录" onChange={(event) => setNewAppExcludePath(event.target.value)} onPressEnter={addAppExcludePath}/>
                                <Button type="primary" ghost onClick={addAppExcludePath}>添加</Button>
                            </div>
                        </section>

                        <section className="indexSettingCard">
                            <div className="indexSettingHeader">
                                <div>
                                    <h3 className="indexSettingHeading">文件包含路径</h3>
                                    <div className="indexSettingHint">仅扫描和监听这些目录；包含与排除冲突时以排除为准</div>
                                </div>
                                <span className="indexSettingCount">{fileSearchPaths === null ? '未初始化' : `${fileSearchPaths.length} 项`}</span>
                            </div>
                            <div className="indexSettingList">
                                {fileSearchPaths === null && <div className="indexSettingEmpty">尚未初始化，应用启动时将生成默认包含路径</div>}
                                {fileSearchPaths?.length === 0 && <div className="indexSettingEmpty">已明确设置为空，不扫描任何目录</div>}
                                {(fileSearchPaths || []).map((path) => <Tag title={path} key={path} closable onClose={() => setFileSearchPaths(fileSearchPaths.filter((item) => item !== path))}>{path}</Tag>)}
                            </div>
                            <div className="indexSettingAdd">
                                <Input value={newFileSearchPath} placeholder="例如 C:\\Users\\admin\\Downloads 或 D:\\" onChange={(event) => setNewFileSearchPath(event.target.value)} onPressEnter={addFileSearchPath}/>
                                <Button type="primary" ghost onClick={addFileSearchPath}>添加</Button>
                            </div>
                        </section>

                        <section className="indexSettingCard">
                            <div className="indexSettingHeader">
                                <div>
                                    <h3 className="indexSettingHeading">文件排除路径</h3>
                                    <div className="indexSettingHint">支持目录路径或通配规则</div>
                                </div>
                                <span className="indexSettingCount">{excludePaths.length} 项</span>
                            </div>
                            <div className="indexSettingList">
                                {excludePaths.length === 0 && <div className="indexSettingEmpty">暂无排除路径</div>}
                                {excludePaths.map((path) => <Tag title={path} key={path} closable onClose={() => setExcludePaths(excludePaths.filter((item) => item !== path))}>{path}</Tag>)}
                            </div>
                            <div className="indexSettingAdd">
                                <Input value={newExcludePath} placeholder="例如 */node_modules 或 C:\\Windows" onChange={(event) => setNewExcludePath(event.target.value)} onPressEnter={addExcludePath}/>
                                <Button type="primary" ghost onClick={addExcludePath}>添加</Button>
                            </div>
                        </section>

                        <section className="indexSettingCard">
                            <div className="indexSettingHeader">
                                <div>
                                    <h3 className="indexSettingHeading">排除文件类型</h3>
                                    <div className="indexSettingHint">填写扩展名时无需输入点号</div>
                                </div>
                                <span className="indexSettingCount">{excludeTypes.length} 项</span>
                            </div>
                            <div className="indexSettingList">
                                {excludeTypes.length === 0 && <div className="indexSettingEmpty">暂无排除类型</div>}
                                {excludeTypes.map((type) => <Tag title={type} key={type} closable onClose={() => setExcludeTypes(excludeTypes.filter((item) => item !== type))}>{type}</Tag>)}
                            </div>
                            <div className="indexSettingAdd">
                                <Input value={newExcludeType} placeholder="例如 tmp" onChange={(event) => setNewExcludeType(event.target.value)} onPressEnter={addExcludeType}/>
                                <Button type="primary" ghost onClick={addExcludeType}>添加</Button>
                            </div>
                        </section>
                    </div>

                    <div className="indexSettingsFooter">
                        <span className="indexSettingsFooterHint">修改后需保存才会应用到后续索引扫描。</span>
                        <Button type="primary" onClick={handleIndexSettingSave}>保存设置</Button>
                    </div>
                </div>}
                {activeTab === 'custom' && <div className="customAppsPane">
                    <header className="customAppsIntro">
                        <h2 className="customAppsTitle">手动应用</h2>
                        <p className="customAppsDescription">将未被自动扫描发现的程序手动加入搜索结果，添加后即可通过应用名称快速启动。</p>
                    </header>

                    <section className="customAppCard">
                        <div className="customAppFormHeader">
                            <div>
                                <h3 className="customAppHeading">添加应用</h3>
                                <div className="customAppHint">填写显示名称和可执行文件的完整路径</div>
                            </div>
                        </div>
                        <div className="customAppForm">
                            <Input value={customAppName} placeholder="应用名称" aria-label="应用名称"
                                   onChange={(event) => setCustomAppName(event.target.value)}/>
                            <Input value={customAppPath} placeholder="例如 D:\\Apps\\MyApp.exe" aria-label="应用路径"
                                   onChange={(event) => setCustomAppPath(event.target.value)} onPressEnter={addCustomApp}/>
                            <Button type="primary" onClick={addCustomApp}>添加应用</Button>
                        </div>
                    </section>

                    {customAppError && <div className="customAppError">{customAppError}</div>}

                    <section className="customAppCard">
                        <div className="customAppListHeader">
                            <div>
                                <h3 className="customAppHeading">已添加应用</h3>
                                <div className="customAppHint">这些应用会直接出现在本地应用搜索结果中</div>
                            </div>
                            <span className="customAppCount">{customApps.length} 个</span>
                        </div>
                        <div className="customAppList">
                            {customApps.length === 0 && <div className="customAppEmpty">
                                <div className="customAppEmptyTitle">还没有手动应用</div>
                                <div>在上方填写应用信息后添加</div>
                            </div>}
                            {customApps.map((app) => <div className="customAppItem" key={app.id}>
                                {app.icon
                                    ? <img className="customAppIcon" src={`data:image/png;base64,${app.icon}`} alt=""/>
                                    : <div className="customAppIconFallback" aria-hidden="true">{app.title?.trim().charAt(0).toUpperCase() || 'A'}</div>}
                                <div className="customAppMeta">
                                    <div className="customAppName" title={app.title}>{app.title}</div>
                                    <div className="customAppPath" title={app.path}>{app.path}</div>
                                </div>
                                <Popconfirm title="删除这个手动应用？" onConfirm={() => deleteCustomApp(app.id)} okText="删除" cancelText="取消">
                                    <Button type="text" danger size="small">删除</Button>
                                </Popconfirm>
                            </div>)}
                        </div>
                    </section>
                </div>}
                {activeTab === 'app' && <div className="appSettingsPane">
                    <section className="appSettingCard">
                        <div className="snippetStatusRow">
                            <div>
                                <h3 className="snippetHeading">开机启动</h3>
                                <div className="snippetHint">登录 Windows 后自动启动百灵鸟。</div>
                            </div>
                            <Switch checked={autoLaunch} onChange={handleAutoLaunchChange} />
                        </div>
                    </section>
                    <section className="appSettingCard">
                        <h3 className="snippetHeading">快捷键</h3>
                        <div className="snippetHint">点击快捷键框，然后按下新的组合键。</div>
                        <div className="hotkeysFrame"
                             onFocusCapture={() => handleHotkeyCapture(true).catch(() => {})}
                             onBlurCapture={(event) => {
                                 if (!event.currentTarget.contains(event.relatedTarget)) {
                                     activeHotkeyField.current = null;
                                     handleHotkeyCapture(false).catch(() => {});
                                 }
                             }}>
                            <div className="hotkeys-item">
                                <div>
                                    <div className="hotkeyName">打开百灵鸟</div>
                                    <div className="hotkeyDescription">显示或隐藏主搜索窗口</div>
                                </div>
                                <div contentEditable suppressContentEditableWarning className="hotkeys-input"
                                     role="textbox" aria-label="百灵鸟快捷键"
                                     onFocus={() => { activeHotkeyField.current = 'lark'; }}
                                     onKeyDown={(event) => handleHotkeysDown(event, 'lark')}
                                     onKeyUp={(event) => handleHotkeysUp(event, 'lark')}>
                                    <HotkeyKeys downKey={larkDisplayText.downKey}/>
                                </div>
                            </div>
                            <div className="hotkeys-item">
                                <div>
                                    <div className="hotkeyName">打开剪贴板</div>
                                    <div className="hotkeyDescription">快速打开剪贴板历史</div>
                                </div>
                                <div contentEditable suppressContentEditableWarning className="hotkeys-input"
                                     role="textbox" aria-label="剪贴板快捷键"
                                     onFocus={() => { activeHotkeyField.current = 'cbd'; }}
                                     onKeyDown={(event) => handleHotkeysDown(event, 'cbd')}
                                     onKeyUp={(event) => handleHotkeysUp(event, 'cbd')}>
                                    <HotkeyKeys downKey={cbdDisplayText.downKey}/>
                                </div>
                            </div>
                            <div className="hotkeys-item">
                                <div>
                                    <div className="hotkeyName">文件跳转</div>
                                    <div className="hotkeyDescription">文件选择框自动跳转到当前目录</div>
                                </div>
                                <div contentEditable suppressContentEditableWarning className="hotkeys-input"
                                     role="textbox" aria-label="文件跳转快捷键"
                                     onFocus={() => { activeHotkeyField.current = 'fileJump'; }}
                                     onKeyDown={(event) => handleHotkeysDown(event, 'fileJump')}
                                     onKeyUp={(event) => handleHotkeysUp(event, 'fileJump')}>
                                    <HotkeyKeys downKey={fileJumpDisplayText.downKey}/>
                                </div>
                            </div>
                        </div>
                    </section>

                    <section className="appSettingCard">
                        <h3 className="snippetHeading">剪贴板历史</h3>
                        <div className="snippetHint">限制保存数量，或按内容类型设置保留天数。</div>
                        <div className="clipboardRetentionGrid">
                            <div className="settingSmallFrame">
                                <Checkbox checked={clipboardCountSwitch} onChange={(event) => setClipboardCountSwitch(event.target.checked)}>数量（个）</Checkbox>
                                <InputNumber size="small" min={10} max={200} value={clipboardCount}
                                             disabled={!clipboardCountSwitch}
                                             onChange={(value) => setClipboardCount(value ?? 100)} changeOnWheel/>
                            </div>
                            <div className="settingSmallFrame">
                                <Checkbox checked={clipboardTextSwitch} onChange={(event) => setClipboardTextSwitch(event.target.checked)}>文本（天）</Checkbox>
                                <InputNumber size="small" min={1} max={30} value={clipboardText}
                                             disabled={!clipboardTextSwitch}
                                             onChange={(value) => setClipboardText(value ?? 10)} changeOnWheel/>
                            </div>
                            <div className="settingSmallFrame">
                                <Checkbox checked={clipboardImageSwitch} onChange={(event) => setClipboardImageSwitch(event.target.checked)}>图片（天）</Checkbox>
                                <InputNumber size="small" min={1} max={15} value={clipboardImage}
                                             disabled={!clipboardImageSwitch}
                                             onChange={(value) => setClipboardImage(value ?? 5)} changeOnWheel/>
                            </div>
                            <div className="settingSmallFrame">
                                <Checkbox checked={clipboardFileSwitch} onChange={(event) => setClipboardFileSwitch(event.target.checked)}>文件（天）</Checkbox>
                                <InputNumber size="small" min={1} max={10} value={clipboardFile}
                                             disabled={!clipboardFileSwitch}
                                             onChange={(value) => setClipboardFile(value ?? 1)} changeOnWheel/>
                            </div>
                        </div>
                    </section>

                    <section className="appSettingCard">
                        <h3 className="snippetHeading">Python 环境</h3>
                        <div className="snippetHint">
                            插件 Python 与外部 Python 索引脚本共用这个解释器；留空使用系统默认。指向虚拟环境时请选择
                            它下面的 Scripts\python.exe（不需要「激活」环境）。
                        </div>
                        <div style={{display: 'flex', gap: 8, alignItems: 'center'}}>
                            <Input size="small" value={pythonInterpreter || ''} allowClear
                                   placeholder="留空使用系统默认解释器"
                                   onChange={(event) => setPythonInterpreter(event.target.value || null)}
                                   onBlur={() => probePythonInterpreter(pythonInterpreter)}/>
                            <Button size="small" onClick={choosePythonInterpreter}>选择…</Button>
                            <Button size="small" onClick={() => probePythonInterpreter(pythonInterpreter)}>检测</Button>
                        </div>
                        <div className={pythonProbeError ? 'settingError' : 'snippetHint'} style={{marginTop: 8}}>
                            {pythonProbeSummary()}
                        </div>
                    </section>

                    <div className="appSettingFooter">
                        {settingNotice.text && <span className={settingNotice.type === 'error' ? 'settingError' : 'settingNotice'}>
                            {settingNotice.text}
                        </span>}
                        <Button onClick={handleSettingReset}>重置</Button>
                        <Button type="primary" onClick={handleSettingSave}>保存设置</Button>
                    </div>
                </div>}
            </div>
        </>
    );
};

export default Component;
