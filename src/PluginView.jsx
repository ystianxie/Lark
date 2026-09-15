import { useEffect, useRef, useState } from "react";
import { loadPluginRuntime } from "./pluginRuntime";

export default function PluginView({ manifest, input, onClose }) {
  const containerRef = useRef(null);
  const [error, setError] = useState("");

  useEffect(() => {
    let disposed = false;
    let runtime;
    const mount = async () => {
      try {
        runtime = await loadPluginRuntime(manifest, {
          text: input || "",
          ui: { close: onClose },
        });
        if (!disposed && typeof runtime?.mount === "function") {
          await runtime.mount(containerRef.current, { input: input || "" });
        }
      } catch (err) {
        if (!disposed) setError(String(err));
      }
    };
    mount();
    return () => {
      disposed = true;
      try { runtime?.unmount?.(); } catch (_) { /* plugin cleanup must not break host */ }
      if (containerRef.current) containerRef.current.replaceChildren();
    };
  }, [manifest, input, onClose]);

  return (
    <div className="pluginViewRoot" ref={containerRef} style={{ height: "calc(100vh - 75px)", width: "100%" }}>
      {error ? <div className="pluginViewError">插件加载失败：{error}</div> : null}
    </div>
  );
}
