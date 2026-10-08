import React, {useState, useRef, useEffect, useReducer} from 'react';
import styled, {createGlobalStyle} from 'styled-components';
import {distance} from "mathjs";
import {Button, Input, InputNumber, Checkbox, Flex, Tabs, Tag, Popconfirm, Switch, Select} from 'antd';
import {
    DEFAULT_TRANSLATION_SETTINGS,
    TRANSLATION_PROVIDER_CATALOG,
    OCR_PROVIDER_CATALOG,
    normalizeTranslationSettings,
    readTranslationSettings,
    TRANSLATION_SETTINGS_KEY
} from '../translationSettings';
import {invoke} from "@tauri-apps/api/core";
import {open} from "@tauri-apps/plugin-dialog";
import {listen} from "@tauri-apps/api/event";

const Wrapper = createGlobalStyle`
    a {
        font-size: 20px;
    }

    #settingframe {
        box-sizing: border-box;
        height: 100%;
        padding-bottom: 12px;
        overflow-y: auto;
    }

    .settingInput {
        font-size: 15px;
        height: 35px;
        width: 200px;

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

    .appSettingActionRow {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 16px;
    }

    .appSettingActionText {
        min-width: 0;
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
        position: sticky;
        bottom: 0;
        z-index: 2;
        display: flex;
        align-items: center;
        justify-content: flex-end;
        gap: 8px;
        padding: 10px 12px;
        background: rgba(255, 255, 255, .94);
        border-top: 1px solid #edf0f5;
        border-radius: 8px;
    }

    .indexSettingsPane {
        display: grid;
        gap: 12px;
        margin: 4px 15px 16px;
    }

    .indexCounts {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
        margin-top: 12px;
    }

    .indexCountCard {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 9px 12px;
        border: 1px solid #dce8ff;
        border-radius: 8px;
        background: #fff;
        color: #475569;
        font-size: 12px;
    }

    .indexCountValue {
        color: #2563eb;
        font-size: 15px;
        font-weight: 650;
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

    .indexSettingList::-webkit-scrollbar {
        width: 6px;
    }

    .indexSettingList::-webkit-scrollbar-thumb {
        border-radius: 999px;
        background: #d6deea;
    }

    .indexSettingList::-webkit-scrollbar-track {
        border-radius: 999px;
        background: #f1f4f8;
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
        padding: 10px 12px;
        position: sticky;
        bottom: 0;
        z-index: 2;
        background: rgba(255, 255, 255, .94);
        border-top: 1px solid #edf0f5;
        border-radius: 8px;
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
        font-size: 12px;
        line-height: 1.5;
    }

    .settingNotice {
        color: #389e0d;
        font-size: 13px;
    }

    .settingDirtyMark,
    .settingDirtyHint {
        color: #cf1322;
    }

    .settingDirtyMark {
        margin-left: 4px;
        font-weight: 700;
    }

    .settingDirtyHint {
        margin-right: auto;
        font-size: 12px;
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
        font-size: 13px;
    }

    .snippetError {
        padding: 9px 12px;
        border: 1px solid #ffccc7;
        border-radius: 8px;
        background: #fff2f0;
        color: #cf1322;
        font-size: 13px;
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
        else downKey.key = normalized === 'space' ? 'Space' : part.replace(/^Key([a-z])$/i, (_, letter) => letter.toUpperCase()).replace(/^Digit([0-9])$/i, '$1');
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

function downKeyToHotkey(downKey) {
    return [downKey.ctrl && 'Ctrl', downKey.alt && 'Alt', downKey.shift && 'Shift', downKey.meta && 'Super', downKey.key].filter(Boolean).join('+');
}

function HotkeyKeys({downKey}) {
    const parts = hotkeyDisplayParts(downKey);
    if (parts.length === 0) return <span className="hotkeyPlaceholder">未设置（点击录入）</span>;
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
    const [translationSettings, setTranslationSettings] = useState(DEFAULT_TRANSLATION_SETTINGS);
    const [collapsedTranslationServices, setCollapsedTranslationServices] = useState({});
    const [collapsedOcrServices, setCollapsedOcrServices] = useState({});
    const [translationNotice, setTranslationNotice] = useState('');
    const [ocrServices, setOcrServices] = useState([]);
    const [clipboardCount, setClipboardCount] = useState(100);
    const [clipboardText, setClipboardText] = useState(10);
    const [clipboardImage, setClipboardImage] = useState(5);
    const [clipboardFile, setClipboardFile] = useState(1);
    const [clipboardCountSwitch, setClipboardCountSwitch] = useState(true);
    const [clipboardTextSwitch, setClipboardTextSwitch] = useState(false);
    const [clipboardImageSwitch, setClipboardImageSwitch] = useState(false);
    const [clipboardFileSwitch, setClipboardFileSwitch] = useState(false);
    const [hotkeyAwaken, setHotkeyAwaken] = useState("Alt+Space");
    const [hotkeyClipboard, setHotkeyClipboard] = useState("Ctrl+Alt+V");
    const [hotkeySelection, setHotkeySelection] = useState("Ctrl+Alt+X");
    const [hotkeyFileJump, setHotkeyFileJump] = useState("Ctrl+G");
    const [hotkeyScreenshot, setHotkeyScreenshot] = useState((navigator.userAgent || '').includes('Mac') ? "Option+D" : "Ctrl+Alt+D");
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
    const [settingDirty, setSettingDirty] = useState(false);
    const [pythonInterpreter, setPythonInterpreter] = useState(null);
    const [pythonProbe, setPythonProbe] = useState(null);
    const [pythonProbeError, setPythonProbeError] = useState('');
    const [autoLaunch, setAutoLaunch] = useState(false);
    const [indexCounts, setIndexCounts] = useState({app: 0, file: 0});
    const [rebuildingIndex, setRebuildingIndex] = useState('');
    const hotkeyCaptureActive = useRef(false);
    const activeHotkeyField = useRef(null);
    const hotkeyCaptureTransition = useRef(Promise.resolve());
    const hotkeyReservedDraft = useRef(null);
    const snippetSettingsLoaded = useRef(false);
    const hotkeyAwakenRef = useRef(hotkeyAwaken);
    const hotkeyClipboardRef = useRef(hotkeyClipboard);
    const hotkeySelectionRef = useRef(hotkeySelection);
    const hotkeyFileJumpRef = useRef(hotkeyFileJump);
    const hotkeyScreenshotRef = useRef(hotkeyScreenshot);

    const loadIndexCounts = async () => {
        try {
            setIndexCounts(await invoke('get_index_counts'));
        } catch (error) {
            console.error('读取索引数量失败', error);
            setSettingNotice({type: 'error', text: `读取索引状态失败：${error}`});
        }
    };

    const rebuildIndex = async (type) => {
        setRebuildingIndex(type);
        try {
            await invoke(type === 'app' ? 'create_app_index' : 'create_file_index');
            setTimeout(loadIndexCounts, 1000);
        } catch (error) {
            console.error('重建索引失败', error);
            setRebuildingIndex('');
        }
    };

    useEffect(() => {
        const decryptPasswordFields = async (services, catalog) => Promise.all((services || []).map(async (service) => {
            const passwordFields = (catalog.find(item => item.id === service.provider)?.fields || [])
                .filter(field => field.type === 'password').map(field => field.key);
            const next = {...service, config: {...(service.config || {})}};
            for (const key of passwordFields) {
                for (const target of [next, next.config]) {
                    const value = String(target[key] || '');
                    if (value.startsWith('dpapi:')) {
                        try {
                            target[key] = await invoke('unprotect_secret', {value: value.slice('dpapi:'.length)});
                        } catch (error) {
                            console.warn(`解密配置字段 ${key} 失败`, error);
                            target[key] = '';
                        }
                    }
                }
            }
            return next;
        }));
        const loadTranslation = async () => {
            const storedTranslation = readTranslationSettings();
            const services = await decryptPasswordFields(storedTranslation.services, TRANSLATION_PROVIDER_CATALOG);
            const storedOcr = await decryptPasswordFields(storedTranslation.ocrServices, OCR_PROVIDER_CATALOG);
            setCollapsedTranslationServices(Object.fromEntries(services.map(service => [service.id, true])));
            setCollapsedOcrServices(Object.fromEntries(storedOcr.map(service => [service.id, true])));
            setTranslationSettings({...storedTranslation, services});
            let foundEnabled = false;
            setOcrServices(storedOcr.map(service => {
                if (!service.enabled) return service;
                if (foundEnabled) return {...service, enabled: false};
                foundEnabled = true;
                return service;
            }));
        };
        loadTranslation();
        invoke("get_app_settings").then((settings) => {
            const isMac = (navigator.userAgent || '').includes('Mac');
            const defaults = {
                lark: 'Alt+Space',
                cbd: 'Ctrl+Alt+V',
                selection: isMac ? 'Option+X' : 'Ctrl+Alt+X',
                fileJump: 'Ctrl+G',
                screenshot: isMac ? 'Option+D' : 'Ctrl+Alt+D'
            };
            const loadedHotkeys = {
                lark: settings.hotkeyAwaken ?? defaults.lark,
                cbd: settings.hotkeyClipboard ?? defaults.cbd,
                selection: settings.hotkeySelection ?? defaults.selection,
                fileJump: settings.hotkeyFileJump ?? defaults.fileJump,
                screenshot: settings.hotkeyScreenshot ?? defaults.screenshot,
            };
            for (const [name, value] of Object.entries(loadedHotkeys)) {
                const field = hotkeyFields()[name];
                field.ref.current = value;
                field.set(value);
            }
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
        invoke('get_auto_launch_enabled').then(setAutoLaunch).catch((error) => {
            console.error('读取开机启动状态失败', error);
            setSettingNotice({type: 'error', text: `读取开机启动状态失败：${error}`});
        });
        loadIndexCounts();
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
            snippetSettingsLoaded.current = true;
        }).catch((error) => setSnippetError(String(error)));
        loadCustomApps();
    }, []);

    const saveTranslationSettings = async () => {
        const normalized = normalizeTranslationSettings(translationSettings);
        if (!normalized.services.length) {
            setTranslationNotice('请至少添加一个翻译服务');
            return;
        }
        if (normalized.services.some(service => service.provider === 'baidu'
            ? (!service.appId.trim() || !service.appKey.trim())
            : service.provider === 'tengxun'
                ? (!service.secretId.trim() || !service.secretKey.trim())
                : (!service.apiKey.trim() && service.provider !== 'custom'))) {
            setTranslationNotice('请填写已添加服务所需的认证信息');
            return;
        }
        const enabledOcr = (ocrServices || []).filter(service => service.enabled);
        if (enabledOcr.length > 1) {
            setTranslationNotice('OCR 服务最多只能启用一个');
            return;
        }
        const baiduOcrMissingCredentials = enabledOcr.some(service => {
            if (service.provider !== 'baidu-ocr') return false;
            const config = service.config || {};
            return !String(config.apiKey || service.apiKey || '').trim()
                || !String(config.secretKey || service.secretKey || '').trim();
        });
        if (baiduOcrMissingCredentials) {
            setTranslationNotice('请填写百度 OCR 的 API Key 和 Secret Key');
            return;
        }
        try {
            const protectedFields = (service, catalog) => new Set(
                (catalog.find(item => item.id === service.provider)?.fields || [])
                    .filter(field => field.type === 'password')
                    .map(field => field.key)
            );
            const protectService = async (service, catalog) => {
                const next = {...service, config: {...(service.config || {})}};
                for (const key of protectedFields(service, catalog)) {
                    for (const target of [next, next.config]) {
                        const value = String(target[key] || '');
                        if (value && !value.startsWith('dpapi:')) {
                            target[key] = `dpapi:${await invoke('protect_secret', {value})}`;
                        }
                    }
                }
                return next;
            };
            const protectedServices = await Promise.all(normalized.services.map(service => protectService(service, TRANSLATION_PROVIDER_CATALOG)));
            const protectedOcrServices = await Promise.all((ocrServices || []).map(service => protectService(service, OCR_PROVIDER_CATALOG)));
            localStorage.setItem(TRANSLATION_SETTINGS_KEY, JSON.stringify({
                ...normalized,
                services: protectedServices,
                ocrServices: protectedOcrServices
            }));
            setTranslationSettings(normalized);
            setTranslationNotice('翻译设置已保存');
        } catch (error) {
            setTranslationNotice(`翻译设置保存失败：${String(error)}`);
        }
    };

    const addTranslationService = () => {
        const id = `service-${Date.now()}`;
        setCollapsedTranslationServices(current => ({...current, [id]: false}));
        setTranslationSettings(current => ({
            ...current,
            services: [...current.services, {
                id,
                provider: 'niutrans',
                name: '小牛翻译',
                enabled: true,
                apiKey: '',
                appId: '',
                appKey: '',
                baseUrl: '',
                model: '',
                dictNo: '',
                memoryNo: '',
                dictflag: false,
                dict: '',
                needIntervene: false
            }]
        }));
    };

    const updateTranslationService = (id, patch) => setTranslationSettings(current => ({
        ...current,
        services: current.services.map(service => service.id === id ? {...service, ...patch} : service)
    }));

    const removeTranslationService = (id) => setTranslationSettings(current => ({
        ...current,
        services: current.services.filter(service => service.id !== id)
    }));
    const addOcrService = () => {
        const id = `ocr-${Date.now()}`;
        setCollapsedOcrServices(current => ({...current, [id]: false}));
        setOcrServices(current => [...current, {
            id,
            provider: 'paddleocr-local',
            name: 'PaddleOCR（本地）',
            enabled: false,
            config: {endpoint: 'http://127.0.0.1:8866/ocr', language: 'ch'}
        }]);
    };
    const updateOcrService = (id, patch) => setOcrServices(current => current.map(service => {
        if (service.id === id) return {...service, ...patch};
        if (patch.enabled === true) return {...service, enabled: false};
        return service;
    }));
    const removeOcrService = (id) => setOcrServices(current => current.filter(service => service.id !== id));

    useEffect(() => () => {
        activeHotkeyField.current = null;
        hotkeyCaptureActive.current = false;
        hotkeyCaptureTransition.current.catch(() => {
        }).then(() =>
            invoke('set_hotkey_capture_active', {active: false})
        ).catch(console.error);
    }, []);

    useEffect(() => {
        let disposed = false;
        let unlisten;
        listen('hotkey-capture', ({payload}) => {
            const name = activeHotkeyField.current;
            if (!disposed && name && typeof payload === 'string') submitHotkeyCapture(name, payload);
        }).then((stop) => {
            if (disposed) stop();
            else unlisten = stop;
        }).catch(console.error);
        return () => {
            disposed = true;
            unlisten?.();
        };
    }, []);

    function hotkeyFields() {
        return {
            lark: {
                ref: hotkeyAwakenRef,
                set: setHotkeyAwaken,
                display: setLarkDisplayText,
                setting: 'hotkeyAwaken',
                argument: 'awaken'
            },
            cbd: {
                ref: hotkeyClipboardRef,
                set: setHotkeyClipboard,
                display: setCBDDisplayText,
                setting: 'hotkeyClipboard',
                argument: 'clipboard'
            },
            selection: {
                ref: hotkeySelectionRef,
                set: setHotkeySelection,
                display: setSelectionDisplayText,
                setting: 'hotkeySelection',
                argument: 'selection'
            },
            fileJump: {
                ref: hotkeyFileJumpRef,
                set: setHotkeyFileJump,
                display: setFileJumpDisplayText,
                setting: 'hotkeyFileJump',
                argument: 'fileJump'
            },
            screenshot: {
                ref: hotkeyScreenshotRef,
                set: setHotkeyScreenshot,
                display: setScreenshotDisplayText,
                setting: 'hotkeyScreenshot',
                argument: 'screenshot'
            },
        };
    }

    function hotkeyDraft(property) {
        return Object.fromEntries(Object.values(hotkeyFields()).map(field => [field[property], field.ref.current]));
    }

    function submitHotkeyCapture(name, value) {
        if (!hotkeyCaptureActive.current) return Promise.resolve();
        const candidate = downKeyToHotkey(hotkeyToDownKey(value));
        // 原生回传和 DOM 键盘事件共用队列；执行时读取最新草稿，避免重复注册和相互覆盖。
        const transition = hotkeyCaptureTransition.current.then(async () => {
            const field = hotkeyFields()[name];
            const draft = {...hotkeyDraft('argument'), [field.argument]: candidate};
            const reservation = JSON.stringify(draft);
            if (hotkeyReservedDraft.current !== reservation) {
                await invoke('reserve_hotkey_capture', draft);
                hotkeyReservedDraft.current = reservation;
            }
            if (field.ref.current !== candidate) {
                field.ref.current = candidate;
                field.set(candidate);
                setSettingDirty(true);
            }
            field.display({behavior: 'set', data: hotkeyToDownKey(candidate)});
            setSettingNotice({type: '', text: ''});
        }).catch(error => {
            const field = hotkeyFields()[name];
            field.display({behavior: 'set', data: hotkeyToDownKey(field.ref.current)});
            setSettingNotice({type: 'error', text: '快捷键暂时无法占用：' + error});
        });
        hotkeyCaptureTransition.current = transition;
        return transition;
    }

    async function loadCustomApps() {
        try {
            setCustomApps(await invoke("get_custom_app_indexes"));
            setCustomAppError('');
        } catch (error) {
            setCustomAppError(`读取自定义应用失败：${error}`);
        }
    }

    const [larkDisplayText, setLarkDisplayText] = useReducer(hotkeysFrameShow, {
        downKey: {
            alt: true,
            meta: false,
            ctrl: false,
            shift: false,
            key: "Space"
        }
    });
    const [cbdDisplayText, setCBDDisplayText] = useReducer(hotkeysFrameShow, {downKey: {}});
    const [selectionDisplayText, setSelectionDisplayText] = useReducer(hotkeysFrameShow, {downKey: {}});
    const [fileJumpDisplayText, setFileJumpDisplayText] = useReducer(hotkeysFrameShow, {downKey: {}});
    const [screenshotDisplayText, setScreenshotDisplayText] = useReducer(hotkeysFrameShow, {downKey: {}});

    useEffect(() => {
        setLarkDisplayText({behavior: "set", data: hotkeyToDownKey(hotkeyAwaken)});
        setCBDDisplayText({behavior: "set", data: hotkeyToDownKey(hotkeyClipboard)});
        setSelectionDisplayText({behavior: "set", data: hotkeyToDownKey(hotkeySelection)});
        setFileJumpDisplayText({behavior: "set", data: hotkeyToDownKey(hotkeyFileJump)});
        setScreenshotDisplayText({behavior: "set", data: hotkeyToDownKey(hotkeyScreenshot)});
    }, [hotkeyAwaken, hotkeyClipboard, hotkeySelection, hotkeyFileJump, hotkeyScreenshot]);


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
        const otherSettings = {
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
            activeHotkeyField.current = null;
            hotkeyCaptureActive.current = false;
            const save = hotkeyCaptureTransition.current.catch(() => {
            }).then(async () => {
                await invoke('set_hotkey_capture_active', {active: false});
                hotkeyReservedDraft.current = null;
                await invoke('save_setting', {settingInfo: {...otherSettings, ...hotkeyDraft('setting')}});
            });
            hotkeyCaptureTransition.current = save;
            await save;
            setSettingDirty(false);
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

    const handleOpenOnboarding = async () => {
        try {
            await invoke('open_onboarding');
            setSettingNotice({type: 'success', text: '新手向导已打开'});
        } catch (error) {
            setSettingNotice({type: 'error', text: '打开新手向导失败：' + error});
        }
    };

    const handleHotkeyCapture = async (active) => {
        if (active === hotkeyCaptureActive.current) return hotkeyCaptureTransition.current;
        hotkeyCaptureActive.current = active;
        const transition = hotkeyCaptureTransition.current.catch(() => {
        }).then(async () => {
            await invoke('set_hotkey_capture_active', {active});
            // 后端进入录制时用已保存绑定；下一次提交必须重新验证完整草稿。
            hotkeyReservedDraft.current = null;
        });
        hotkeyCaptureTransition.current = transition;
        try {
            await transition;
        } catch (error) {
            hotkeyCaptureActive.current = false;
            setSettingNotice({type: 'error', text: '快捷键录制状态切换失败：' + error});
            throw error;
        }
    }

    const handleIndexSettingSave = async () => {
        await invoke("save_index_settings", {
            settingInfo: {
                localAppSearchPaths: appSearchPaths,
                localAppSearchExcludePaths: appExcludePaths,
                localFileSearchPaths: fileSearchPaths,
                localFileSearchExcludePaths: excludePaths,
                localFileSearchExcludeTypes: excludeTypes
            }
        })
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

    const saveSnippets = async (next = {}) => {
        const enabled = next.enabled ?? snippetEnabled;
        const trigger = next.trigger ?? snippetTrigger;
        const nextSnippets = next.snippets ?? snippets;
        if ([...trigger].length !== 1 || /\s/.test(trigger)) {
            setSnippetError('触发符必须是一个非空白字符');
            return;
        }
        try {
            await invoke('save_snippet_settings', {
                settingInfo: {
                    enabled,
                    trigger,
                    snippets: nextSnippets,
                }
            });
            setSnippetError('');
        } catch (error) {
            setSnippetError(String(error));
        }
    };

    useEffect(() => {
        if (!snippetSettingsLoaded.current) return undefined;
        const timer = setTimeout(() => {
            saveSnippets({enabled: snippetEnabled, trigger: snippetTrigger, snippets});
        }, 250);
        return () => clearTimeout(timer);
    }, [snippetEnabled, snippetTrigger, snippets]);

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
        event.stopPropagation();
        if (event.key === 'Tab') return; // 保留键盘焦点导航。
        event.preventDefault();
        if (event.repeat) return;
        const downKey = {alt: event.altKey, meta: event.metaKey, ctrl: event.ctrlKey, shift: event.shiftKey, key: ''};
        const hasModifier = downKey.ctrl || downKey.alt || downKey.shift || downKey.meta;
        if (!hasModifier && (event.key === 'Backspace' || event.key === 'Delete')) {
            return submitHotkeyCapture(name, '');
        }
        if (!modifierKeyMap[event.key]) {
            // Ctrl+Alt 或输入法可能改变 event.key，字母/数字优先使用物理按键编码。
            downKey.key = /^Key[A-Z]$/.test(event.code) || /^Digit[0-9]$/.test(event.code)
                ? hotkeyToDownKey(event.code).key
                : event.code === 'Space' ? 'Space' : event.key;
        }
        hotkeyFields()[name].display({behavior: 'down', data: downKey});
        if (!downKey.key) return;
        if (!hasModifier) {
            hotkeyFields()[name].display({behavior: 'set', data: hotkeyToDownKey(hotkeyFields()[name].ref.current)});
            setSettingNotice({type: 'error', text: '快捷键必须至少包含一个修饰键（Ctrl、Alt、Shift 或 Win）'});
            return;
        }
        return submitHotkeyCapture(name, downKeyToHotkey(downKey));
    }

    const handleHotkeysUp = (event, name) => {
        event.stopPropagation();
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
        } else if (name === "selection") {
            setSelectionDisplayText({behavior: "up", data: upKey})
        } else if (name === "fileJump") {
            setFileJumpDisplayText({behavior: "up", data: upKey})
        } else if (name === "screenshot") {
            setScreenshotDisplayText({behavior: "up", data: upKey})
        }
    }

    function hotkeysFrameShow(stats, action) {
        let downKey = {}
        if (action.behavior === "set" || action.behavior === "down") {
            downKey = {...action.data}

        } else if (action.behavior === "up") {
            downKey = {...stats.downKey}
            if (!downKey.key || !(downKey.ctrl || downKey.alt || downKey.shift || downKey.meta)) {
                if (Object.values(modifierKeyMap).includes(action.data.key)) {
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
                    {key: 'app', label: '常规设置'},
                    {key: 'translation', label: '翻译服务'},
                    {key: 'index', label: '索引扫描'},
                    {key: 'custom', label: '手动应用'},
                    {key: 'snippets', label: '文本片段'}
                ]}/>
                {activeTab === 'translation' && <div style={{padding: '8px 18px 24px'}}>
                    <section className="snippetCard">
                        <h3 className="snippetHeading">翻译方向</h3>
                        <div
                            className="snippetHint">根据检测结果自动选择目标语言：简体中文翻译成英文，其他语言翻译成简体中文。
                        </div>
                        <div style={{
                            display: 'grid',
                            gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
                            gap: 12,
                            marginTop: 14
                        }}>
                            <label className="snippetField"><span className="snippetLabel">源语言</span><Select
                                value={translationSettings.sourceLang} options={[{value: 'auto', label: '自动检测'}]}
                                disabled style={{width: '100%'}}/></label>
                            <label className="snippetField"><span className="snippetLabel">简体中文目标</span><Select
                                value="en" options={[{value: 'en', label: '英语'}]} disabled
                                style={{width: '100%'}}/></label>
                            <label className="snippetField"><span className="snippetLabel">其他语言目标</span><Select
                                value="zh-CN" options={[{value: 'zh-CN', label: '简体中文'}]} disabled
                                style={{width: '100%'}}/></label>
                        </div>
                        <div style={{display: 'flex', gap: 18, alignItems: 'center', marginTop: 14}}>
                            <label className="snippetField"><span
                                className="snippetLabel">请求超时（毫秒）</span><InputNumber min={3000} max={30000}
                                                                                           step={500}
                                                                                           value={translationSettings.requestTimeoutMs}
                                                                                           onChange={(value) => setTranslationSettings(current => ({
                                                                                               ...current,
                                                                                               requestTimeoutMs: value || 8000
                                                                                           }))}/></label>
                            <label className="snippetSwitch"><Switch size="small"
                                                                     checked={translationSettings.autoClipboard}
                                                                     onChange={(checked) => setTranslationSettings(current => ({
                                                                         ...current,
                                                                         autoClipboard: checked
                                                                     }))}/><span>唤起时自动读取剪贴板</span></label>
                        </div>
                    </section>
                    <section className="snippetCard">
                        <div className="snippetListHeader">
                            <div><h3 className="snippetHeading">翻译服务</h3>
                                <div
                                    className="snippetHint">启用的服务会并行翻译，并分别展示结果；单个服务失败不影响其他服务。
                                </div>
                            </div>
                            <Button type="primary" onClick={addTranslationService}>添加服务</Button></div>
                        {translationSettings.services.length === 0 &&
                            <div className="snippetEmpty">还没有配置翻译服务。</div>}
                        {translationSettings.services.map(service => <div key={service.id} style={{
                            padding: 18,
                            marginTop: 14,
                            border: '1px solid #e8eaf0',
                            borderRadius: 12,
                            background: '#fff',
                            boxShadow: '0 2px 8px rgba(31,35,41,.04)'
                        }}>
                            <div style={{
                                display: 'flex',
                                justifyContent: 'space-between',
                                alignItems: 'center',
                                marginBottom: 16
                            }}>
                                <div style={{display: 'flex', gap: 10, alignItems: 'center'}}><Select
                                    value={service.provider} options={TRANSLATION_PROVIDER_CATALOG.map(item => ({
                                    value: item.id,
                                    label: item.name
                                }))} onChange={(value) => updateTranslationService(service.id, {
                                    provider: value,
                                    name: TRANSLATION_PROVIDER_CATALOG.find(item => item.id === value)?.name || value,
                                    ...(value === 'deepseek' ? {
                                        model: service.model || 'deepseek-flash',
                                        baseUrl: service.baseUrl || 'https://api.deepseek.com'
                                    } : {}),
                                    ...(value === 'zhipu' ? {
                                        model: service.model || 'glm-5.3',
                                        baseUrl: service.baseUrl && service.baseUrl.includes('bigmodel.cn') ? service.baseUrl : 'https://open.bigmodel.cn/api/paas/v4'
                                    } : {}),
                                    ...(value === 'tengxun' ? {
                                        region: service.region || 'ap-guangzhou',
                                        projectId: service.projectId || '0'
                                    } : {})
                                })} style={{width: 180}}/><Input value={service.name} placeholder="服务显示名称"
                                                                 onChange={(event) => updateTranslationService(service.id, {name: event.target.value})}
                                                                 style={{width: 190}}/></div>
                                <div style={{display: 'flex', gap: 8, alignItems: 'center'}}><Switch size="small"
                                                                                                     checked={service.enabled}
                                                                                                     onChange={(checked) => updateTranslationService(service.id, {enabled: checked})}/><Button
                                    danger type="text" aria-label="删除服务" title="删除服务"
                                    onClick={() => removeTranslationService(service.id)}
                                    style={{fontSize: 20, width: 28, height: 28, padding: 0, lineHeight: 1}}>×</Button>
                                </div>
                            </div>
                            <details
                                open={collapsedTranslationServices[service.id] !== true}
                                onToggle={(event) => {
                                    const isOpen = event.currentTarget?.open === true;
                                    setCollapsedTranslationServices(current => ({
                                        ...current,
                                        [service.id]: !isOpen
                                    }));
                                }}
                            >
                                <summary style={{cursor: 'pointer', padding: '6px 0', fontSize: '12px'}}>配置详情
                                </summary>
                                <div style={{padding: 14, borderRadius: 9, background: '#f8f9fb'}}>
                                    {service.provider === 'baidu' &&
                                        <div style={{display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12}}>
                                            <label className="snippetField"><span
                                                className="snippetLabel">APP ID</span><Input value={service.appId}
                                                                                             placeholder="从百度开发者信息获取"
                                                                                             onChange={(event) => updateTranslationService(service.id, {appId: event.target.value})}/></label>
                                            <label className="snippetField"><span
                                                className="snippetLabel">密钥</span><Input.Password
                                                value={service.appKey}
                                                placeholder="从百度开发者信息获取"
                                                onChange={(event) => updateTranslationService(service.id, {appKey: event.target.value})}/></label>
                                            <label className="snippetSwitch" style={{gridColumn: '1 / -1'}}><Switch
                                                size="small" checked={service.needIntervene}
                                                onChange={(checked) => updateTranslationService(service.id, {needIntervene: checked})}/><span>启用我的术语库</span></label>
                                        </div>}
                                    {service.provider === 'niutrans' &&
                                        <div style={{display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12}}>
                                            <label className="snippetField"><span
                                                className="snippetLabel">API Key</span><Input.Password
                                                value={service.apiKey} placeholder="输入小牛 API Key"
                                                onChange={(event) => updateTranslationService(service.id, {apiKey: event.target.value})}/></label>
                                            <div style={{display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8}}>
                                                <label
                                                    className="snippetField"><span
                                                    className="snippetLabel">术语词典 ID</span><Input
                                                    value={service.dictNo}
                                                    onChange={(event) => updateTranslationService(service.id, {dictNo: event.target.value})}/></label><label
                                                className="snippetField"><span
                                                className="snippetLabel">翻译记忆 ID</span><Input
                                                value={service.memoryNo}
                                                onChange={(event) => updateTranslationService(service.id, {memoryNo: event.target.value})}/></label>
                                            </div>
                                        </div>}
                                    {service.provider === 'tengxun' &&
                                        <div style={{display: 'grid', gridTemplateColumns: '1fr 1fr 1fr 1fr', gap: 12}}>
                                            <label className="snippetField"><span
                                                className="snippetLabel">SecretId</span><Input
                                                value={service.secretId || ''}
                                                onChange={(event) => updateTranslationService(service.id, {secretId: event.target.value})}/></label>
                                            <label className="snippetField"><span
                                                className="snippetLabel">SecretKey</span><Input.Password
                                                value={service.secretKey || ''}
                                                onChange={(event) => updateTranslationService(service.id, {secretKey: event.target.value})}/></label>
                                            <label className="snippetField"><span
                                                className="snippetLabel">地域</span><Input value={service.region || ''}
                                                                                           placeholder="ap-guangzhou"
                                                                                           onChange={(event) => updateTranslationService(service.id, {region: event.target.value})}/></label>
                                            <label className="snippetField"><span
                                                className="snippetLabel">项目 ID</span><Input
                                                value={service.projectId || ''} placeholder="0"
                                                onChange={(event) => updateTranslationService(service.id, {projectId: event.target.value})}/></label>
                                        </div>}
                                    {!['baidu', 'niutrans', 'tengxun'].includes(service.provider) &&
                                        <div style={{display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 12}}>
                                            <label
                                                className="snippetField"><span
                                                className="snippetLabel">API Key</span><Input.Password
                                                value={service.apiKey}
                                                onChange={(event) => updateTranslationService(service.id, {apiKey: event.target.value})}/></label><label
                                            className="snippetField"><span className="snippetLabel">模型</span><Input
                                            value={service.model}
                                            placeholder={service.provider === 'deepseek' ? 'deepseek-flash' : service.provider === 'zhipu' ? 'glm-5.3' : ''}
                                            onChange={(event) => updateTranslationService(service.id, {model: event.target.value})}/></label><label
                                            className="snippetField"><span
                                            className="snippetLabel">Base URL</span><Input
                                            value={service.baseUrl}
                                            placeholder={service.provider === 'deepseek' ? 'https://api.deepseek.com' : service.provider === 'zhipu' ? 'https://open.bigmodel.cn/api/paas/v4' : ''}
                                            onChange={(event) => updateTranslationService(service.id, {baseUrl: event.target.value})}/></label>
                                        </div>}
                                </div>
                            </details>
                        </div>)}
                        {translationNotice && <div
                            className={translationNotice.includes('失败') || translationNotice.includes('请') ? 'snippetError' : 'snippetHint'}
                            style={{marginTop: 12}}>{translationNotice}</div>}
                    </section>
                    <section className="snippetCard" style={{marginTop: 16}}>
                        <div className="snippetListHeader">
                            <div><h3 className="snippetHeading">OCR 服务</h3>
                                <div className="snippetHint">截图识别服务。识别完成后，文本会交给上面的翻译服务处理。</div>
                            </div>
                            <Button onClick={addOcrService}>添加 OCR 服务</Button></div>
                        {ocrServices.length === 0 && <div className="snippetEmpty">还没有配置 OCR 服务。</div>}
                        {ocrServices.map(service => {
                            const definition = OCR_PROVIDER_CATALOG.find(item => item.id === service.provider);
                            const config = service.config || {};
                            return <div key={service.id} style={{
                                padding: 18,
                                marginTop: 14,
                                border: '1px solid #e8eaf0',
                                borderRadius: 12,
                                background: '#fff'
                            }}>
                                <details
                                    open={collapsedOcrServices[service.id] !== true}
                                    onToggle={(event) => {
                                        const isOpen = event.currentTarget?.open === true;
                                        setCollapsedOcrServices(current => ({
                                            ...current,
                                            [service.id]: !isOpen
                                        }));
                                    }}
                                >
                                    <summary style={{cursor: 'pointer', padding: '6px 0', fontSize: '12px'}}>配置详情
                                    </summary>
                                    <div style={{
                                        display: 'flex',
                                        justifyContent: 'space-between',
                                        alignItems: 'center',
                                        marginBottom: 16
                                    }}>
                                        <div style={{display: 'flex', gap: 10}}><Select value={service.provider}
                                                                                        options={OCR_PROVIDER_CATALOG.map(item => ({
                                                                                            value: item.id,
                                                                                            label: item.name
                                                                                        }))}
                                                                                        onChange={(value) => updateOcrService(service.id, {
                                                                                            provider: value,
                                                                                            name: OCR_PROVIDER_CATALOG.find(item => item.id === value)?.name || value,
                                                                                            config: {}
                                                                                        })} style={{width: 210}}/><Input
                                            value={service.name}
                                            onChange={(event) => updateOcrService(service.id, {name: event.target.value})}
                                            style={{width: 190}}/></div>
                                        <div style={{display: 'flex', gap: 8}}><Switch size="small"
                                                                                       checked={service.enabled}
                                                                                       onChange={(checked) => updateOcrService(service.id, {enabled: checked})}/><Button
                                            danger type="text" aria-label="删除 OCR 服务" title="删除 OCR 服务"
                                            onClick={() => removeOcrService(service.id)}
                                            style={{fontSize: 20, width: 28, height: 28, padding: 0}}>×</Button></div>
                                    </div>
                                    <div style={{
                                        display: 'grid',
                                        gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
                                        gap: 12,
                                        padding: 14,
                                        borderRadius: 9,
                                        background: '#f8f9fb'
                                    }}>{(definition?.fields || []).map(field => <label className="snippetField"
                                                                                       key={field.key}><span
                                        className="snippetLabel">{field.label}{field.required ? ' *' : ''}</span>{field.type === 'boolean' ?
                                        <Switch size="small" checked={config[field.key] ?? field.default ?? false}
                                                onChange={(checked) => updateOcrService(service.id, {
                                                    config: {
                                                        ...config,
                                                        [field.key]: checked
                                                    }
                                                })}/> : field.type === 'select' ?
                                            <Select value={config[field.key] ?? field.default} options={field.options}
                                                    onChange={(value) => updateOcrService(service.id, {
                                                        config: {
                                                            ...config,
                                                            [field.key]: value
                                                        }
                                                    })} style={{width: '100%'}}/> : field.type === 'password' ?
                                                <Input.Password value={config[field.key] || ''}
                                                                onChange={(event) => updateOcrService(service.id, {
                                                                    config: {
                                                                        ...config,
                                                                        [field.key]: event.target.value
                                                                    }
                                                                })}/> :
                                                <Input value={config[field.key] || field.default || ''}
                                                       onChange={(event) => updateOcrService(service.id, {
                                                           config: {
                                                               ...config,
                                                               [field.key]: event.target.value
                                                           }
                                                       })}/>}</label>)}</div>
                                </details>
                            </div>;
                        })}
                    </section>
                    <div style={{
                        position: 'sticky',
                        bottom: 0,
                        zIndex: 5,
                        display: 'flex',
                        justifyContent: 'flex-end',
                        alignItems: 'center',
                        gap: 12,
                        margin: '16px -18px -24px',
                        padding: '12px 18px',
                        background: 'rgba(255,255,255,.94)',
                        borderTop: '1px solid #edf0f4',
                        boxShadow: '0 -3px 10px rgba(31,35,41,.04)'
                    }}>
                        {translationNotice && <span
                            className={translationNotice.includes('失败') || translationNotice.includes('请') ? 'snippetError' : 'snippetHint'}>{translationNotice}</span>}
                        <Button type="primary" onClick={saveTranslationSettings}>保存翻译与 OCR 设置</Button>
                    </div>
                </div>}
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
                        <div className="snippetHint">关键词不需要包含触发符，例如填写
                            email，使用时输入 {snippetTrigger || ';'}email。
                        </div>
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
                                    <Button type="text" danger size="small" icon="🗙"/>
                                </Popconfirm>
                            </div>)}
                        </div>
                    </section>

                </div>}
                {activeTab === 'index' && <div className="indexSettingsPane">
                    <header className="indexSettingsIntro">
                        <h2 className="indexSettingsTitle">索引扫描</h2>
                        <p className="indexSettingsDescription">设置应用的扫描范围，并过滤不需要进入搜索结果的目录和文件类型。</p>
                        <div className="indexCounts">
                            <div className="indexCountCard"><span>应用索引</span><span
                                className="indexCountValue">{indexCounts.app}</span><Button size="small"
                                                                                            loading={rebuildingIndex === 'app'}
                                                                                            disabled={Boolean(rebuildingIndex)}
                                                                                            onClick={() => rebuildIndex('app')}>重建</Button>
                            </div>
                            <div className="indexCountCard"><span>文件索引</span><span
                                className="indexCountValue">{indexCounts.file}</span><Button size="small"
                                                                                             loading={rebuildingIndex === 'file'}
                                                                                             disabled={Boolean(rebuildingIndex)}
                                                                                             onClick={() => rebuildIndex('file')}>重建</Button>
                            </div>
                        </div>
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
                                {appSearchPaths.map((path) => <Tag title={path} key={path} closable
                                                                   onClose={() => setAppSearchPaths(appSearchPaths.filter((item) => item !== path))}>{path}</Tag>)}
                            </div>
                            <div className="indexSettingAdd">
                                <Input value={newAppSearchPath} placeholder="例如 D:\\Apps"
                                       onChange={(event) => setNewAppSearchPath(event.target.value)}
                                       onPressEnter={addAppSearchPath}/>
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
                                {appExcludePaths.map((path) => <Tag title={path} key={path} closable
                                                                    onClose={() => setAppExcludePaths(appExcludePaths.filter((item) => item !== path))}>{path}</Tag>)}
                            </div>
                            <div className="indexSettingAdd">
                                <Input value={newAppExcludePath} placeholder="例如 D:\\Apps\\不需要扫描的目录"
                                       onChange={(event) => setNewAppExcludePath(event.target.value)}
                                       onPressEnter={addAppExcludePath}/>
                                <Button type="primary" ghost onClick={addAppExcludePath}>添加</Button>
                            </div>
                        </section>

                        <section className="indexSettingCard">
                            <div className="indexSettingHeader">
                                <div>
                                    <h3 className="indexSettingHeading">文件包含路径</h3>
                                    <div className="indexSettingHint">仅扫描和监听这些目录；包含与排除冲突时以排除为准
                                    </div>
                                </div>
                                <span
                                    className="indexSettingCount">{fileSearchPaths === null ? '未初始化' : `${fileSearchPaths.length} 项`}</span>
                            </div>
                            <div className="indexSettingList">
                                {fileSearchPaths === null &&
                                    <div className="indexSettingEmpty">尚未初始化，应用启动时将生成默认包含路径</div>}
                                {fileSearchPaths?.length === 0 &&
                                    <div className="indexSettingEmpty">已明确设置为空，不扫描任何目录</div>}
                                {(fileSearchPaths || []).map((path) => <Tag title={path} key={path} closable
                                                                            onClose={() => setFileSearchPaths(fileSearchPaths.filter((item) => item !== path))}>{path}</Tag>)}
                            </div>
                            <div className="indexSettingAdd">
                                <Input value={newFileSearchPath} placeholder="例如 C:\\Users\\admin\\Downloads 或 D:\\"
                                       onChange={(event) => setNewFileSearchPath(event.target.value)}
                                       onPressEnter={addFileSearchPath}/>
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
                                {excludePaths.map((path) => <Tag title={path} key={path} closable
                                                                 onClose={() => setExcludePaths(excludePaths.filter((item) => item !== path))}>{path}</Tag>)}
                            </div>
                            <div className="indexSettingAdd">
                                <Input value={newExcludePath} placeholder="例如 */node_modules 或 C:\\Windows"
                                       onChange={(event) => setNewExcludePath(event.target.value)}
                                       onPressEnter={addExcludePath}/>
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
                                {excludeTypes.map((type) => <Tag title={type} key={type} closable
                                                                 onClose={() => setExcludeTypes(excludeTypes.filter((item) => item !== type))}>{type}</Tag>)}
                            </div>
                            <div className="indexSettingAdd">
                                <Input value={newExcludeType} placeholder="例如 tmp"
                                       onChange={(event) => setNewExcludeType(event.target.value)}
                                       onPressEnter={addExcludeType}/>
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
                                   onChange={(event) => setCustomAppPath(event.target.value)}
                                   onPressEnter={addCustomApp}/>
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
                                    : <div className="customAppIconFallback"
                                           aria-hidden="true">{app.title?.trim().charAt(0).toUpperCase() || 'A'}</div>}
                                <div className="customAppMeta">
                                    <div className="customAppName" title={app.title}>{app.title}</div>
                                    <div className="customAppPath" title={app.path}>{app.path}</div>
                                </div>
                                <Popconfirm title="删除这个手动应用？" onConfirm={() => deleteCustomApp(app.id)}
                                            okText="删除" cancelText="取消">
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
                            <Switch checked={autoLaunch} onChange={handleAutoLaunchChange}/>
                        </div>
                    </section>
                    <section className="appSettingCard appSettingActionRow">
                        <div className="appSettingActionText">
                            <h3 className="snippetHeading">新手向导</h3>
                            <div className="snippetHint">重新查看快捷键和主要功能介绍，不会重置其他设置。</div>
                        </div>
                        <Button onClick={handleOpenOnboarding}>重新打开</Button>
                    </section>
                    <section className="appSettingCard">
                        <h3 className="snippetHeading">快捷键{settingDirty &&
                            <span className="settingDirtyMark" aria-label="有未保存修改">*</span>}</h3>
                        <div className="snippetHint">点击快捷键框后按下包含修饰键的组合键；按 Backspace 或 Delete
                            清空为未设置。修改后点击保存才会生效。
                        </div>
                        <div className="hotkeysFrame"
                             onFocusCapture={() => handleHotkeyCapture(true).catch(() => {
                             })}
                             onBlurCapture={(event) => {
                                 if (!event.currentTarget.contains(event.relatedTarget)) {
                                     activeHotkeyField.current = null;
                                     handleHotkeyCapture(false).catch(() => {
                                     });
                                 }
                             }}>
                            <div className="hotkeys-item">
                                <div>
                                    <div className="hotkeyName">打开百灵鸟</div>
                                    <div className="hotkeyDescription">显示或隐藏主搜索窗口</div>
                                </div>
                                <div tabIndex={0} className="hotkeys-input"
                                     role="textbox" aria-label="百灵鸟快捷键"
                                     onFocus={() => {
                                         activeHotkeyField.current = 'lark';
                                     }}
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
                                <div tabIndex={0} className="hotkeys-input"
                                     role="textbox" aria-label="剪贴板快捷键"
                                     onFocus={() => {
                                         activeHotkeyField.current = 'cbd';
                                     }}
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
                                <div tabIndex={0} className="hotkeys-input"
                                     role="textbox" aria-label="文件跳转快捷键"
                                     onFocus={() => {
                                         activeHotkeyField.current = 'fileJump';
                                     }}
                                     onKeyDown={(event) => handleHotkeysDown(event, 'fileJump')}
                                     onKeyUp={(event) => handleHotkeysUp(event, 'fileJump')}>
                                    <HotkeyKeys downKey={fileJumpDisplayText.downKey}/>
                                </div>
                            </div>
                            <div className="hotkeys-item">
                                <div>
                                    <div className="hotkeyName">划词翻译</div>
                                    <div className="hotkeyDescription">获取当前选区并打开翻译面板</div>
                                </div>
                                <div tabIndex={0} className="hotkeys-input"
                                     role="textbox" aria-label="划词翻译快捷键"
                                     onFocus={() => {
                                         activeHotkeyField.current = 'selection';
                                     }}
                                     onKeyDown={(event) => handleHotkeysDown(event, 'selection')}
                                     onKeyUp={(event) => handleHotkeysUp(event, 'selection')}>
                                    <HotkeyKeys downKey={selectionDisplayText.downKey}/>
                                </div>
                            </div>
                            <div className="hotkeys-item">
                                <div>
                                    <div className="hotkeyName">截图翻译</div>
                                    <div className="hotkeyDescription">使用截图区域识别文字并翻译（功能开发中）</div>
                                </div>
                                <div tabIndex={0} className="hotkeys-input"
                                     role="textbox" aria-label="截图翻译快捷键"
                                     onFocus={() => {
                                         activeHotkeyField.current = 'screenshot';
                                     }}
                                     onKeyDown={(event) => handleHotkeysDown(event, 'screenshot')}
                                     onKeyUp={(event) => handleHotkeysUp(event, 'screenshot')}>
                                    <HotkeyKeys downKey={screenshotDisplayText.downKey}/>
                                </div>
                            </div>
                        </div>
                    </section>

                    <section className="appSettingCard">
                        <h3 className="snippetHeading">剪贴板历史{settingDirty &&
                            <span className="settingDirtyMark" aria-label="有未保存修改">*</span>}</h3>
                        <div className="snippetHint">限制保存数量，或按内容类型设置保留天数。</div>
                        <div className="clipboardRetentionGrid">
                            <div className="settingSmallFrame">
                                <Checkbox checked={clipboardCountSwitch}
                                          onChange={(event) => {
                                              setClipboardCountSwitch(event.target.checked);
                                              setSettingDirty(true);
                                          }}>数量（个）</Checkbox>
                                <InputNumber size="small" min={10} max={200} value={clipboardCount}
                                             disabled={!clipboardCountSwitch}
                                             onChange={(value) => {
                                                 setClipboardCount(value ?? 100);
                                                 setSettingDirty(true);
                                             }} changeOnWheel/>
                            </div>
                            <div className="settingSmallFrame">
                                <Checkbox checked={clipboardTextSwitch}
                                          onChange={(event) => {
                                              setClipboardTextSwitch(event.target.checked);
                                              setSettingDirty(true);
                                          }}>文本（天）</Checkbox>
                                <InputNumber size="small" min={1} max={30} value={clipboardText}
                                             disabled={!clipboardTextSwitch}
                                             onChange={(value) => {
                                                 setClipboardText(value ?? 10);
                                                 setSettingDirty(true);
                                             }} changeOnWheel/>
                            </div>
                            <div className="settingSmallFrame">
                                <Checkbox checked={clipboardImageSwitch}
                                          onChange={(event) => {
                                              setClipboardImageSwitch(event.target.checked);
                                              setSettingDirty(true);
                                          }}>图片（天）</Checkbox>
                                <InputNumber size="small" min={1} max={15} value={clipboardImage}
                                             disabled={!clipboardImageSwitch}
                                             onChange={(value) => {
                                                 setClipboardImage(value ?? 5);
                                                 setSettingDirty(true);
                                             }} changeOnWheel/>
                            </div>
                            <div className="settingSmallFrame">
                                <Checkbox checked={clipboardFileSwitch}
                                          onChange={(event) => {
                                              setClipboardFileSwitch(event.target.checked);
                                              setSettingDirty(true);
                                          }}>文件（天）</Checkbox>
                                <InputNumber size="small" min={1} max={10} value={clipboardFile}
                                             disabled={!clipboardFileSwitch}
                                             onChange={(value) => {
                                                 setClipboardFile(value ?? 1);
                                                 setSettingDirty(true);
                                             }} changeOnWheel/>
                            </div>
                        </div>
                    </section>

                    <section className="appSettingCard">
                        <h3 className="snippetHeading">Python 环境{settingDirty &&
                            <span className="settingDirtyMark" aria-label="有未保存修改">*</span>}</h3>
                        <div className="snippetHint">
                            插件 Python 与外部 Python 索引脚本共用这个解释器；留空使用系统默认。指向虚拟环境时请选择
                            它下面的 Scripts\python.exe（不需要「激活」环境）。
                        </div>
                        <div style={{display: 'flex', gap: 8, alignItems: 'center'}}>
                            <Input size="small" value={pythonInterpreter || ''} allowClear
                                   placeholder="留空使用系统默认解释器"
                                   onChange={(event) => {
                                       setPythonInterpreter(event.target.value || null);
                                       setSettingDirty(true);
                                   }}
                                   onBlur={() => probePythonInterpreter(pythonInterpreter)}/>
                            <Button size="small" onClick={choosePythonInterpreter}>选择…</Button>
                            <Button size="small" onClick={() => probePythonInterpreter(pythonInterpreter)}>检测</Button>
                        </div>
                        <div className={pythonProbeError ? 'settingError' : 'snippetHint'} style={{marginTop: 8}}>
                            {pythonProbeSummary()}
                        </div>
                    </section>

                    <div className="appSettingFooter">
                        {settingNotice.text &&
                            <span className={settingNotice.type === 'error' ? 'settingError' : 'settingNotice'}>
                            {settingNotice.text}
                        </span>}
                        {settingDirty &&
                            <span className="settingDirtyHint"><span aria-hidden="true">*</span> 有未保存的修改</span>}
                        <Button onClick={() => {
                            handleSettingReset();
                            setSettingDirty(true);
                        }}>重置</Button>
                        <Button type="primary" onClick={handleSettingSave}>保存设置</Button>
                    </div>
                </div>}
            </div>
        </>
    );
};

export default Component;
