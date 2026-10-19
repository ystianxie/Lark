import { cpSync, mkdirSync, readFileSync, writeFileSync, readdirSync } from "node:fs";
import { execSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { join, resolve, basename } from "node:path";
import { build } from "vite";
import { definePluginConfig } from "../packages/plugin-sdk/src/vite.js";

const root = fileURLToPath(new URL("../", import.meta.url));
const sdk = join(root, "packages/plugin-sdk");
const output = join(root, "plugins/_sdk");
const metadata = JSON.parse(readFileSync(join(sdk, "package.json"), "utf8"));
const archive = `lark-plugin-sdk-${metadata.version}.tgz`;
mkdirSync(output, { recursive: true });
// 固定命令，不拼接用户输入；npm 自己处理包的 files/exports。
execSync("npm pack --ignore-scripts --pack-destination ../../plugins/_sdk", { cwd: sdk, stdio: "inherit" });
const template = join(output, "template");
cpSync(join(sdk, "template"), template, {
  recursive: true,
  filter: path => !["node_modules", "dist"].includes(basename(path)),
});
const packagePath = join(template, "package.json");
writeFileSync(packagePath, readFileSync(packagePath, "utf8").replaceAll("__SDK_ARCHIVE__", archive));
cpSync(join(sdk, "README.md"), join(output, "README.md"));
const config = definePluginConfig({ root: template })({ command: "build" });
// 发布模板预构建使用 Lark 已安装的依赖；用户项目使用本地 tgz 安装 SDK。
await build({ ...config, configFile: false, resolve: {
  ...config.resolve,
  alias: [{ find: /^@lark\/plugin-sdk$/, replacement: resolve(sdk, "src/index.jsx") }],
} });
// 防止开发者误把 node_modules 放进随安装包附带的开发材料。
for (const name of readdirSync(output)) {
  if (name === "node_modules") throw new Error("_sdk 不应携带 node_modules");
}
console.log(`SDK 与 JSX 模板已准备：${output}`);
