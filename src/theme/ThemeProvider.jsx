import React, { useEffect, useRef, useState } from "react";
import { ConfigProvider, theme as antdTheme } from "antd";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { applyAppearance, normalizeAppearance, readCachedAppearance, resolveTheme, THEME_STORAGE_KEY } from "./presets";
import { ThemeContext } from "./ThemeContext";
import { syncNativeAppearanceEffect } from "./nativeEffect";
import "./tokens.css";

const systemQuery = window.matchMedia("(prefers-color-scheme: dark)");
// Apply the cache before React's first render. Native config is the source of truth.
applyAppearance(readCachedAppearance(), systemQuery.matches);

export default function ThemeProvider({ children }) {
  const [appearance, setAppearance] = useState(readCachedAppearance);
  const [systemDark, setSystemDark] = useState(systemQuery.matches);
  const [ready, setReady] = useState(!isTauri());
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const current = useRef(appearance);
  const busy = useRef(false);
  const resolvedTheme = resolveTheme(appearance, systemDark);

  const acceptAppearance = (value) => {
    const next = normalizeAppearance(value);
    current.current = next;
    applyAppearance(next, systemQuery.matches);
    setAppearance(next);
    try { localStorage.setItem(THEME_STORAGE_KEY, JSON.stringify(next)); } catch { /* Native config remains available. */ }
  };

  useEffect(() => {
    const changed = (event) => setSystemDark(event.matches);
    systemQuery.addEventListener("change", changed);
    return () => systemQuery.removeEventListener("change", changed);
  }, []);

  useEffect(() => { applyAppearance(appearance, systemDark); }, [appearance, systemDark]);
  useEffect(() => { syncNativeAppearanceEffect(resolvedTheme); }, [resolvedTheme]);

  useEffect(() => {
    let disposed = false;
    let off;
    const storageChanged = (event) => {
      if (event.key === THEME_STORAGE_KEY) acceptAppearance(readCachedAppearance());
    };
    window.addEventListener("storage", storageChanged);
    if (isTauri()) {
      (async () => {
        try {
          off = await listen("appearance-changed", (event) => {
            if (!disposed) acceptAppearance(event.payload);
          });
          if (disposed) { off(); return; }
          const value = await invoke("get_appearance");
          if (!disposed) acceptAppearance(value);
        } catch (cause) {
          if (!disposed) setError(`读取外观设置失败：${String(cause)}`);
        } finally {
          if (!disposed) setReady(true);
        }
      })();
    }
    return () => { disposed = true; off?.(); window.removeEventListener("storage", storageChanged); };
  }, []);

  const changeTheme = async (theme) => {
    if (busy.current || !ready) return;
    const previous = current.current;
    const next = normalizeAppearance({ theme });
    busy.current = true;
    setSaving(true);
    setError("");
    acceptAppearance(next);
    try {
      if (isTauri()) await invoke("save_appearance", { appearance: next });
    } catch (cause) {
      acceptAppearance(previous);
      setError(`保存外观失败，已恢复原皮肤：${String(cause)}`);
    } finally {
      busy.current = false;
      setSaving(false);
    }
  };

  return (
    <ThemeContext.Provider value={{ appearance, resolvedTheme, systemDark, changeTheme, ready, saving, error }}>
      <ConfigProvider theme={{
        algorithm: resolvedTheme === "dark" ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
        token: {
          fontFamily: "var(--font-family-ui)",
          colorPrimary: resolvedTheme === "dark" ? "#7aa7ff" : "#4f7df3",
          colorBgContainer: resolvedTheme === "dark" ? "#252a33" : "#ffffff",
          colorBgElevated: resolvedTheme === "dark" ? "#2c323d" : "#ffffff",
          colorText: resolvedTheme === "dark" ? "#e3e8f0" : "#273142",
          colorTextSecondary: resolvedTheme === "dark" ? "#a8b2c1" : "#667085",
          colorTextLightSolid: resolvedTheme === "dark" ? "#172238" : "#ffffff",
          colorBorder: resolvedTheme === "dark" ? "#414a59" : "#dce1e9",
        },
      }}>{children}</ConfigProvider>
    </ThemeContext.Provider>
  );
}
