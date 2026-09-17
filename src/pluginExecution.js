const queues = new Map();

export function schedulePluginExecution(key, execute, isCurrent) {
  let queue = queues.get(key);
  if (!queue) {
    queue = {running: false, pending: null};
    queues.set(key, queue);
  }
  return new Promise((resolve, reject) => {
    queue.pending?.resolve(undefined);
    queue.pending = {execute, isCurrent, resolve, reject};
    const drain = async () => {
      if (queue.running) return;
      queue.running = true;
      try {
        while (queue.pending) {
          const task = queue.pending;
          queue.pending = null;
          if (!task.isCurrent()) { task.resolve(undefined); continue; }
          try { task.resolve(await task.execute()); }
          catch (error) { task.reject(error); }
        }
      } finally {
        queue.running = false;
        queues.delete(key);
      }
    };
    drain();
  });
}
