import { invoke, isTauri } from "@tauri-apps/api/core";

let selectedTheme = "classic";
let suspended = false;
let pending = Promise.resolve();
let revision = 0;

function applyNativeEffect() {
  if (!isTauri()) return Promise.resolve();
  const enabled = selectedTheme === "frosted" && !suspended;
  const request = ++revision;
  // Serialize updates so a slower enable cannot overwrite a later disable.
  pending = pending.catch(() => {}).then(async () => {
    if (request !== revision) return;
    try {
      const supported = await invoke("set_frosted_effect", { enabled });
      if (request === revision) {
        const nativeActive = enabled && supported;
        document.documentElement.dataset.nativeBlur = nativeActive ? "active" : "inactive";
      }
    } catch (error) {
      if (request === revision) {
        document.documentElement.dataset.nativeBlur = "inactive";
        // Clear the accent policy when enabling the native surface fails.
        await invoke("set_frosted_effect", { enabled: false }).catch(() => {});
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
  revision += 1;
});
