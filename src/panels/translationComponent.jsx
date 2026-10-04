import React, {useEffect, useRef, useState} from 'react';
import {Button, Input, Select, Empty} from 'antd';
import {CopyOutlined, DeleteOutlined, DownOutlined, RightOutlined, SwapOutlined} from '@ant-design/icons';
import {invoke} from '@tauri-apps/api/core';
import {readTranslationSettings} from '../translationSettings';
import niutransIcon from '../assets/小牛翻译.svg';
import baiduIcon from '../assets/百度翻译.svg';
import deepseek from '../assets/deepseek.svg';
import zhipuIcon from '../assets/智谱.svg';
import tengxunIcon from '../assets/腾讯翻译君.svg';
import unknownProviderIcon from '../assets/翻译.svg';

const {TextArea} = Input;

const providerIcons = {
    niutrans: niutransIcon,
    baidu: baiduIcon,
    deepseek: deepseek,
    zhipu: zhipuIcon,
    tengxun: tengxunIcon
};

export default function TranslationComponent() {
    const [source, setSource] = useState('');
    const [sourceLanguage, setSourceLanguage] = useState('auto');
    const [target, setTarget] = useState('zh-CN');
    const [services, setServices] = useState([]);
    const [results, setResults] = useState({});
    const [collapsed, setCollapsed] = useState({});
    const timeoutRef = useRef(8000);
    const requestRef = useRef(0);

    useEffect(() => {
        const settings = readTranslationSettings();
        const configured = settings.services.filter(service => service.enabled !== false);
        timeoutRef.current = settings.requestTimeoutMs || 8000;
        setServices(configured);
        setCollapsed(Object.fromEntries(configured.map(service => [service.id, true])));
        return () => {
            requestRef.current += 1;
        };
    }, []);

    const translate = async () => {
        const text = source.trim();
        if (!text) return;
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
                if (requestId !== requestRef.current) return;
                setResults(current => ({...current, [service.id]: {status: 'success', text: response.text || ''}}));
                if (response.text) {
                    setCollapsed(current => ({...current, [service.id]: false}));
                }
            } catch (error) {
                if (requestId !== requestRef.current) return;
                const message = String(error || '翻译失败').replace(/^Error:\s*/, '');
                setResults(current => ({...current, [service.id]: {status: 'error', text: message}}));
            }
        }));
    };

    const toSnake = text => text.trim().replace(/([a-z\d])([A-Z])/g, '$1_$2').replace(/[\s-]+/g, '_').toLowerCase();
    const toCamel = text => text.trim().toLowerCase().replace(/[-_\s]+(.)?/g, (_, c) => c ? c.toUpperCase() : '');
    const copyText = text => navigator.clipboard?.writeText(text || '');
    const copySource = transform => copyText(transform(source));

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
            <TextArea autoFocus value={source} onChange={event => setSource(event.target.value)}
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
                size="small" type="text" icon={<DeleteOutlined/>} danger onClick={() => setSource('')} aria-label="清空"
                title="清空"/></div>
        </section>
        <div style={{display: 'flex', justifyContent: 'center', alignItems: 'center', gap: 8, padding: '2px 0'}}><Select
            style={{minWidth: 116}} value={sourceLanguage} onChange={setSourceLanguage}
            options={[{value: 'auto', label: '自动检测'}, {value: 'zh-CN', label: '简体中文'}, {
                value: 'en',
                label: '英语'
            }]}/><Button icon={<SwapOutlined/>} onClick={() => {
            setSourceLanguage(target);
            setTarget(sourceLanguage === 'auto' ? 'en' : sourceLanguage);
        }} aria-label="交换语言" title="交换语言"/><Select style={{minWidth: 116}} value={target} onChange={setTarget}
                                                           options={[{value: 'zh-CN', label: '简体中文'}, {
                                                               value: 'en',
                                                               label: '英语'
                                                           }]}/></div>
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
