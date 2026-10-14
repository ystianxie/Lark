import React, {useEffect, useLayoutEffect, useRef} from "react";
import {isWindows} from "react-device-detect";
import {getAppLocationPath} from "./utils/appLocation";
import "./ResultContextMenu.css";

export default function ResultContextMenu({menu, onClose, onAction}) {
    const ref = useRef(null);
    // Keep the action visible for every Windows app. Some AppsFolder/UWP
    // entries cannot be elevated, but the launcher can fall back to normal mode.
    const canRunAsAdmin = isWindows;
    const hasLocation = Boolean(getAppLocationPath(menu.item));
    useLayoutEffect(() => {
        const element = ref.current;
        const rect = element.getBoundingClientRect();
        element.style.left = `${Math.max(4, Math.min(menu.x, window.innerWidth - rect.width - 4))}px`;
        element.style.top = `${Math.max(4, Math.min(menu.y, window.innerHeight - rect.height - 4))}px`;
        element.querySelector("button")?.focus();
    }, [menu]);
    useEffect(() => {
        const dismissOutside = (event) => {
            if (!ref.current?.contains(event.target)) onClose();
        };
        const handleKey = (event) => {
            if (event.key === "Escape" || event.key === "Tab") {
                event.preventDefault();
                event.stopPropagation();
                onClose();
            } else if (["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
                event.preventDefault();
                event.stopPropagation();
                const buttons = [...ref.current.querySelectorAll("button")];
                const index = buttons.indexOf(document.activeElement);
                const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1
                    : (index + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
                buttons[next]?.focus();
            }
        };
        document.addEventListener("pointerdown", dismissOutside, true);
        document.addEventListener("keydown", handleKey, true);
        document.addEventListener("scroll", onClose, true);
        window.addEventListener("blur", onClose);
        return () => {
            document.removeEventListener("pointerdown", dismissOutside, true);
            document.removeEventListener("keydown", handleKey, true);
            document.removeEventListener("scroll", onClose, true);
            window.removeEventListener("blur", onClose);
            document.querySelector("#mainInput")?.focus();
        };
    }, [onClose]);
    return <div ref={ref} className="result-context-menu" role="menu" aria-label="应用操作"
        style={{left: menu.x, top: menu.y}} onContextMenu={(event) => event.preventDefault()}>
        {canRunAsAdmin && <button role="menuitem" onClick={() => onAction("admin")}>以管理员身份运行</button>}
        {hasLocation && <button role="menuitem" onClick={() => onAction("folder")}>打开所在文件夹</button>}
        {!canRunAsAdmin && !hasLocation && <span>此应用没有可用操作</span>}
    </div>;
}
