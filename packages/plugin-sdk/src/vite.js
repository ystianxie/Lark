import { cpSync, mkdirSync, readFileSync, copyFileSync } from "node:fs";
import { resolve, join } from "node:path";
import { defineConfig } from "vite";

// 不向运行文件保留 react/SDK 等裸 import，也不生成需要额外加载的 CSS。
export function definePluginConfig({ root = process.cwd(), output = "in-place" } = {}) {
  root = resolve(root);
  const manifest = JSON.parse(readFileSync(join(root, "manifest.json"), "utf8"));
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(manifest.id)) throw new Error("插件 id 必须由小写字母、数字和连字符组成");
  if (manifest.entry?.type !== "js" || manifest.entry.path !== "dist/main.js") {
    throw new Error("SDK 示例构建要求 manifest.entry 为 js / dist/main.js");
  }
  if (!["in-place", "release"].includes(output)) throw new Error("output 只能是 in-place 或 release");
  const destination = output === "release" ? join(root, "release", manifest.id) : root;
  return defineConfig(({ command }) => ({
    define: { "process.env.NODE_ENV": JSON.stringify(command === "build" ? "production" : "development") },
    root,
    publicDir: false,
    resolve: { dedupe: ["react", "react-dom"] },
    build: {
      outDir: join(destination, "dist"),
      emptyOutDir: true,
      lib: { entry: join(root, "src/main.jsx"), formats: ["es"], fileName: () => "main.js" },
      rollupOptions: { output: { inlineDynamicImports: true } },
    },
    plugins: [{
      name: "lark-plugin-package",
      enforce: "post",
      generateBundle(_, bundle) {
        const styles = Object.values(bundle).filter(file => file.type === "asset" && file.fileName.endsWith(".css"));
        const entry = Object.values(bundle).find(file => file.type === "chunk" && file.isEntry);
        if (entry && styles.length) {
          const css = styles.map(file => String(file.source)).join("\n");
          entry.code = `(()=>{const styleKey=${JSON.stringify(manifest.id)};let style=[...document.querySelectorAll('style[data-lark-plugin]')].find(node=>node.dataset.larkPlugin===styleKey);if(!style){style=document.createElement('style');style.dataset.larkPlugin=styleKey;document.head.append(style);}style.textContent=${JSON.stringify(css)};})();\n` + entry.code;
          for (const style of styles) delete bundle[style.fileName];
        }
        for (const file of Object.values(bundle)) {
          if (file.type === "chunk" && (file.imports.length || file.dynamicImports.length)) {
            this.error("插件构建仍包含外部 JS 依赖，请将依赖一起打包");
          }
          if (file.type === "asset") this.error(`插件生成了额外资源 ${file.fileName}，图片或字体请改用内联 data URL`);
        }
      },
      closeBundle() {
        if (destination === root) return;
        mkdirSync(destination, { recursive: true });
        copyFileSync(join(root, "manifest.json"), join(destination, "manifest.json"));
        cpSync(join(root, "assets"), join(destination, "assets"), { recursive: true });
      },
    }],
  }));
}
