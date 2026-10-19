import React, { createContext, useContext, useEffect, useRef } from "react";
import { createRoot } from "react-dom/client";

const PluginContext = createContext(null);

export function usePluginContext() {
  const context = useContext(PluginContext);
  if (!context) throw new Error("请通过 definePlugin 或 previewPlugin 挂载插件页面");
  return context;
}

function control(type) {
  function Control({ children, ...props }) {
    const { ui } = usePluginContext();
    const container = useRef(null);
    const mounted = useRef(null);
    const previous = useRef({});
    const options = { ...props };
    if (type === "Button" || type === "Checkbox") {
      if (children != null && typeof children !== "string" && typeof children !== "number") {
        throw new Error(`${type} 的文字请使用字符串或数字`);
      }
      options.label = children ?? props.label;
    }
    if (props.onChange && ["Input", "Password", "TextArea", "Checkbox"].includes(type)) {
      options.onChange = (event) => props.onChange(type === "Checkbox"
        ? (typeof event === "boolean" ? event : Boolean(event?.target?.checked))
        : (event?.target?.value ?? event ?? ""));
    }
    useEffect(() => {
      if (typeof ui?.mountControl !== "function") throw new Error("当前环境缺少 ui.mountControl");
      const element = document.createElement("span");
      element.style.display = "contents";
      container.current.append(element);
      const dispose = ui.mountControl(element, type, {});
      mounted.current = dispose;
      return () => {
        mounted.current = null;
        // 子控件有独立 React root，等外层 React 完成清理后再卸载它。
        queueMicrotask(() => { dispose(); element.remove(); });
      };
    }, [ui?.mountControl]);
    useEffect(() => {
      const next = Object.fromEntries(Object.keys(previous.current).map(key => [key, undefined]));
      Object.assign(next, options);
      mounted.current?.update(next);
      previous.current = options;
    });
    return <span ref={container} style={{ display: "inline-flex", minWidth: 0, verticalAlign: "middle" }} />;
  }
  Control.displayName = `Lark${type}`;
  return Control;
}

export const Button = control("Button");
export const Input = control("Input");
export const Password = control("Password");
export const TextArea = control("TextArea");
export const Select = control("Select");
export const Checkbox = control("Checkbox");
export const Tabs = control("Tabs");

export function definePlugin(App) {
  return function activate(context) {
    let root;
    const unmount = () => {
      root?.unmount();
      root = null;
    };
    return {
      mount(container, options = {}) {
        unmount();
        const currentContext = {
          ...context,
          input: { ...context.input, text: options.input ?? context.input?.text ?? "" },
        };
        root = createRoot(container);
        root.render(<PluginContext.Provider value={currentContext}><App /></PluginContext.Provider>);
      },
      unmount,
    };
  };
}
