import React, { useState } from "react";
import { Button, Checkbox, Input, Password, Select, Tabs, TextArea, usePluginContext } from "@lark/plugin-sdk";
import "./style.css";

export default function App() {
  const { api, input } = usePluginContext();
  const [name, setName] = useState(input.text || "");
  const [password, setPassword] = useState("");
  const [language, setLanguage] = useState("中文");
  const [enabled, setEnabled] = useState(true);
  const [notes, setNotes] = useState("");
  const [tab, setTab] = useState("edit");
  const [result, setResult] = useState("还没有提交");
  const [message, setMessage] = useState("");
  const [disabled, setDisabled] = useState(false);

  const save = async () => {
    try {
      await api.setConfig({ name, notes });
      setMessage("已保存");
    } catch (error) { setMessage(String(error)); }
  };
  const load = async () => {
    try {
      const config = await api.getConfig();
      setName(config.name || "");
      setNotes(config.notes || "");
      setMessage("已读取");
    } catch (error) { setMessage(String(error)); }
  };
  return (
    <main className="sdk-demo">
      <h2>Lark JSX 插件示例</h2>
      <p>同一份页面既可以在浏览器预览，也可以构建后放进 Lark。</p>
      <Tabs activeKey={tab} onChange={setTab} items={[{ key: "edit", label: "编辑" }, { key: "about", label: "说明" }]} />
      {tab === "edit" ? <>
        <label>名字 <Input aria-label="名字" placeholder="输入名字" value={name} onChange={setName} disabled={disabled || undefined} /></label>
        <label>密码 <Password aria-label="密码" value={password} onChange={setPassword} /></label>
        <label>语言 <Select aria-label="语言" value={language} onChange={setLanguage} options={[{ value: "中文", label: "中文" }, { value: "English", label: "English" }]} style={{ width: 150 }} /></label>
        <Checkbox checked={enabled} onChange={setEnabled}>启用问候</Checkbox>
        <label>备注 <TextArea aria-label="备注" value={notes} onChange={setNotes} rows={3} /></label>
        <div className="sdk-actions">
          <Button type="primary" onClick={() => setResult(enabled ? `${name || "朋友"}，你好！语言：${language}` : "问候已关闭")}>提交</Button>
          <Button onClick={() => setDisabled(value => !value)}>切换输入禁用</Button>
          <Button onClick={save}>保存</Button>
          <Button onClick={load}>读取</Button>
        </div>
        <output aria-label="结果">{result}</output>
        <p role="status">{message}</p>
      </> : <p>界面由 SDK 组件组成，不需要直接调用 mountControl。</p>}
    </main>
  );
}
