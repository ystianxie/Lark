import React, { useState } from "react";
import { Button, Input } from "@lark/plugin-sdk";
import "./style.css";

export default function App() {
  const [name, setName] = useState("");
  const [message, setMessage] = useState("修改 src/App.jsx，开始编写你的插件。");
  return <main className="lark-jsx-starter">
    <h2>我的 Lark 插件</h2>
    <p>{message}</p>
    <Input placeholder="输入名字" value={name} onChange={setName} />
    <Button type="primary" onClick={() => setMessage(`你好，${name || "Lark"}！`)}>打个招呼</Button>
  </main>;
}
