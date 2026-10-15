import { useEffect } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";

export const wallpaperUrl = (path) => path ? convertFileSrc(path) : "";

export function wallpaperStyle(wallpaper) {
  return {
    backgroundImage: `url(${JSON.stringify(wallpaperUrl(wallpaper.path))})`,
    backgroundPosition: `${wallpaper.x}% ${wallpaper.y}%`,
    transform: `scale(${wallpaper.scale / 100})`,
    transformOrigin: `${wallpaper.x}% ${wallpaper.y}%`,
    opacity: wallpaper.opacity / 100,
  };
}

export default function Wallpaper({ wallpaper }) {
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.wallpaper = wallpaper.enabled && wallpaper.path ? "on" : "off";
    const style = wallpaperStyle(wallpaper);
    for (const [key, value] of Object.entries(style)) {
      root.style.setProperty(`--wallpaper-${key.replace(/[A-Z]/g, letter => `-${letter.toLowerCase()}`)}`, value);
    }
    return () => { delete root.dataset.wallpaper; };
  }, [wallpaper]);
  return null;
}
