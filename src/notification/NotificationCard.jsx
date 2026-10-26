import {useEffect, useRef, useState} from "react";
import {convertFileSrc, invoke} from "@tauri-apps/api/core";

export default function NotificationCard({
                                             notification,
                                             index,
                                             error,
                                             exiting,
                                             paused,
                                             busyAction,
                                             onDismiss,
                                             onAction,
                                             onErrorClear,
                                             onActionsOpenChange
                                         }) {
    const [expandedMessage, setExpandedMessage] = useState(false);
    const [actionsOpen, setActionsOpen] = useState(false);
    const [hovered, setHovered] = useState(false);
    const remaining = useRef(null);
    const expired = useRef(false);
    const cardRef = useRef(null);
    const drag = useRef(null);
    const suppressClick = useRef(false);
    const [swipeDismiss, setSwipeDismiss] = useState(false);
    const dragging = useRef(false);

    useEffect(() => {
        remaining.current = notification.durationMs == null ? null : Math.max(0, notification.durationMs - (Date.now() - notification.createdAtMs));
        expired.current = false;
    }, [notification.id, notification.createdAtMs, notification.durationMs]);

    useEffect(() => {
        if (remaining.current == null) return undefined;
        const timer = setInterval(() => {
            if (hovered || paused || remaining.current == null) return;
            remaining.current = Math.max(0, remaining.current - 100);
            if (remaining.current <= 0 && !expired.current) {
                expired.current = true;
                onDismiss(notification.id);
            }
        }, 100);
        return () => clearInterval(timer);
    }, [notification.id, notification.durationMs, hovered, paused, onDismiss]);

    const onPointerDown = (event) => {
        if (event.button !== 0 || event.target.closest("button, a, input, textarea, select")) return;
        drag.current = {
            pointerId: event.pointerId,
            startX: event.clientX,
            startY: event.clientY,
            horizontal: null,
            moved: false
        };
        event.currentTarget.style.transition = "none";
        event.currentTarget.setPointerCapture(event.pointerId);
    };

    const onPointerMove = (event) => {
        const current = drag.current;
        if (!current || current.pointerId !== event.pointerId) return;
        const dx = event.clientX - current.startX;
        const dy = event.clientY - current.startY;
        if (current.horizontal === null && (Math.abs(dx) > 6 || Math.abs(dy) > 6)) {
            current.horizontal = Math.abs(dx) > Math.abs(dy);
        }
        if (!current.horizontal) return;
        current.moved = true;
        if (!dragging.current) {
            dragging.current = true;
            cardRef.current?.classList.add("notification-card--dragging");
        }
        suppressClick.current = true;
        if (cardRef.current) cardRef.current.style.translate = `${Math.max(0, dx)}px 0`;
    };

    const onPointerUp = (event) => {
        const current = drag.current;
        if (!current || current.pointerId !== event.pointerId) return;
        const shouldDismiss = current.horizontal && event.clientX - current.startX > Math.max(90, event.currentTarget.clientWidth * 0.3);
        drag.current = null;
        dragging.current = false;
        cardRef.current?.classList.remove("notification-card--dragging");
        if (shouldDismiss) {
            cardRef.current?.style.setProperty("--swipe-start-x", `${Math.max(0, event.clientX - current.startX)}px`);
            setSwipeDismiss(true);
            onDismiss(notification.id);
            return;
        }
        if (cardRef.current) {
            cardRef.current.style.transition = "translate 180ms cubic-bezier(.2,.8,.2,1)";
            cardRef.current.style.translate = "0 0";
            window.setTimeout(() => {
                if (cardRef.current) {
                    cardRef.current.style.transition = "";
                    cardRef.current.style.translate = "";
                }
            }, 180);
        }
    };

    const cancelPointer = (event) => {
        if (drag.current?.pointerId === event.pointerId) {
            drag.current = null;
            dragging.current = false;
            cardRef.current?.classList.remove("notification-card--dragging");
            if (cardRef.current) {
                cardRef.current.style.transition = "translate 180ms cubic-bezier(.2,.8,.2,1)";
                cardRef.current.style.translate = "0 0";
            }
        }
    };

    const setActionMenuOpen = async (open) => {
        const update = onActionsOpenChange?.(notification.id, open, notification.actions?.length || 0);
        if (open && update) await update;
        setActionsOpen(open);
    };

    const setHover = (value) => {
        setHovered(value);
        invoke("set_notification_hovered", {
            id: notification.id,
            hovered: value
        }).catch((error) => console.warn("暂停通知倒计时失败", error));
    };

    return <article
        ref={cardRef}
        className={`notification-card notification-card--${notification.level}${error ? " notification-card--has-error" : ""}${actionsOpen ? " notification-card--actions-open" : ""}${exiting ? " notification-card--exiting" : ""}${swipeDismiss ? " notification-card--swipe-exiting" : ""}`}
        style={{"--card-index": index}}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={cancelPointer}
        onLostPointerCapture={cancelPointer}
        onClick={(event) => {
            if (suppressClick.current) {
                event.preventDefault();
                event.stopPropagation();
                suppressClick.current = false;
            } else event.stopPropagation();
        }}
        onMouseEnter={() => setHover(true)}
        onMouseLeave={() => setHover(false)}
    >
        <div className="notification-card__icon" aria-hidden="true">{notification.sourceIcon ?
            <img src={convertFileSrc(notification.sourceIcon)} alt="" onError={(event) => {
                event.currentTarget.style.display = "none";
            }}/> : <span>✦</span>}</div>
        <div className="notification-card__content">
            <strong className="notification-card__title" title={notification.title}>{notification.title}</strong>
            {notification.message ? <div
                className={expandedMessage ? "notification-card__message notification-card__message--expanded" : "notification-card__message"}>
                <p>{notification.message}</p>
                {notification.message.length > 120 && <button className="notification-card__more"
                                                              onClick={() => setExpandedMessage((value) => !value)}>{expandedMessage ? "收起" : "展开"}</button>}
            </div> : null}
        </div>
        <button className="notification-card__close" aria-label="关闭通知"
                onClick={() => onDismiss(notification.id)}>×
        </button>
        {error && <div className="notification-card__error" role="alert">{error}
            <button aria-label="关闭错误提示" onClick={onErrorClear}>×</button>
        </div>}
        {notification.actions?.length ? <footer
            className={`notification-card__actions${notification.actions.length > 1 ? " notification-card__actions--menu" : ""}`}>
            {notification.actions.length === 1 ? (
                <button type="button" disabled={busyAction === 0} onClick={() => onAction(notification.id, 0)}>
                    {busyAction === 0 ? "处理中…" : notification.actions[0].label || actionLabel(notification.actions[0])}
                </button>
            ) : (
                <div className="notification-card__action-menu">
                    <button type="button" aria-haspopup="menu" aria-expanded={actionsOpen}
                            onClick={() => setActionMenuOpen(!actionsOpen)}>选项↓
                    </button>
                    {actionsOpen ? <div className="notification-card__action-popover" role="menu">
                        {notification.actions.map((action, actionIndex) => <button type="button" role="menuitem"
                                                                                   key={`${action.type}-${actionIndex}`}
                                                                                   disabled={busyAction === actionIndex}
                                                                                   onClick={() => {
                                                                                       setActionMenuOpen(false);
                                                                                       onAction(notification.id, actionIndex);
                                                                                   }}>{busyAction === actionIndex ? "处理中…" : action.label || actionLabel(action)}</button>)}
                    </div> : null}
                </div>
            )}
        </footer> : null}
    </article>;
}

function actionLabel(action) {
    if (action.type === "open") return "打开";
    if (action.type === "open_dir") return "打开目录";
    return "关闭";
}
