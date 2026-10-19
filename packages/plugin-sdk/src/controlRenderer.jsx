import React from "react";
import { createRoot } from "react-dom/client";
import { CopyOutlined, DeleteOutlined, FolderOpenOutlined, PlusOutlined, SaveOutlined, SendOutlined, UploadOutlined, CodeOutlined, CloseOutlined } from "@ant-design/icons";
import { Button, Checkbox, ConfigProvider, Input, Select, Tabs, theme } from "antd";

export function mountControl(container, type, props = {}, getTheme) {
  const Component = {Button, Select, Input, Password: Input.Password, TextArea: Input.TextArea, Checkbox, Tabs}[type];
  let currentProps = props;
  let disposed = false;
  if (!Component) throw new Error(`不支持的插件控件：${type}`);
  const root = createRoot(container);
  const render = () => {
    const dark = document.documentElement.dataset.theme === "dark";
    const {label, icon, onChange, ...options} = currentProps;
    const handleChange = (valueOrEvent) => {
      let nextValue = valueOrEvent;
      if (["Input", "Password", "TextArea"].includes(type)) nextValue = valueOrEvent?.target?.value ?? "";
      else if (type === "Checkbox") nextValue = typeof valueOrEvent === "boolean" ? valueOrEvent : Boolean(valueOrEvent?.target?.checked);
      if (["Input", "Password", "TextArea", "Select"].includes(type)) currentProps = {...currentProps, value: nextValue};
      if (type === "Checkbox") currentProps = {...currentProps, checked: nextValue};
      // These controls are intentionally controlled so plugin state and the
      // visible value cannot drift apart. Update the host-side prop before
      // notifying the plugin, then let the plugin persist its own state.
      render();
      onChange?.(valueOrEvent);
    };
    const iconNode = {
      copy: <CopyOutlined />,
      delete: <DeleteOutlined />,
      load: <FolderOpenOutlined />,
      plus: <PlusOutlined />,
      save: <SaveOutlined />,
      send: <SendOutlined />,
      upload: <UploadOutlined />,
      code: <CodeOutlined />,
      close: <CloseOutlined />,
    }[icon];
    root.render(
      <ConfigProvider theme={getTheme?.(dark) ?? {algorithm: dark ? theme.darkAlgorithm : theme.defaultAlgorithm}}>
        <Component {...(type === "Select" ? {getPopupContainer: () => container} : {})} {...options} {...(onChange ? {onChange: handleChange} : {})} {...(type === "Button" && iconNode ? {icon: iconNode} : {})}>
          {["Button", "Checkbox"].includes(type) ? label : undefined}
        </Component>
      </ConfigProvider>
    );
  };
  const observer = new MutationObserver(render);
  observer.observe(document.documentElement, {attributes: true, attributeFilter: ["data-theme"]});
  render();
  const dispose = () => {
    if (disposed) return;
    disposed = true;
    observer.disconnect();
    root.unmount();
  };
  dispose.update = (nextProps) => {
    if (disposed) return;
    currentProps = {...currentProps, ...nextProps};
    render();
  };
  return dispose;
}
