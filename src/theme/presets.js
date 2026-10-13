export const THEME_STORAGE_KEY = "lark.appearance";
export const DEFAULT_APPEARANCE = Object.freeze({ theme: "classic" });

export const THEME_PRESETS = [
  { id: "classic", name: "经典", description: "深色搜索，浅色面板" },
  { id: "light", name: "浅色", description: "清爽明亮，轻盈专注" },
  { id: "dark", name: "深色", description: "柔和暗色，减少眩光" },
  { id: "frosted", name: "磨砂", description: "朦胧桌面，柔和透光" },
];

export function normalizeAppearance(value) {
  return { theme: [...THEME_PRESETS.map(preset => preset.id), "system"].includes(value?.theme) ? value.theme : "classic" };
}

export function resolveTheme(appearance, systemDark) {
  return appearance.theme === "system" ? (systemDark ? "dark" : "light") : appearance.theme;
}

export function readCachedAppearance() {
  try {
    return normalizeAppearance(JSON.parse(localStorage.getItem(THEME_STORAGE_KEY)));
  } catch {
    return { ...DEFAULT_APPEARANCE };
  }
}

export function applyAppearance(appearance, systemDark) {
  const resolved = resolveTheme(appearance, systemDark);
  document.documentElement.dataset.theme = resolved;
  document.documentElement.style.colorScheme = resolved === "dark" ? "dark" : "light";
  return resolved;
}
