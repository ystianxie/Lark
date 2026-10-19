# Lark 插件 SDK（初版）

插件作者直接导入 JSX 组件。本地预览和 Lark 使用同一份页面，构建后不需要替换 import 或 Lark 安装路径。

## 直接运行示例

在 `examples/jsx-plugin` 目录执行：

```sh
npm install
npm run dev
```

打开终端显示的本地地址，即可编辑 `src/App.jsx` 并查看效果。SDK 是一个独立包，源码在 `packages/plugin-sdk`；示例用 `file:../../packages/plugin-sdk` 安装它，无需发布到 npm。第三方只需要 SDK 文件夹和示例文件夹，不需要 Lark 宿主源码。移动目录后要相应修改这个安装路径。也可以在 SDK 目录执行 `npm pack`，再在自己的项目里 `npm install /路径/lark-plugin-sdk-0.1.0.tgz`。

```jsx
import React, { useState } from "react";
import { Button, Input } from "@lark/plugin-sdk";

export default function App() {
  const [name, setName] = useState("");
  return <div>
    <Input value={name} onChange={setName} />
    <Button onClick={() => console.log(name)}>提交</Button>
  </div>;
}
```

`onChange={setName}` 会把输入的文字存到 `name` 中。复选框的 `onChange` 给出 `true` 或 `false`；下拉框给出选项的值；Tabs 给出选中的 key。不需要接触 `event.target.value`。

## 页面、预览入口、正式入口

- `src/App.jsx`：插件作者主要修改的页面。
- `src/dev.jsx`：用 `previewPlugin(App, element)` 在浏览器显示页面。
- `src/main.jsx`：用 `export const activate = definePlugin(App)` 提供 Lark 插件入口。
- `vite.config.js`：`import { definePluginConfig } from "@lark/plugin-sdk/vite"; export default definePluginConfig();`

初版模板使用 JSX，代码中保留 `import React from "react"`。包附带 TS 类型声明；自己的 TSX 项目可以复用 SDK，需自行配置 TSX 构建入口（当前构建助手固定使用 `src/main.jsx`）。

## 安装版 Lark 中开发

在组件库点击“新增插件 → JSX 界面插件”，填写 ID 和名称，即可生成完整项目和可运行的初始页面。SDK 和模板随 Lark 一起提供在 `resources/plugins/_sdk`，不需要下载 Lark 源码。

也可以手动把 `_sdk/template` 复制成与 `_sdk` 同级的插件目录，再修改 manifest 的 id、name 和 keywords。模板的 package.json 已配置 `file:../_sdk/lark-plugin-sdk-版本号.tgz`；包名仍是 `@lark/plugin-sdk`，它不是安装路径。

开发需要 Node.js（包含 npm），插件目录需要可写。第一次在新插件目录执行 `npm install`，这会安装本地 SDK，并从 npm 下载 React、Vite 等依赖；Lark 不附带 Node.js 或 node_modules。

```sh
npm install
npm run dev
```

主要修改 `src/App.jsx`。修改完成后：

```sh
npm run build
```

默认直接更新当前插件的 `dist/main.js`，回到组件库点击“刷新”即可重新加载。manifest 和源码保留在当前目录，不需要复制 release 文件夹或替换 import 路径。初始模板已经构建好，创建后可直接在 Lark 打开。

仓库内的 `examples/jsx-plugin` 演示了另一种用法：`definePluginConfig({ output: "release" })` 会生成 `release/jsx-demo/` 完整插件目录，便于复制分发。默认 `definePluginConfig()` 用于在插件目录直接开发。

修改 SDK 源码后运行 `pnpm sdk:prepare` 更新安装包与模板。`pnpm build:desktop` 以及 Tauri 的打包流程会自动执行这一步并校验资源；生成内容位于 `plugins/_sdk`，最终随现有 resources 配置一起打包。

构建助手会把 React 和 SDK 需要的代码放进 JS，把 CSS 也嵌入 JS，不保留需要运行时寻找的 npm 包路径。图片/字体请使用可内联的导入或 data URL，不要在 CSS 中引用需要从 Blob 相对路径查找的文件。`assets` 目录用于 manifest 图标，构建时原样复制。初版不自动复制 Python 脚本等额外资源。

SDK 尚未发布到 npm，不能直接运行 `npm install @lark/plugin-sdk`。本地路径安装和 `npm pack` 都可用。

## 当前组件

| 组件 | 常用写法 |
| --- | --- |
| Button | `<Button type="primary" onClick={submit}>提交</Button>` |
| Input | `<Input value={text} onChange={setText} />` |
| Password | `<Password value={password} onChange={setPassword} />` |
| TextArea | `<TextArea value={text} onChange={setText} rows={3} />` |
| Select | `<Select value={value} onChange={setValue} options={[{value: "a", label: "选项 A"}]} />` |
| Checkbox | `<Checkbox checked={checked} onChange={setChecked}>启用</Checkbox>` |
| Tabs | `<Tabs activeKey={tab} onChange={setTab} items={[{key: "a", label: "设置"}]} />` |

Button/Checkbox 的文字、Select 选项文字、Tabs 页签文字使用字符串。Tabs 只负责页签，页面内容用普通 JSX 按选中 key 显示。初版不支持把任意 JSX 传进这些宿主控件，也不支持所有 Ant Design 组件。布局仍可直接使用 `div`、`section`、CSS 等。

Button 支持的图标字符串：`copy`、`delete`、`load`、`plus`、`save`、`send`、`upload`、`code`、`close`。

在 Lark 中，按钮等使用宿主控件和主题；在浏览器中，使用同一份控件实现和 Ant Design 默认主题。预览颜色不会完全等同于 Lark。

## 读取配置与调用 Lark

```jsx
import { usePluginContext } from "@lark/plugin-sdk";
const { api, ui, input } = usePluginContext(); // 在组件内调用
const values = await api.getConfig();
await api.setConfig({ name: "小明" });
```

真实 Lark 配置要在 manifest 的 `config.fields` 声明，否则宿主不会保存该键。示例已经声明 name、notes。浏览器预览的配置只保存在内存，刷新页面就会清空。

Python、剪贴板、文件等能力依然需要 manifest 权限和真实 Lark 环境。本地预览默认对这些调用报出“需要在 Lark 中测试”，不会伪装成调用成功；也可在预览入口显式传入测试实现：

```jsx
import { createDevContext, previewPlugin } from "@lark/plugin-sdk/dev";
const context = createDevContext({
  api: { runPython: async () => ({ example: "本地测试数据" }) },
});
previewPlugin(App, document.getElementById("root"), context);
```

## 包里负责的事

SDK 负责把 JSX 控件连接到 `context.ui.mountControl`，并自动更新和移除控件。插件作者不需要做字符串替换、寻找 Lark 安装路径、手动创建控件容器。开发预览用的 Ant Design 不会被正式入口引入；正式插件仍会包含自己的 React 来显示插件页面，按钮等复用 Lark 的 Ant Design。

## 回归检查

先在示例目录运行 `npm run build` 和 `npm run dev`，再打开 `/smoke.html`。这个页面像 Lark 一样通过 Blob 动态加载构建后的 JS，不依赖开发时的 JSX 加载。检查输入、下拉框、复选框、页签和按钮；点击“卸载插件”后应显示活动控件 0，点击“从本地重新加载 JS”后应恢复活动控件 10，插件样式始终只有 1 份。

这覆盖了 JS 加载和组件桥接，但不代替真正的 Tauri/Lark 权限与 Python 调用验证。
