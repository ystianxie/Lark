# JSX 插件开发

需要先安装 Node.js（包含 npm），并确保当前插件目录可写。

在此目录打开终端，第一次执行：

```sh
npm install
npm run dev
```

主要修改 `src/App.jsx`。浏览器预览不需要启动 Lark；Python、剪贴板等宿主能力需要在 Lark 内测试。

修改完成后执行：

```sh
npm run build
```

构建结果直接写入当前插件的 `dist/main.js`。回到 Lark 组件库点击“刷新”，再打开插件即可，不需要重启 Lark。初始页面已经构建好，创建后可以直接打开。

`manifest.json` 管理插件名称、搜索关键词、权限及配置项。`src/main.jsx` 是 Lark 入口，`src/dev.jsx` 是浏览器预览入口，`vite.config.js` 使用 SDK 提供的构建配置。

SDK 来自旁边 `_sdk` 目录中的本地安装包。代码中保持 `from "@lark/plugin-sdk"`，build 会把所需代码合入 JS，不需要替换安装路径。目录结构保持为 `plugins/_sdk` 与 `plugins/你的插件`。复制 `_sdk/template` 手动创建时，请修改 manifest 的 id、name 和 workflow 的 keywords；ID 应与目录名一致。

首次 npm install 仍需要下载 React、Vite 等依赖，Lark 不附带 Node.js 或 node_modules。普通用户运行构建好的插件不需要这些开发工具。SDK 组件及宿主能力说明见 `../_sdk/README.md`。
