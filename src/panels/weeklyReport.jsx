import React, {useEffect, useMemo, useState} from "react";
import {DatePicker} from "antd";
import dayjs from "dayjs";
import "./weeklyReport.css";

const KEY = "larkWeeklyReports";
const pad = (n) => String(n).padStart(2, "0");
const iso = (date) => `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;

function monday(date = new Date()) {
    const d = new Date(date);
    d.setHours(0, 0, 0, 0);
    d.setDate(d.getDate() - ((d.getDay() + 6) % 7));
    return d;
}

function weekKey(date) {
    const start = monday(date);
    return iso(start);
}

function weekLabel(key) {
    const start = new Date(`${key}T00:00:00`);
    const end = new Date(start);
    end.setDate(end.getDate() + 6);
    return `${start.getMonth() + 1}/${start.getDate()} - ${end.getMonth() + 1}/${end.getDate()}`;
}

function weekdayLabel(value) {
    const date = new Date(`${value}T00:00:00`);
    return `周${["日", "一", "二", "三", "四", "五", "六"][date.getDay()]}`;
}

function readData() {
    try {
        return JSON.parse(localStorage.getItem(KEY) || "{}");
    } catch {
        return {};
    }
}

export default function WeeklyReport() {
    const currentWeek = weekKey(new Date());
    const [data, setData] = useState(readData);
    const [activeWeek, setActiveWeek] = useState(currentWeek);
    const weeks = useMemo(() => Array.from({length: 6}, (_, i) => {
        const d = monday();
        d.setDate(d.getDate() - i * 7);
        return weekKey(d);
    }), []);
    useEffect(() => {
        const trimmed = Object.fromEntries(weeks.map((week) => [week, data[week] || []]));
        if (!trimmed[currentWeek]) trimmed[currentWeek] = [];
        setData(trimmed);
        localStorage.setItem(KEY, JSON.stringify(trimmed));
    }, []);
    useEffect(() => localStorage.setItem(KEY, JSON.stringify(data)), [data]);
    const rows = (data[activeWeek] || []).slice().sort((a, b) => b.date.localeCompare(a.date));
    const start = new Date(`${activeWeek}T00:00:00`);
    const end = new Date(start);
    end.setDate(end.getDate() + 6);
    const updateRows = (next) => setData((all) => ({...all, [activeWeek]: next}));
    const addRow = () => updateRows([...(data[activeWeek] || []), {
        id: crypto.randomUUID?.() || String(Date.now()),
        title: "",
        hours: "",
        status: "已完成",
        note: "",
        date: iso(new Date())
    }]);
    const updateRow = (id, patch) => {
        if (Object.prototype.hasOwnProperty.call(patch, "hours")) {
            const value = patch.hours === "" ? "" : Math.min(8, Math.max(0, Number(patch.hours) || 0));
            patch = {...patch, hours: value};
        }
        updateRows((data[activeWeek] || []).map((row) => row.id === id ? {...row, ...patch} : row));
    };
    const total = rows.reduce((sum, row) => sum + (Number(row.hours) || 0), 0);
    const daily = rows.reduce((all, row) => {
        all[row.date] = (all[row.date] || 0) + (Number(row.hours) || 0);
        return all;
    }, {});
    const copyWeeklyReport = async () => {
        const grouped = new Map();
        (data[activeWeek] || []).slice().sort((a, b) => a.date.localeCompare(b.date)).forEach((row) => {
            const title = row.title.trim();
            if (!title) return;
            const item = grouped.get(title) || [];
            const note = row.note.trim();
            if (note && !item.includes(note)) item.push(note);
            grouped.set(title, item);
        });
        const content = Array.from(grouped.entries()).map(([title, notes], index) => `${index + 1}、${title}${notes.length ? `：${notes.join("，")}` : ""}`).join("\n");
        if (!content) return;
        try {
            await navigator.clipboard.writeText(content);
        } catch {
            const textarea = document.createElement("textarea");
            textarea.value = content;
            document.body.appendChild(textarea);
            textarea.select();
            document.execCommand("copy");
            textarea.remove();
        }
    };
    return <div className="weekly-page">
        <div className="weekly-head">
            <div><h1>周报</h1><p>记录每周工作进展与投入工时</p></div>
            <div className="weekly-head-actions"><button className="weekly-copy" onClick={copyWeeklyReport}>复制</button><button className="weekly-add" onClick={addRow}>＋ 新增</button></div>
        </div>
        <div className="weekly-tabs">{weeks.map((week) => <button key={week}
                                                                  className={week === activeWeek ? "active" : ""}
                                                                  onClick={() => setActiveWeek(week)}>{week === currentWeek ? "本周" : weekLabel(week)}<small>{week}</small>
        </button>)}</div>
        <div className="weekly-table-wrap">
            <table>
                <thead>
                <tr>
                    <th>标题</th>
                    <th>工时</th>
                    <th>注意</th>
                    <th>日期</th>
                    <th>周几</th>
                    <th>状态</th>
                    <th></th>
                </tr>
                </thead>
                <tbody>{rows.map((row) => <tr key={row.id}>
                    <td><input value={row.title} onChange={(e) => updateRow(row.id, {title: e.target.value})}
                               placeholder="工作内容"/></td>
                    <td><input className="hours" type="number" min="0" max="8" step="0.5" value={row.hours}
                               onChange={(e) => updateRow(row.id, {hours: e.target.value})}/></td>
                    <td><input value={row.note} onChange={(e) => updateRow(row.id, {note: e.target.value})}
                               placeholder="备注"/></td>
                    <td><DatePicker className="weekly-date-picker"
                                    value={row.date ? dayjs(row.date) : null}
                                    format="MM/DD"
                                    allowClear={false}
                                    disabledDate={(date) => date.isBefore(dayjs(start), "day") || date.isAfter(dayjs(end), "day")}
                                    onChange={(date) => date && updateRow(row.id, {date: date.format("YYYY-MM-DD")})}/>
                    </td>
                    <td className="weekly-weekday">{weekdayLabel(row.date)}</td>
                    <td><select value={row.status || "已完成"} onChange={(e) => updateRow(row.id, {status: e.target.value})}>
                        <option>已完成</option><option>进行中</option><option>未开始</option><option>阻塞</option>
                    </select></td>
                    <td>
                        <button className="weekly-delete" aria-label="删除记录" title="删除记录"
                                onClick={() => updateRows((data[activeWeek] || []).filter((item) => item.id !== row.id))}>🗑
                        </button>
                    </td>
                </tr>)}</tbody>
            </table>
            {!rows.length && <div className="weekly-empty">本周还没有记录，点击“新增”开始填写。</div>}</div>
        <div className="weekly-summary"><div className="weekly-total"><span>本周总工时</span><strong>{total}<small> h</small></strong></div>
            <div
                className="daily-summary">{Object.entries(daily).sort(([a], [b]) => a.localeCompare(b)).map(([date, hours]) =>
                <span key={date}><b>{weekdayLabel(date)}</b><em>{hours}h</em></span>)}</div>
        </div>
    </div>;
}



