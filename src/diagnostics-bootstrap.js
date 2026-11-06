import {invoke} from '@tauri-apps/api/core';
import {installDiagnostics} from './diagnostics.js';

if (window.__TAURI_INTERNALS__) {
  let queue = [];
  let timer;
  const flush = () => {
    clearTimeout(timer);
    timer = undefined;
    const entries = queue;
    queue = [];
    if (entries.length) invoke('write_frontend_logs', {entries}).catch(() => {});
  };
  installDiagnostics(window, entry => {
    queue.push(entry);
    if (entry.level === 'error' || queue.length >= 50) flush();
    else if (!timer) timer = setTimeout(flush, 100);
  });
  window.addEventListener('pagehide', flush);
}
