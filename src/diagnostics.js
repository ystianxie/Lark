const INSTALLED = Symbol.for('lark.diagnostics');
const MESSAGE_LIMIT = 16384;

function describe(value) {
  try {
    if (value instanceof Error) return value.stack || `${value.name}: ${value.message}`;
    if (typeof value === 'string') return value;
    const seen = new WeakSet();
    return JSON.stringify(value, (_key, item) => {
      if (typeof item === 'bigint') return String(item);
      if (item && typeof item === 'object') {
        if (seen.has(item)) return '[Circular]';
        seen.add(item);
      }
      return item;
    }) ?? String(value);
  } catch {
    return '[unserializable value]';
  }
}

// The transport is injected so logging itself never depends on console or recurses on IPC failure.
export function installDiagnostics(host, transport) {
  if (host[INSTALLED]) return;
  host[INSTALLED] = true;
  const send = (level, args) => {
    try {
      let message = args.map(describe).join(' ');
      if (message.length > MESSAGE_LIMIT) message = message.slice(0, MESSAGE_LIMIT - 12) + ' [truncated]';
      Promise.resolve(transport({level, message})).catch(() => {});
    } catch { /* An unavailable logger must not break the application. */ }
  };
  for (const level of ['log', 'info', 'warn', 'error', 'debug']) {
    const original = host.console[level].bind(host.console);
    host.console[level] = (...args) => {
      original(...args);
      send(level, args);
    };
  }
  host.addEventListener('error', event => {
    send('error', ['Uncaught error', `${event.filename || ''}:${event.lineno || 0}:${event.colno || 0}`, event.error || event.message]);
  });
  host.addEventListener('unhandledrejection', event => send('error', ['Unhandled rejection', event.reason]));
}

export const reactErrorHandlers = Object.fromEntries(
  ['onCaughtError', 'onUncaughtError', 'onRecoverableError'].map(name =>
    [name, (error, info) => console.error(`[React ${name}]`, error, info?.componentStack || '')]),
);
