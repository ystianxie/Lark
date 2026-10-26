import {useCallback, useEffect, useMemo, useRef, useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";
import {getCurrentWindow} from "@tauri-apps/api/window";
import NotificationCard from "./NotificationCard";
import "./notification.css";

const MAX_EXPANDED_CARDS = 3;
const ACTION_MENU_ITEM_HEIGHT = 35;
const ACTION_MENU_MARGIN = 6;
const ACTION_MENU_PADDING = 12;
const ACTION_MENU_GAP = 2;

function actionMenuHeight(actionCount) {
    return ACTION_MENU_MARGIN
        + ACTION_MENU_PADDING
        + (ACTION_MENU_ITEM_HEIGHT * Math.max(0, actionCount - 1))
        + (ACTION_MENU_GAP * Math.max(0, actionCount - 1));
}

export default function NotificationWindow() {
    const [state, setState] = useState({notifications: [], enabled: true, paused: false});
    const [expanded, setExpanded] = useState(false);
    const [errors, setErrors] = useState({});
    const [pendingActions, setPendingActions] = useState({});
    const [openActionMenus, setOpenActionMenus] = useState({});
    const [exitingIds, setExitingIds] = useState(() => new Set());
    const resizeTimer = useRef(null);
    const notifications = useMemo(() => state.notifications || [], [state.notifications]);

    const sync = useCallback(async () => {
        try {
            const snapshot = await invoke("get_notifications");
            setState(snapshot);
        } catch (error) {
            console.error("读取通知状态失败", error);
        }
    }, []);

    useEffect(() => {
        let unlisten;
        let disposed = false;
        listen("notification-state", (event) => {
            if (!disposed) setState(event.payload);
        }).then((off) => {
            unlisten = off;
            if (disposed) off();
            else sync();
        });
        return () => {
            disposed = true;
            unlisten?.();
        };
    }, [sync]);

    useEffect(() => {
        setErrors((previous) => Object.fromEntries(Object.entries(previous).filter(([id]) => notifications.some((item) => item.id === id))));
        setPendingActions((previous) => Object.fromEntries(Object.entries(previous).filter(([id]) => notifications.some((item) => item.id === id))));
        if (!notifications.length) setExpanded(false);
    }, [notifications]);

    useEffect(() => {
        const window = getCurrentWindow();
        const visible = state.enabled && !state.paused && notifications.length > 0;
        (visible ? window.show() : window.hide()).catch((error) => console.warn("切换通知窗口可见性失败", error));
    }, [state.enabled, state.paused, notifications.length]);

    useEffect(() => {
        const root = document.getElementById("notification-stack");
        if (!root) return;
        const workHeight = window.screen.availHeight || window.screen.height;
        const workWidth = window.screen.availWidth || window.screen.width;
        root.style.setProperty("--notification-collapsed-limit", `${Math.min(520, workHeight * 0.4)}px`);
        root.style.setProperty("--notification-expanded-limit", `${Math.min(800, workHeight * 0.7)}px`);
        root.style.setProperty("--notification-width", `${Math.min(380, Math.max(280, workWidth - 32))}px`);
    }, []);

    // Expanded view is preview-only; only the top three notifications can count down.
    const collapsedIds = notifications.slice(MAX_EXPANDED_CARDS).map((item) => item.id);
    const collapsedIdsRef = useRef(new Set());
    useEffect(() => {
        const next = new Set(collapsedIds);
        const previous = collapsedIdsRef.current;
        for (const id of new Set([...previous, ...next])) {
            const collapsed = next.has(id);
            if (previous.has(id) !== collapsed) {
                invoke("set_notification_collapsed", {
                    id,
                    collapsed
                }).catch((error) => console.warn("同步折叠通知状态失败", error));
            }
        }
        collapsedIdsRef.current = next;
    }, [collapsedIds.join("\u0000")]);
    const actionMenuExtraHeight = useMemo(() => notifications.reduce((total, item) => {
        if (!openActionMenus[item.id] || !item.actions || item.actions.length <= 1) return total;
        return total + actionMenuHeight(item.actions.length);
    }, 0), [notifications, openActionMenus]);

    useEffect(() => {
        const element = document.getElementById("notification-stack");
        if (!element || !state.enabled || state.paused || !notifications.length) return undefined;
        const resize = () => {
            const {width, height} = element.getBoundingClientRect();
            clearTimeout(resizeTimer.current);
            resizeTimer.current = setTimeout(() => {
                invoke("notification_window_resize", {
                    width,
                    height
                }).catch((error) => console.warn("调整通知窗口尺寸失败", error));
            }, 60);
        };
        const observer = new ResizeObserver(resize);
        observer.observe(element);
        resize();
        return () => {
            clearTimeout(resizeTimer.current);
            observer.disconnect();
        };
    }, [state.enabled, state.paused, expanded, notifications, actionMenuExtraHeight]);

    const dismiss = useCallback(async (id) => {
        if (exitingIds.has(id)) return;
        setExitingIds((old) => new Set(old).add(id));
        await new Promise((resolve) => setTimeout(resolve, 150));
        try {
            await invoke("dismiss_notification", {id});
        } catch (error) {
            setExitingIds((old) => {
                const next = new Set(old);
                next.delete(id);
                return next;
            });
            setErrors((old) => ({...old, [id]: String(error)}));
        }
    }, [exitingIds]);

    const runAction = useCallback(async (id, actionIndex) => {
        setPendingActions((old) => ({...old, [id]: actionIndex}));
        setErrors((old) => ({...old, [id]: undefined}));
        try {
            await invoke("invoke_notification_action", {id, actionIndex});
        } catch (error) {
            setErrors((old) => ({...old, [id]: String(error)}));
        } finally {
            setPendingActions((old) => {
                const next = {...old};
                delete next[id];
                return next;
            });
        }
    }, []);

    const handleAction = useCallback((id, actionIndex) => {
        if (pendingActions[id] !== undefined) return;
        setPendingActions((old) => ({...old, [id]: actionIndex}));
        runAction(id, actionIndex);
    }, [pendingActions, runAction]);

    const handleActionsOpenChange = useCallback(async (id, open, actionCount) => {
        if (open) {
            // Resize the native window before React renders the absolute popover.
            // This avoids one frame where the WebView has the old viewport height.
            const element = document.getElementById("notification-stack");
            if (element) {
                const {width, height} = element.getBoundingClientRect();
                await invoke("notification_window_resize", {
                    width,
                    height: height + actionMenuHeight(actionCount),
                }).catch((error) => console.warn("预调整通知窗口尺寸失败", error));
            }
        }
        setOpenActionMenus((previous) => {
            const next = {...previous};
            if (open) next[id] = true;
            else delete next[id];
            return next;
        });
    }, []);

    const toggleExpanded = () => setExpanded((value) => !value);
    const displayCount = expanded ? notifications.length : Math.min(MAX_EXPANDED_CARDS, notifications.length);
    const hiddenCount = Math.max(0, notifications.length - displayCount);

    return <main id="notification-stack"
                 className={expanded ? "notification-stack notification-stack--expanded" : "notification-stack"}
                 style={{"--notification-action-space": `${actionMenuExtraHeight}px`}} aria-live="polite"
                 onClick={(event) => {
                     if (expanded && (event.target === event.currentTarget || event.target.classList.contains("notification-stack__cards"))) toggleExpanded();
                 }}>
        <div className="notification-stack__cards">
            {notifications.slice(0, displayCount).map((item, index) => <NotificationCard
                key={item.id}
                notification={item}
                index={index}
                paused={state.paused}
                busyAction={pendingActions[item.id]}
                exiting={exitingIds.has(item.id)}
                error={errors[item.id]}
                onDismiss={dismiss}
                onAction={handleAction}
                onActionsOpenChange={handleActionsOpenChange}
                onErrorClear={() => setErrors((old) => ({...old, [item.id]: undefined}))}
            />)}
            {hiddenCount > 0 && <button className="notification-stack__pile" onClick={toggleExpanded}
                                        aria-label={`展开另外 ${hiddenCount} 条通知`}>
                还有 {hiddenCount} 条通知 · 点击展开
            </button>}
        </div>
        {expanded && notifications.length > MAX_EXPANDED_CARDS &&
            <button className="notification-stack__collapse" onClick={toggleExpanded}>收起通知</button>}
    </main>;
}
