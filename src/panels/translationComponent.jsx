import React, {useEffect, useRef, useState} from 'react';
import {Button, Input, Select, Empty} from 'antd';
import {CopyOutlined, DeleteOutlined, DownOutlined, ReloadOutlined, RightOutlined, SwapOutlined, TranslationOutlined} from '@ant-design/icons';
import {invoke} from '@tauri-apps/api/core';
import {readTranslationSettings} from '../translationSettings';
import niutransIcon from '../assets/小牛翻译.svg';
import baiduIcon from '../assets/百度翻译.svg';
import deepseek from '../assets/deepseek.svg';
import zhipuIcon from '../assets/智谱.svg';
import tengxunIcon from '../assets/腾讯翻译君.svg';
import unknownProviderIcon from '../assets/翻译.svg';
import './translationComponent.css';

const {TextArea} = Input;

const providerIcons = {
    niutrans: niutransIcon,
    baidu: baiduIcon,
    deepseek: deepseek,
    zhipu: zhipuIcon,
    tengxun: tengxunIcon
};

// Keep these values aligned with NiuTrans' text translation API language
// codes. The UI uses zh-CN for simplified Chinese and the Rust provider maps
// it to the API's zh code.
const translationLanguageOptions = [
    {value: 'zh-CN', label: '简体中文'},
    {value: 'en', label: '英语'},
    {value: 'ja', label: '日语'},
    {value: 'ko', label: '韩语'},
    {value: 'fr', label: '法语'},
    {value: 'es', label: '西班牙语'},
    {value: 'ru', label: '俄语'},
    {value: 'ar', label: '阿拉伯语'},
    {value: 'pt', label: '葡萄牙语'},
    {value: 'th', label: '泰语'},
    {value: 'mn', label: '蒙古语（外蒙）'},
    {value: 'de', label: '德语'},
    {value: 'lo', label: '老挝语'},
    {value: 'am', label: '阿姆哈拉语'},
    {value: 'bn', label: '孟加拉语'},
    {value: 'cs', label: '捷克语'},
    {value: 'da', label: '丹麦语'},
    {value: 'et', label: '爱沙尼亚语'},
    {value: 'fi', label: '芬兰语'},
    {value: 'fil', label: '菲律宾语'},
    {value: 'he', label: '希伯来语'},
    {value: 'hr', label: '克罗地亚语'},
    {value: 'hu', label: '匈牙利语'},
    {value: 'jv', label: '印尼爪哇语'},
    {value: 'mg', label: '马尔加什语'},
    {value: 'mi', label: '毛利语'},
    {value: 'mk', label: '马其顿语'},
    {value: 'ms', label: '马来语'},
    {value: 'nl', label: '荷兰语'},
    {value: 'no', label: '挪威语'},
    {value: 'pl', label: '波兰语'},
    {value: 'ro', label: '罗马尼亚语'},
    {value: 'sk', label: '斯洛伐克语'},
    {value: 'sl', label: '斯洛文尼亚语'},
    {value: 'sm', label: '萨摩亚语'},
    {value: 'sq', label: '阿尔巴尼亚语'},
    {value: 'sr', label: '塞尔维亚语'},
    {value: 'su', label: '印尼巽他语'},
    {value: 'sv', label: '瑞典语'},
    {value: 'sw', label: '斯瓦希里语'},
    {value: 'uk', label: '乌克兰语'},
    {value: 'vi', label: '越南语'},
    {value: 'yo', label: '约鲁巴语'},
    {value: 'yue', label: '粤语'},
    {value: 'kk', label: '哈萨克语（西里尔）'},
    {value: 'km', label: '高棉语'},
    {value: 'my', label: '缅甸语'},
    {value: 'id', label: '印尼语'},
    {value: 'ps', label: '普什图语'},
    {value: 'hi', label: '印地语'},
    {value: 'fa', label: '波斯语'},
    {value: 'ta', label: '泰米尔语'},
    {value: 'si', label: '僧伽罗语'},
    {value: 'it', label: '意大利语'},
    {value: 'tr', label: '土耳其语'},
    {value: 'bg', label: '保加利亚语'},
    {value: 'ckb', label: '库尔德语（索拉尼语）'}
];

const sourceLanguageOptions = [{value: 'auto', label: '自动检测'}, ...translationLanguageOptions];

export default function TranslationComponent({initialText = '', autoTranslate = false, translationRequestId, screenshotResult, onScreenshotClose, onScreenshotRetry, onOpenTranslation}) {
    const [source, setSource] = useState('');
    const [sourceLanguage, setSourceLanguage] = useState('auto');
    const [target, setTarget] = useState('zh-CN');
    const [services, setServices] = useState([]);
    const [ocrService, setOcrService] = useState(null);
    const [results, setResults] = useState({});
    const [collapsed, setCollapsed] = useState({});
    const [screenshotText, setScreenshotText] = useState('');
    const [ocrStatus, setOcrStatus] = useState('idle');
    const [ocrError, setOcrError] = useState('');
    const timeoutRef = useRef(8000);
    const requestRef = useRef(0);
    const sourceRef = useRef('');
    const translatingRef = useRef(false);

    useEffect(() => {
        if (typeof initialText === 'string') {
            sourceRef.current = initialText;
            setSource(initialText);
            requestRef.current += 1;
        }
    }, [initialText]);

    useEffect(() => {
        let cancelled = false;
        if (!screenshotResult) {
            setScreenshotText('');
            setOcrStatus('idle');
            setOcrError('');
            return () => {
                cancelled = true;
            };
        }
        setScreenshotText(screenshotResult.ocrText || screenshotResult.text || '');
        setOcrError(screenshotResult.error || '');
        if (screenshotResult.error || !screenshotResult.path) {
            setOcrStatus(screenshotResult.error ? 'error' : 'idle');
            return () => {
                cancelled = true;
            };
        }
        if (!ocrService) {
            setOcrStatus('error');
            setOcrError('请先在设置中启用一个 OCR 服务');
            return () => {
                cancelled = true;
            };
        }
        setOcrStatus('loading');
        setOcrError('');
        invoke('ocr_image', {
            request: {
                provider: ocrService.provider,
                imagePath: screenshotResult.path,
                service: {...(ocrService.config || {}), ...ocrService},
                timeoutMs: timeoutRef.current,
            }
        }).then(response => {
            if (cancelled) return;
            setScreenshotText(response?.text || '');
            setOcrStatus('success');
        }).catch(error => {
            if (cancelled) return;
            setOcrStatus('error');
            setOcrError(String(error || 'OCR 识别失败').replace(/^Error:\s*/, ''));
        });
        return () => {
            cancelled = true;
        };
    }, [screenshotResult, ocrService]);

    useEffect(() => {
        const settings = readTranslationSettings();
        const configured = settings.services.filter(service => service.enabled !== false);
        const configuredOcr = settings.ocrServices.find(service => service.enabled === true) || null;
        timeoutRef.current = settings.requestTimeoutMs || 8000;
        setServices(configured);
        setOcrService(configuredOcr);
        setCollapsed(Object.fromEntries(configured.map(service => [service.id, true])));
        return () => {
            requestRef.current += 1;
        };
    }, []);

    const translate = async (value = source) => {
        const text = value.trim();
        if (!text) return;
        if (translatingRef.current) return;
        translatingRef.current = true;
        const nonWhitespaceChars = Array.from(text).filter(char => !/\s/u.test(char));
        const chineseChars = nonWhitespaceChars.filter(char => /[\u3400-\u9fff]/u.test(char));
        const isChinese = nonWhitespaceChars.length > 0
            && chineseChars.length / nonWhitespaceChars.length >= 0.6;
        const resolvedTarget = sourceLanguage === 'auto'
            ? (isChinese ? 'en' : 'zh-CN')
            : target;
        if (sourceLanguage === 'auto' && resolvedTarget !== target) {
            setTarget(resolvedTarget);
        }
        const requestId = ++requestRef.current;
        setResults(current => Object.fromEntries(services.map(service => [service.id, {
            ...current[service.id],
            status: 'loading',
            text: ''
        }])));
        try {
            await Promise.all(services.map(async service => {
                try {
                    const response = await invoke('translate_text', {
                        request: {
                            provider: service.provider || service.id,
                            text,
                            sourceLang: sourceLanguage,
                            targetLang: resolvedTarget,
                            service: {...(service.config || {}), ...service},
                            timeoutMs: timeoutRef.current,
                        },
                    });
                    if (requestId !== requestRef.current || sourceRef.current.trim() !== text) return;
                    setResults(current => ({...current, [service.id]: {status: 'success', text: response.text || ''}}));
                    if (response.text) {
                        setCollapsed(current => ({...current, [service.id]: false}));
                    }
                } catch (error) {
                    if (requestId !== requestRef.current || sourceRef.current.trim() !== text) return;
                    const message = String(error || '翻译失败').replace(/^Error:\s*/, '');
                    setResults(current => ({...current, [service.id]: {status: 'error', text: message}}));
                }
            }));
        } finally {
            translatingRef.current = false;
        }
    };

    useEffect(() => {
        if (!autoTranslate || !initialText.trim() || !services.length) return;
        const timer = setTimeout(() => {
            if (sourceRef.current.trim() === initialText.trim()) {
                translate(initialText);
            }
        }, 300);
        return () => clearTimeout(timer);
        // The request is keyed by the selection event, waits for configured
        // services, and is cancelled if the user edits the inserted text.
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [autoTranslate, initialText, services, translationRequestId]);

    const toSnake = text => text.trim().replace(/([a-z\d])([A-Z])/g, '$1_$2').replace(/[\s-]+/g, '_').toLowerCase();
    const toCamel = text => text.trim().toLowerCase().replace(/[-_\s]+(.)?/g, (_, c) => c ? c.toUpperCase() : '');
    const copyText = text => navigator.clipboard?.writeText(text || '');
    const copySource = transform => copyText(transform(source));

    if (screenshotResult) {
        return <div style={{height: '100%', boxSizing: 'border-box', padding: 16, overflowY: 'auto'}}>
            <div style={{display: 'flex', flexDirection: 'column', gap: 14, height: '100%', minHeight: 0}}>
                <div style={{flex: '0 0 auto', overflow: 'hidden', borderRadius: 8, background: '#f0f1f3'}}>
                    <img src={screenshotResult.dataUrl} alt="已选截图" style={{display: 'block', width: '100%', maxHeight: 300, objectFit: 'contain'}}/>
                </div>
                <label style={{display: 'flex', flexDirection: 'column', gap: 6, minHeight: 0, flex: '1 1 auto', color: '#595959', fontSize: 12, fontWeight: 500}}>
                    <span>OCR 结果{ocrStatus === 'loading' ? '（识别中…）' : ''}</span>
                    <Input.TextArea value={screenshotText} onChange={event => setScreenshotText(event.target.value)} placeholder={ocrError || 'OCR 结果将在这里显示，也可以手动编辑'} style={{flex: 1, resize: 'none'}}/>
                </label>
                <div style={{display: 'flex', justifyContent: 'flex-end', gap: 8, flex: '0 0 auto'}}>
                    <Button icon={<CopyOutlined/>} onClick={() => copyText(screenshotText)} disabled={!screenshotText.trim()} aria-label="复制 OCR 结果" title="复制 OCR 结果"/>
                    <Button icon={<ReloadOutlined/>} onClick={onScreenshotRetry} aria-label="重新截图" title="重新截图"/>
                    <Button type="primary" icon={<TranslationOutlined/>} onClick={() => onOpenTranslation(screenshotText)} disabled={!screenshotText.trim()} aria-label="翻译" title="翻译"/>
                </div>
            </div>
        </div>;
    }

    return <div style={{
        height: '100%',
        minWidth: 0,
        padding: 16,
        display: 'flex',
        flexDirection: 'column',
        gap: 1,
        boxSizing: 'border-box',
        overflow: 'hidden'
    }}>
        <section style={{
            width: '100%',
            minWidth: 0,
            boxSizing: 'border-box',
            border: '1px solid #d9d9d9',
            borderRadius: 12,
            background: '#fff',
            padding: 11,
            boxShadow: '0 1px 2px rgba(0,0,0,.04)'
        }}>
            <TextArea autoFocus value={source} onChange={event => {
                const value = event.target.value;
                sourceRef.current = value;
                requestRef.current += 1;
                setSource(value);
            }}
                      placeholder="输入原文，按 Enter 翻译" autoSize={{minRows: 5, maxRows: 10}} onPressEnter={event => {
                if (!event.shiftKey) {
                    event.preventDefault();
                    translate();
                }
            }}/>
            <div style={{display: 'flex', gap: 4, marginTop: 8, flexWrap: 'wrap'}}><Button size="small" type="text"
                                                                                           icon={<CopyOutlined/>}
                                                                                           style={{color: '#1677ff'}}
                                                                                           onClick={() => copySource(x => x)}
                                                                                           aria-label="复制原文"
                                                                                           title="复制原文"/><Button
                size="small" type="text" icon={<CopyOutlined/>} style={{color: '#13a673'}}
                onClick={() => copySource(toSnake)} aria-label="复制蛇形命名" title="复制蛇形命名"/><Button size="small"
                                                                                                            type="text"
                                                                                                            icon={
                                                                                                                <CopyOutlined/>}
                                                                                                            style={{color: '#722ed1'}}
                                                                                                            onClick={() => copySource(toCamel)}
                                                                                                            aria-label="复制小驼峰命名"
                                                                                                            title="复制小驼峰命名"/><Button
                size="small" type="text" icon={<DeleteOutlined/>} danger onClick={() => {
                    sourceRef.current = '';
                    requestRef.current += 1;
                    setSource('');
                }} aria-label="清空"
                title="清空"/></div>
        </section>
        <div style={{display: 'flex', justifyContent: 'center', alignItems: 'center', gap: 8, padding: '2px 0'}}><Select
            showSearch
            virtual={false}
            optionFilterProp="label"
            popupMatchSelectWidth={false}
            style={{minWidth: 150, flex: '1 1 0', maxWidth: 260}}
            value={sourceLanguage}
            onChange={setSourceLanguage}
            popupClassName="translation-language-select-dropdown"
            options={sourceLanguageOptions}/><Button icon={<SwapOutlined/>} onClick={() => {
            setSourceLanguage(target);
            setTarget(sourceLanguage === 'auto' ? 'en' : sourceLanguage);
        }} aria-label="交换语言" title="交换语言"/><Select
            showSearch
            virtual={false}
            optionFilterProp="label"
            popupMatchSelectWidth={false}
            style={{minWidth: 150, flex: '1 1 0', maxWidth: 260}}
            value={target}
            onChange={setTarget}
            popupClassName="translation-language-select-dropdown"
            options={translationLanguageOptions}/></div>
        <section style={{
            width: '100%',
            maxWidth: '100%',
            minWidth: 0,
            minHeight: 180,
            flex: '1 1 0',
            boxSizing: 'border-box',
            overflowY: 'auto',
            overflowX: 'hidden',
            border: '1px solid #d9d9d9',
            borderRadius: 12,
            background: '#fff',
            padding: 12,
            boxShadow: '0 1px 2px rgba(0,0,0,.04)'
        }}>
            {!services.length &&
                <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="请先在设置中配置并启用翻译服务"/>}
            <div style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
                gap: 8,
                minWidth: 0,
                width: '100%',
                maxWidth: '100%',
                alignItems: 'start'
            }}>{services.map(service => {
                const item = results[service.id] || {};
                const toggle = () => setCollapsed(current => ({...current, [service.id]: !current[service.id]}));
                const stopToggle = event => event.stopPropagation();
                const providerIcon = providerIcons[service.provider || service.id] || unknownProviderIcon;
                return <div key={service.id} style={{
                    width: '100%',
                    maxWidth: '100%',
                    minWidth: 0,
                    boxSizing: 'border-box',
                    border: '1px solid #e6e8ed',
                    borderRadius: 9,
                    overflow: 'hidden',
                    background: '#fff'
                }}>
                    <div role="button" tabIndex={0} onClick={toggle} onKeyDown={event => {
                        if (event.key === 'Enter' || event.key === ' ') {
                            event.preventDefault();
                            toggle();
                        }
                    }} style={{
                        display: 'flex',
                        width: '100%',
                        maxWidth: '100%',
                        minWidth: 0,
                        boxSizing: 'border-box',
                        border: 0,
                        background: '#fafbfc',
                        padding: '6px 8px',
                        alignItems: 'center',
                        gap: 5,
                        cursor: 'pointer',
                        textAlign: 'left',
                        fontSize: 13,
                        overflow: 'hidden'
                    }}>{collapsed[service.id] ? <RightOutlined/> : <DownOutlined/>}<img src={providerIcon} alt=""
                                                                                        aria-hidden="true" style={{
                        width: 18,
                        height: 18,
                        objectFit: 'contain',
                        flex: '0 0 auto'
                    }}/><b style={{
                        minWidth: 0,
                        flex: '1 1 auto',
                        fontSize: 13,
                        fontWeight: 500,
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap'
                    }}>{service.name || service.provider}</b><span onClick={stopToggle} style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 1,
                        flexShrink: 0
                    }}><Button type="text" size="small" icon={<CopyOutlined/>}
                               style={{color: '#1677ff', paddingInline: 3}} disabled={!item.text} onClick={event => {
                        stopToggle(event);
                        copyText(item.text);
                    }} aria-label="复制原文" title="复制原文"/><Button type="text" size="small" icon={<CopyOutlined/>}
                                                                       style={{color: '#13a673', paddingInline: 3}}
                                                                       disabled={!item.text} onClick={event => {
                        stopToggle(event);
                        copyText(toSnake(item.text));
                    }} aria-label="复制蛇形命名" title="复制蛇形命名"/><Button type="text" size="small"
                                                                               icon={<CopyOutlined/>} style={{
                        color: '#722ed1',
                        paddingInline: 3
                    }} disabled={!item.text} onClick={event => {
                        stopToggle(event);
                        copyText(toCamel(item.text));
                    }} aria-label="复制小驼峰命名" title="复制小驼峰命名"/></span><span style={{
                        marginLeft: 3,
                        flexShrink: 0,
                        color: item.status === 'loading' ? '#1677ff' : item.status === 'error' ? '#ff4d4f' : '#8c8c8c',
                        fontSize: 11
                    }}>{item.status === 'loading' ? '翻译中' : item.status === 'error' ? '失败' : '结果'}</span></div>
                    {!collapsed[service.id] && <div style={{
                        width: '100%',
                        maxWidth: '100%',
                        minWidth: 0,
                        boxSizing: 'border-box',
                        padding: 10,
                        overflow: 'hidden'
                    }}>
                        <div style={{
                            width: '100%',
                            maxWidth: '100%',
                            minWidth: 0,
                            minHeight: 64,
                            boxSizing: 'border-box',
                            whiteSpace: 'pre-wrap',
                            overflowWrap: 'anywhere',
                            wordBreak: 'break-word',
                            fontSize: 13,
                            lineHeight: 1.6,
                            color: item.text ? '#262626' : '#bfbfbf'
                        }}>{item.text || (item.status === 'loading' ? '正在翻译…' : '等待翻译')}</div>
                    </div>}</div>
            })}</div>
        </section>
    </div>;
}
