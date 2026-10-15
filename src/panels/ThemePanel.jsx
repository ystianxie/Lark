import React from "react";
import { Button, Switch } from "antd";
import { CheckOutlined, DesktopOutlined } from "@ant-design/icons";
import { useTheme } from "../theme/ThemeContext";
import { DEFAULT_APPEARANCE, THEME_PRESETS } from "../theme/presets";
import "./themePanel.css";
import WallpaperEditor from "./WallpaperEditor";

export default function ThemePanel() {
  const { appearance, resolvedTheme, systemDark, changeTheme, changeAppearance, ready, saving, error } = useTheme();
  const followsSystem = appearance.theme === "system";
  const disabled = !ready || saving;

  return (
    <div className="themePanel">
      <header className="themePanelHeader">
        <div><h2>让 Lark 更合你的眼</h2><p>选择喜欢的外观，搜索与工具面板一同切换。</p></div>
        <span className="themeCurrentBadge">{THEME_PRESETS.find(item => item.id === resolvedTheme)?.name}</span>
      </header>
      <div className="themePresetGrid" role="group" aria-label="选择皮肤">
        {THEME_PRESETS.map(preset => (
          <button type="button" key={preset.id}
                  className={`themePresetCard ${appearance.theme === preset.id ? "is-selected" : ""}`}
                  aria-pressed={appearance.theme === preset.id} disabled={disabled}
                  onClick={() => changeTheme(preset.id)}>
            <div className={`themePreview themePreview--${preset.id}`} aria-hidden="true">
              <div className="themePreviewLauncher">
                <div className="themePreviewInput"><span>⌕</span><span>搜索你的下一步</span><span>⌘</span></div>
                <div className="themePreviewResult"><i/><div><b/><small/></div><span>↵</span></div>
                <div className="themePreviewResult"><i/><div><b/><small/></div></div>
              </div>
              <div className="themePreviewPanel"><span/><div><i/><i/><i/></div></div>
            </div>
            <div className="themePresetCaption"><div><strong>{preset.name}</strong><span>{preset.description}</span></div>
              <span className="themePresetCheck">{appearance.theme === preset.id ? <CheckOutlined/> : null}</span>
            </div>
          </button>
        ))}
      </div>
      <WallpaperEditor/>
      <section className="themeSystemRow">
        <span className="themeSystemIcon"><DesktopOutlined/></span>
        <div><h3>跟随系统</h3><p>自动切换浅色与深色。系统当前为{systemDark ? "深色" : "浅色"}。</p></div>
        <Switch aria-label="跟随系统" checked={followsSystem} disabled={disabled}
                onChange={checked => changeTheme(checked ? "system" : resolvedTheme)}/>
      </section>
      <footer className="themePanelFooter">
        <span role="status" aria-live="polite">{!ready ? "正在读取外观…" : saving ? "正在保存…" : "即时生效，自动保存"}</span>
        <Button disabled={disabled || (appearance.theme === "classic" && !appearance.wallpaper.path)} onClick={() => changeAppearance(DEFAULT_APPEARANCE)}>恢复默认</Button>
      </footer>
      {error && <div className="themePanelError" role="alert">{error}</div>}
    </div>
  );
}
