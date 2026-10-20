import React, {useEffect, useRef, useState} from 'react';
import {Alert, Button, Select, Space, Tag} from 'antd';
import {invoke} from '@tauri-apps/api/core';
import {open} from '@tauri-apps/plugin-dialog';
import './trayIconsPanel.css';

const pathKey = path => path.toLowerCase();
const fileName = path => path.split(/[\\/]/).pop();
const softwareName = item => item.tooltip?.trim() || fileName(item.executable).replace(/\.exe$/i, '');

export default function TrayIconsPanel() {
    const [entries, setEntries] = useState([]);
    const [rules, setRules] = useState([]);
    const [selected, setSelected] = useState();
    const [busy, setBusy] = useState(false);
    const [refreshing, setRefreshing] = useState(false);
    const [notice, setNotice] = useState(null);
    const refreshInFlight = useRef(false);
    const mounted = useRef(false);

    const software = Array.from(entries.reduce((map, item) => {
        const key = pathKey(item.executable);
        const current = map.get(key);
        if (current) current.count += 1;
        else map.set(key, {...item, key, count: 1});
        return map;
    }, new Map()).values()).sort((a, b) => softwareName(a).localeCompare(softwareName(b), 'zh-CN'));
    const active = new Set(software.map(item => item.key));

    const refresh = async () => {
        if (refreshInFlight.current) return;
        refreshInFlight.current = true;
        setRefreshing(true);
        try {
            const [scan, saved] = await Promise.allSettled([invoke('list_tray_icons'), invoke('get_tray_icon_rules')]);
            if (!mounted.current) return;
            if (saved.status === 'fulfilled') setRules(saved.value);
            if (scan.status === 'fulfilled') {
                setEntries(scan.value);
                // Keep selection by executable even when Windows gives its icon a new ID.
                setSelected(current => scan.value.some(item => pathKey(item.executable) === current) ? current : undefined);
            }
            const errors = [scan, saved].filter(result => result.status === 'rejected').map(result => String(result.reason));
            setNotice(errors.length ? {type: 'error', text: errors.join('；')} : null);
        } finally {
            refreshInFlight.current = false;
            if (mounted.current) setRefreshing(false);
        }
    };
    useEffect(() => {
        mounted.current = true;
        refresh();
        return () => { mounted.current = false; };
    }, []);

    const replace = async (custom) => {
        setBusy(true);
        setNotice(null);
        try {
            let imagePath = null;
            if (custom) {
                imagePath = await open({multiple: false, filters: [{name: '图标图片', extensions: ['png', 'ico', 'jpg', 'jpeg', 'webp']}]});
                if (!imagePath) return;
            }
            const target = software.find(item => item.key === selected);
            if (!target) throw new Error('目标软件已退出，请刷新后重试');
            await invoke('replace_tray_icon', {id: target.id, imagePath});
            setRules(await invoke('get_tray_icon_rules'));
            setNotice({type: 'success', text: '已替换并保存规则。Lark 运行时会持续重新应用，重启后也会自动加载。'});
        } catch (error) {
            setNotice({type: 'error', text: String(error)});
        } finally {
            setBusy(false);
        }
    };

    const remove = async (executable) => {
        setBusy(true);
        try {
            await invoke('remove_tray_icon_rule', {executable});
            setRules(await invoke('get_tray_icon_rules'));
            setNotice({type: 'info', text: '已删除规则并停止保持。目标软件下次更新图标或重启时会恢复原图标。'});
        } catch (error) {
            setNotice({type: 'error', text: String(error)});
        } finally {
            setBusy(false);
        }
    };

    return <div className="tray-icons-panel">
        <section className="snippetCard">
            <h3 className="snippetHeading">托盘图标替换（实验）</h3>
            <p className="snippetHint">替换后自动保存，Lark 运行时持续保持。透明图标仍占一个位置，消息闪烁时可能短暂显示原图标。</p>
            <Space direction="vertical" size="middle" style={{width: '100%'}}>
                <Space.Compact style={{width: '100%'}}>
                    <Select style={{flex: 1, minWidth: 0}} placeholder="选择托盘软件" value={selected}
                            onChange={setSelected} disabled={busy || refreshing} loading={refreshing} showSearch optionFilterProp="searchText"
                            notFoundContent={refreshing ? '正在扫描…' : '没有找到活动托盘软件'}
                            options={software.map(item => ({value: item.key, label: softwareName(item), searchText: `${softwareName(item)} ${item.executable}`, item}))}
                            optionRender={({data}) => <div className="tray-software-option">
                                <div>{data.label}{data.item.count > 1 && <span className="tray-option-count">{data.item.count} 个图标</span>}</div>
                                <div className="tray-software-path" title={data.item.executable}>{data.item.executable}</div>
                            </div>}/>
                    <Button onClick={refresh} disabled={busy} loading={refreshing}>刷新</Button>
                </Space.Compact>
                <div className="tray-list-summary">{refreshing ? '正在刷新软件列表…' : `共 ${software.length} 个活动托盘软件`}</div>
                <Space wrap>
                    <Button type="primary" disabled={!selected || busy || refreshing} onClick={() => replace(false)}>换成透明图标</Button>
                    <Button disabled={!selected || busy || refreshing} onClick={() => replace(true)}>选择图片替换</Button>
                </Space>
                <div>
                    <h4 className="tray-rules-heading">已保存的规则 <span>{rules.length}</span></h4>
                    {rules.length === 0 && <p className="snippetHint">暂无规则，替换一次即可自动保存。</p>}
                    {rules.map(rule => <div key={rule.executable} className="tray-rule">
                        <div className="tray-rule-info">
                            <div className="tray-rule-title"><span>{softwareName(rule)}</span><Tag>{rule.custom ? '自定义图片' : '透明图标'}</Tag></div>
                            <div className="tray-software-path" title={rule.executable}>{rule.executable}</div>
                            <div className="tray-rule-status">{active.has(pathKey(rule.executable)) ? '已检测到图标 · 持续保持' : '等待软件启动或下次刷新'}</div>
                        </div>
                        <Button size="small" disabled={busy || refreshing} onClick={() => remove(rule.executable)}>删除</Button>
                    </div>)}
                </div>
                {notice && <Alert type={notice.type} message={notice.text} showIcon/>}
            </Space>
        </section>
    </div>;
}
