import React from "react";
import { createRoot } from "react-dom/client";
import { Button, ConfigProvider, Select, theme } from "antd";

// Plugins are loaded as standalone JS, so bare imports such as "antd" cannot
// resolve there. Mount the host's controls without shipping another React copy.
export function mountPluginControl(container, type, props = {}) {
  const Component = {Button, Select}[type];
  if (!Component) throw new Error(`不支持的插件控件：${type}`);
  const root = createRoot(container);
  const render = () => {
    const dark = document.documentElement.dataset.theme === "dark";
    const {label, ...options} = props;
    root.render(
      <ConfigProvider theme={{
        algorithm: dark ? theme.darkAlgorithm : theme.defaultAlgorithm,
        token: {fontSize: 13, fontFamily: "var(--font-family-ui)", colorPrimary: dark ? "#7aa7ff" : "#4f7df3"},
      }}>
        <Component {...(type === "Select" ? {getPopupContainer: () => container} : {})} {...options}>
          {type === "Button" ? label : undefined}
        </Component>
      </ConfigProvider>
    );
  };
  const observer = new MutationObserver(render);
  observer.observe(document.documentElement, {attributes: true, attributeFilter: ["data-theme"]});
  render();
  return () => { observer.disconnect(); root.unmount(); };
}
