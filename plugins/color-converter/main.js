const clamp = (value, max = 1) => Math.max(0, Math.min(max, value));

export function hsvToRgb(h, s, v) {
  const c = v * s;
  const x = c * (1 - Math.abs((h / 60) % 2 - 1));
  const m = v - c;
  const channels = h < 60 ? [c, x, 0] : h < 120 ? [x, c, 0] : h < 180 ? [0, c, x] : h < 240 ? [0, x, c] : h < 300 ? [x, 0, c] : [c, 0, x];
  return channels.map(value => Math.round((value + m) * 255));
}

function rgbToHsv([r, g, b]) {
  r /= 255; g /= 255; b /= 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b), delta = max - min;
  let hue = 0;
  if (delta) hue = 60 * (max === r ? ((g - b) / delta) % 6 : max === g ? (b - r) / delta + 2 : (r - g) / delta + 4);
  return [(hue + 360) % 360, max ? delta / max : 0, max];
}

function parseColor(text) {
  const hex = text.trim().match(/^#?([a-f\d]{3}|[a-f\d]{6})$/i);
  if (hex) {
    const value = hex[1].length === 3 ? [...hex[1]].map(c => c + c).join("") : hex[1];
    return rgbToHsv([0, 2, 4].map(i => parseInt(value.slice(i, i + 2), 16)));
  }
  const rgb = text.trim().match(/^rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)$/i);
  if (rgb && rgb.slice(1).every(value => Number(value) <= 255)) return rgbToHsv(rgb.slice(1).map(Number));
  const hsl = text.trim().match(/^hsl\(\s*([\d.]+)\s*,\s*([\d.]+)%\s*,\s*([\d.]+)%\s*\)$/i);
  if (hsl && Number(hsl[2]) <= 100 && Number(hsl[3]) <= 100) {
    const h = Number(hsl[1]) % 360, s = Number(hsl[2]) / 100, l = Number(hsl[3]) / 100;
    const v = l + s * Math.min(l, 1 - l);
    return [h, v ? 2 * (1 - l / v) : 0, v];
  }
  throw new Error("支持 #RGB、#RRGGBB、rgb(255, 0, 0)、hsl(0, 100%, 50%)");
}

export function activate(context) {
  let cleanup;
  return {
    mount(container) {
      cleanup?.();
      let hue = 215, saturation = 0.91, value = 1;
      const controlCleanups = [];
      const root = document.createElement("div");
      root.style.cssText = "box-sizing:border-box;padding:16px;display:flex;flex-direction:column;gap:12px;height:100%;overflow:auto;font-family:inherit;font-size:13px;line-height:1.5";
      const heading = document.createElement("strong");
      heading.textContent = "颜色选择与转换";
      const body = document.createElement("div");
      body.style.cssText = "display:flex;gap:24px;align-items:center;flex-wrap:wrap";
      const wheel = document.createElement("div");
      wheel.style.cssText = "position:relative;width:220px;height:220px;flex:none";
      const ring = document.createElement("div");
      ring.style.cssText = "position:absolute;inset:0;border-radius:50%;touch-action:none;cursor:crosshair";
      // Draw the ring directly: radial-gradient percentages in a CSS mask
      // resolve against the corner radius and can hide the entire circumference.
      const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
      svg.setAttribute("viewBox", "0 0 220 220");
      svg.style.cssText = "display:block;width:220px;height:220px;pointer-events:none";
      for (let angle = 0; angle < 360; angle += 2) {
        const start = angle * Math.PI / 180;
        const end = (angle + 2.2) * Math.PI / 180;
        const arc = document.createElementNS("http://www.w3.org/2000/svg", "path");
        arc.setAttribute("d", `M ${110 + 98 * Math.cos(start)} ${110 + 98 * Math.sin(start)} A 98 98 0 0 1 ${110 + 98 * Math.cos(end)} ${110 + 98 * Math.sin(end)}`);
        arc.setAttribute("stroke", `hsl(${angle},100%,50%)`);
        arc.setAttribute("stroke-width", "22");
        arc.setAttribute("fill", "none");
        svg.append(arc);
      }
      ring.append(svg);
      ring.setAttribute("aria-label", "色相圆环");
      const hueDot = document.createElement("div");
      hueDot.style.cssText = "position:absolute;width:12px;height:12px;box-sizing:border-box;border:2px solid white;border-radius:50%;box-shadow:0 0 0 1px #0008;pointer-events:none;transform:translate(-50%,-50%)";
      const square = document.createElement("div");
      square.style.cssText = "position:absolute;left:45px;top:45px;width:130px;height:130px;border-radius:4px;touch-action:none;cursor:crosshair";
      square.setAttribute("aria-label", "饱和度和明度");
      const squareDot = hueDot.cloneNode();
      square.append(squareDot);
      wheel.append(ring, hueDot, square);
      const info = document.createElement("div");
      info.style.cssText = "flex:1;min-width:230px;display:flex;flex-direction:column;gap:10px";
      const swatch = document.createElement("div");
      swatch.style.cssText = "height:44px;border-radius:8px;border:1px solid #8884";
      const input = document.createElement("input");
      input.placeholder = "输入 HEX、RGB 或 HSL";
      input.setAttribute("aria-label", "颜色值");
      input.style.cssText = "padding:8px;border:1px solid #8886;border-radius:6px;background:transparent;color:inherit;font-size:13px";
      const message = document.createElement("div");
      message.style.cssText = "font-size:12px;opacity:.75";
      const results = document.createElement("div");
      results.style.cssText = "display:flex;flex-direction:column;gap:6px";
      const fields = {};
      for (const name of ["HEX", "RGB", "HSL"]) {
        const row = document.createElement("div");
        row.style.cssText = "display:flex;gap:8px;align-items:center";
        const field = document.createElement("input");
        field.readOnly = true;
        field.setAttribute("aria-label", name);
        field.style.cssText = "flex:1;min-width:0;background:transparent;color:inherit;border:1px solid #8884;border-radius:5px;padding:6px;font:12px/1.5 Consolas,monospace";
        const copy = document.createElement("span");
        const copyValue = async () => {
          try { await context.api.writeClipboard(field.value); message.textContent = `${name} 已复制`; }
          catch (error) { message.textContent = `复制失败：${error.message || error}`; }
        };
        controlCleanups.push(context.ui.mountControl(copy, "Button", {
          label: `复制 ${name}`, size: "small", onClick: copyValue,
        }));
        row.append(field, copy);
        results.append(row);
        fields[name] = field;
      }
      info.append(swatch, input, results, message);
      body.append(wheel, info);
      const sliders = document.createElement("div");
      sliders.style.cssText = "display:grid;grid-template-columns:80px 1fr 48px;gap:8px;align-items:center";
      const controls = {};
      for (const [key, label, max] of [["h", "色相", 359], ["s", "饱和度", 100], ["v", "明度", 100]]) {
        const title = document.createElement("span"); title.textContent = label;
        const range = document.createElement("input"); range.type = "range"; range.min = "0"; range.max = String(max);
        range.setAttribute("aria-label", label);
        const number = document.createElement("span");
        range.oninput = () => {
          if (key === "h") hue = Number(range.value);
          if (key === "s") saturation = Number(range.value) / 100;
          if (key === "v") value = Number(range.value) / 100;
          render();
        };
        sliders.append(title, range, number);
        controls[key] = {range, number};
      }
      const render = (replaceInput = true) => {
        const rgb = hsvToRgb(hue, saturation, value);
        const hex = `#${rgb.map(channel => channel.toString(16).padStart(2, "0")).join("")}`;
        const light = value * (1 - saturation / 2);
        const hslSaturation = light === 0 || light === 1 ? 0 : (value - light) / Math.min(light, 1 - light);
        fields.HEX.value = hex;
        fields.RGB.value = `rgb(${rgb.join(", ")})`;
        fields.HSL.value = `hsl(${Math.round(hue)}, ${Math.round(hslSaturation * 100)}%, ${Math.round(light * 100)}%)`;
        if (replaceInput) input.value = hex;
        swatch.style.background = hex;
        square.style.background = `linear-gradient(to top,#000,transparent),linear-gradient(to right,#fff,transparent),hsl(${hue},100%,50%)`;
        hueDot.style.left = `${110 + Math.cos(hue * Math.PI / 180) * 98}px`;
        hueDot.style.top = `${110 + Math.sin(hue * Math.PI / 180) * 98}px`;
        squareDot.style.left = `${saturation * 100}%`;
        squareDot.style.top = `${(1 - value) * 100}%`;
        for (const [key, current] of [["h", hue], ["s", saturation * 100], ["v", value * 100]]) {
          controls[key].range.value = String(Math.round(current));
          controls[key].number.textContent = `${Math.round(current)}${key === "h" ? "°" : "%"}`;
        }
        message.textContent = "拖动色相环或中间色块，也可使用下方滑杆";
      };
      const drag = (element, onMove) => {
        element.onpointerdown = event => { element.setPointerCapture(event.pointerId); onMove(event); };
        element.onpointermove = event => { if (element.hasPointerCapture(event.pointerId)) onMove(event); };
        element.onpointerup = event => { if (element.hasPointerCapture(event.pointerId)) element.releasePointerCapture(event.pointerId); };
      };
      drag(ring, event => {
        const box = ring.getBoundingClientRect();
        hue = (Math.atan2(event.clientY - box.top - box.height / 2, event.clientX - box.left - box.width / 2) * 180 / Math.PI + 360) % 360;
        render();
      });
      drag(square, event => {
        const box = square.getBoundingClientRect();
        saturation = clamp((event.clientX - box.left) / box.width);
        value = 1 - clamp((event.clientY - box.top) / box.height);
        render();
      });
      input.oninput = () => {
        try { [hue, saturation, value] = parseColor(input.value); render(false); }
        catch (error) { message.textContent = error.message; }
      };
      root.append(heading, body, sliders);
      container.replaceChildren(root);
      render();
      cleanup = () => { controlCleanups.forEach(unmount => unmount()); root.remove(); };
    },
    unmount() { cleanup?.(); cleanup = null; },
  };
}
