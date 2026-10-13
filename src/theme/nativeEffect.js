import { invoke, isTauri } from "@tauri-apps/api/core";

let selectedTheme = "classic";
let suspended = false;
let pending = Promise.resolve();
let revision = 0;
let nativeActive = false;
let boundsObserver;
let boundsFrame;
let lastBounds = "";

function readLauncherBounds() {
  const launcher = document.getElementById("mainDiv");
  if (!launcher) return null;
  const rect = launcher.getBoundingClientRect();
  if (!rect.width || !rect.height) return null;
  return {
    x: rect.x,
    y: rect.y,
    width: rect.width,
    height: rect.height,
    radius: parseFloat(getComputedStyle(launcher).borderTopLeftRadius) || 0,
    scale: window.devicePixelRatio || 1,
  };
}

function stopWatchingBounds() {
  boundsObserver?.disconnect();
  boundsObserver = undefined;
  window.removeEventListener("resize", scheduleBoundsUpdate);
  cancelAnimationFrame(boundsFrame);
  boundsFrame = undefined;
}

function scheduleBoundsUpdate() {
  if (!nativeActive || boundsFrame !== undefined) return;
  boundsFrame = requestAnimationFrame(() => {
    boundsFrame = undefined;
    const request = revision;
    pending = pending.catch(() => {}).then(async () => {
      if (!nativeActive || request !== revision) return;
      const bounds = readLauncherBounds();
      const key = JSON.stringify(bounds);
      if (!bounds || key === lastBounds) return;
      try {
        await invoke("update_frosted_bounds", { bounds });
        lastBounds = key;
      } catch (error) {
        if (request !== revision) return;
        nativeActive = false;
        stopWatchingBounds();
        document.documentElement.dataset.nativeBlur = "inactive";
        await invoke("set_frosted_effect", { enabled: false, bounds: null }).catch(() => {});
        console.warn("更新磨砂窗口轮廓失败，使用实色背景", error);
      }
    });
  });
}

function watchLauncherBounds() {
  const launcher = document.getElementById("mainDiv");
  if (!launcher) return;
  boundsObserver = new ResizeObserver(scheduleBoundsUpdate);
  boundsObserver.observe(launcher);
  // Native resizing and changes to the monitor's DPI also need a fresh clip.
  window.addEventListener("resize", scheduleBoundsUpdate);
  scheduleBoundsUpdate();
}

function applyNativeEffect() {
  if (!isTauri()) return Promise.resolve();
  const enabled = selectedTheme === "frosted" && !suspended;
  const request = ++revision;
  nativeActive = false;
  stopWatchingBounds();
  // Serialize updates so a slower enable cannot overwrite a later disable.
  pending = pending.catch(() => {}).then(async () => {
    if (request !== revision) return;
    try {
      const bounds = enabled ? readLauncherBounds() : null;
      const supported = await invoke("set_frosted_effect", { enabled, bounds });
      if (request === revision) {
        nativeActive = enabled && supported;
        document.documentElement.dataset.nativeBlur = nativeActive ? "active" : "inactive";
        lastBounds = JSON.stringify(bounds);
        if (nativeActive) watchLauncherBounds();
      }
    } catch (error) {
      if (request === revision) {
        document.documentElement.dataset.nativeBlur = "inactive";
        // Clear any previous clip as well when enabling the new surface fails.
        await invoke("set_frosted_effect", { enabled: false, bounds: null }).catch(() => {});
      }
      console.warn("更新窗口磨砂效果失败，使用实色背景", error);
    }
  });
  return pending;
}

export function syncNativeAppearanceEffect(theme) {
  selectedTheme = theme;
  return applyNativeEffect();
}

export function suspendNativeAppearanceEffect(value) {
  suspended = value;
  return applyNativeEffect();
}

if (import.meta.hot) import.meta.hot.dispose(() => {
  nativeActive = false;
  revision += 1;
  stopWatchingBounds();
});
