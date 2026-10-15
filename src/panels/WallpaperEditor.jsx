import React, { useEffect, useRef, useState } from "react";
import { Button, Slider, Switch } from "antd";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useTheme } from "../theme/ThemeContext";
import { DEFAULT_WALLPAPER } from "../theme/presets";
import { wallpaperStyle } from "../theme/Wallpaper";

export default function WallpaperEditor() {
  const { appearance, changeAppearance, ready, saving } = useTheme();
  const [draft, setDraft] = useState(appearance.wallpaper);
  const [choosing, setChoosing] = useState(false);
  const [error, setError] = useState("");
  const drag = useRef(null);
  const draftRef = useRef(draft);
  const disabled = !ready || saving || choosing;
  useEffect(() => { setDraft(appearance.wallpaper); draftRef.current = appearance.wallpaper; }, [appearance.wallpaper]);
  const update = (patch) => {
    const next = { ...draftRef.current, ...patch };
    draftRef.current = next;
    setDraft(next);
    return next;
  };
  const save = (value = draftRef.current) => changeAppearance({ wallpaper: value });
  const choose = async () => {
    setChoosing(true);
    setError("");
    try {
      const path = await open({ multiple: false, directory: false, filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "webp", "bmp"] }] });
      if (!path) return;
      const imported = await invoke("import_wallpaper", { path });
      await save({ ...DEFAULT_WALLPAPER, path: imported, enabled: true });
    } catch (cause) { setError(`无法选择壁纸：${String(cause)}`); }
    finally { setChoosing(false); }
  };
  const finishDrag = (event) => {
    if (!drag.current || drag.current.id !== event.pointerId) return;
    drag.current = null;
    save();
  };
  return <section className="wallpaperEditor">
    <div className="wallpaperHeader">
      <div><h3>自定义壁纸</h3><p>选一张本地图片，拖动预览调整位置。壁纸叠加在当前皮肤上。</p></div>
      <Switch aria-label="启用自定义壁纸" checked={draft.enabled && !!draft.path} disabled={disabled || !draft.path}
        onChange={enabled => save(update({ enabled }))}/>
    </div>
    <div className="wallpaperLayout">
      <div className={`wallpaperPreview ${draft.path ? "has-image" : ""}`} role="group" aria-label="壁纸预览，可拖动调整位置"
        onPointerDown={event => {
          if (disabled || !draft.path || event.button !== 0) return;
          event.preventDefault();
          event.currentTarget.setPointerCapture(event.pointerId);
          const rect = event.currentTarget.getBoundingClientRect();
          drag.current = { id: event.pointerId, px: event.clientX, py: event.clientY, x: draft.x, y: draft.y, width: rect.width, height: rect.height };
        }}
        onPointerMove={event => {
          const start = drag.current;
          if (!start || start.id !== event.pointerId) return;
          const factor = Math.max(.5, draftRef.current.scale / 100 - 1);
          update({ x: Math.round(Math.max(0, Math.min(100, start.x - (event.clientX - start.px) / start.width / factor * 100))),
            y: Math.round(Math.max(0, Math.min(100, start.y - (event.clientY - start.py) / start.height / factor * 100))) });
        }} onPointerUp={finishDrag} onPointerCancel={finishDrag} onLostPointerCapture={finishDrag}>
        {draft.path ? <div className="wallpaperImage" style={wallpaperStyle(draft)}/> : <span>选择图片后在这里预览</span>}
        {draft.path && <div className="wallpaperPreviewContent"><span>⌕ 搜索你的下一步</span><small>拖动图片调整位置</small></div>}
      </div>
      <div className="wallpaperControls">
        <div className="wallpaperActions"><Button onClick={choose} disabled={disabled} loading={choosing}>{draft.path ? "更换图片" : "选择本地图片"}</Button>
          <Button disabled={disabled || !draft.path} onClick={() => save({ ...DEFAULT_WALLPAPER })}>移除</Button></div>
        {[["scale", "缩放", 100, 300, "%"], ["opacity", "不透明度", 0, 100, "%"], ["x", "水平位置", 0, 100, "%"], ["y", "垂直位置", 0, 100, "%"]].map(([key, label, min, max, unit]) =>
          <div key={key}><label id={`wallpaper-${key}`}>{label}<span>{draft[key]}{unit}</span></label>
            <Slider aria-labelledby={`wallpaper-${key}`} min={min} max={max} value={draft[key]} disabled={disabled || !draft.path}
              onChange={value => update({ [key]: value })} onChangeComplete={() => save()}/></div>)}
        <Button size="small" disabled={disabled || !draft.path} onClick={() => save(update({ scale: 100, x: 50, y: 50, opacity: 35 }))}>重置调整</Button>
      </div>
    </div>
    <p className="wallpaperHint">支持 PNG、JPG、WebP、BMP，最大 20 MB。图片自动保存到应用目录；不透明度越低，背景越淡。</p>
    {error && <div className="themePanelError" role="alert">{error}</div>}
  </section>;
}
