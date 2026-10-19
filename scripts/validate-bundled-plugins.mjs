import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../plugins/", import.meta.url));
const ids = new Set();
for (const directory of await readdir(root, { withFileTypes: true })) {
  if (!directory.isDirectory() || directory.name === "_sdk") continue;
  const pluginRoot = path.join(root, directory.name);
  const manifest = JSON.parse(await readFile(path.join(pluginRoot, "manifest.json"), "utf8"));
  if (manifest.id !== directory.name || ids.has(manifest.id)) {
    throw new Error(`插件 ID 重复或与目录不一致：${directory.name}`);
  }
  ids.add(manifest.id);
  const assets = [manifest.entry?.path, manifest.icon,
    ...(manifest.workflows || []).map(workflow => workflow.icon)].filter(Boolean);
  if (!manifest.entry?.path) throw new Error(`${manifest.id} 缺少入口`);
  if ((manifest.permissions || []).includes("python.execute")) assets.push("python/main.py");
  for (const asset of assets) {
    const resolved = path.resolve(pluginRoot, asset);
    if (!resolved.startsWith(pluginRoot + path.sep) || !(await stat(resolved)).isFile()) {
      throw new Error(`${manifest.id} 资源缺失或越出插件目录：${asset}`);
    }
  }
}
console.log(`已验证 ${ids.size} 个附带插件：${[...ids].join(", ")}`);

const sdkRoot = path.join(root, "_sdk");
const template = path.join(sdkRoot, "template");
const sdkPackage = JSON.parse(await readFile(path.join(template, "package.json"), "utf8"));
const dependency = sdkPackage.dependencies["@lark/plugin-sdk"];
if (typeof dependency !== "string" || !/^file:\.\.\/_sdk\/lark-plugin-sdk-[0-9][^/\\]*\.tgz$/.test(dependency)) {
  throw new Error("JSX 模板必须依赖同目录附带的 SDK 安装包");
}
for (const file of [path.join(sdkRoot, path.basename(dependency)), path.join(template, "dist/main.js"),
  path.join(template, "src/App.jsx"), path.join(sdkRoot, "README.md")]) {
  if (!(await stat(file)).isFile()) throw new Error(`SDK 附带资源缺失：${file}`);
}
console.log("已验证附带 SDK 安装包和可运行的 JSX 模板");
