import { theme } from "antd";
import { mountControl } from "../packages/plugin-sdk/src/controlRenderer.jsx";

// SDK 的本地预览也使用同一份控件实现；宿主在这里提供 Lark 的主题颜色。
export function mountPluginControl(container, type, props = {}) {
  return mountControl(container, type, props, (dark) => ({
    algorithm: dark ? theme.darkAlgorithm : theme.defaultAlgorithm,
    token: {
      fontSize: 13,
      fontFamily: "var(--font-family-ui)",
      colorPrimary: dark ? "#7aa7ff" : "#4f7df3",
      colorBgContainer: "var(--panel-surface)",
      colorBgElevated: "var(--panel-surface)",
      colorFillSecondary: "var(--surface-muted)",
      colorFillTertiary: "var(--surface-muted)",
      colorBorder: "var(--launcher-border)",
      colorBorderSecondary: "var(--launcher-chip-border)",
      colorText: "var(--text-primary)",
      colorTextSecondary: "var(--text-secondary)",
      colorTextTertiary: "var(--text-muted)",
      colorTextPlaceholder: "var(--text-muted)",
    },
  }));
}
