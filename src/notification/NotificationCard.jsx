import { useEffect, useRef, useState } from "react";

export default function NotificationCard({ notification, index, error, exiting, paused, busyAction, onDismiss, onAction, onErrorClear }) {
  const [expandedMessage, setExpandedMessage] = useState(false);
  const [hovered, setHovered] = useState(false);
  const remaining = useRef(null);
  const expired = useRef(false);

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

  const setHover = (value) => {
    setHovered(value);
  };

  return <article
    className={`notification-card notification-card--${notification.level}${error ? " notification-card--has-error" : ""}${exiting ? " notification-card--exiting" : ""}`}
    style={{ "--card-index": index }}
    onClick={(event) => event.stopPropagation()}
    onMouseEnter={() => setHover(true)}
    onMouseLeave={() => setHover(false)}
  >
    <div className="notification-card__content">
      <strong className="notification-card__title" title={notification.title}>{notification.title}</strong>
      {notification.message ? <div className={expandedMessage ? "notification-card__message notification-card__message--expanded" : "notification-card__message"}>
        <p>{notification.message}</p>
        {notification.message.length > 120 && <button className="notification-card__more" onClick={() => setExpandedMessage((value) => !value)}>{expandedMessage ? "收起" : "展开"}</button>}
      </div> : null}
    </div>
    <button className="notification-card__close" aria-label="关闭通知" onClick={() => onDismiss(notification.id)}>×</button>
    {error && <div className="notification-card__error" role="alert">{error}<button aria-label="关闭错误提示" onClick={onErrorClear}>×</button></div>}
    {notification.actions?.length ? <footer>{notification.actions.map((action, actionIndex) => <button key={`${action.type}-${actionIndex}`} disabled={busyAction === actionIndex} onClick={() => onAction(notification.id, actionIndex)}>{busyAction === actionIndex ? "处理中…" : action.label || actionLabel(action)}</button>)}</footer> : null}
  </article>;
}

function actionLabel(action) {
  if (action.type === "open") return "打开";
  if (action.type === "open_dir") return "打开目录";
  return "关闭";
}
