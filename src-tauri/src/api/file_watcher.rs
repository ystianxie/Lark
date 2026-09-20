//! 文件系统增量监听（第二阶段独立模块）。
//!
//! 本模块只负责监听、过滤、重命名识别和合并文件系统事件，不负责启动应用或写入数据库。
//! 调用方应在全量索引完成后启动 [`FileWatcher::start`]，并在回调中批量更新 `file_index`。

use crate::utils::database::{FileIndex, FileIndexChange};
use notify::event::{ModifyKind, RenameMode};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEventKind {
    Created,
    Changed,
    Removed,
    Renamed { from: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEvent {
    pub kind: FileEventKind,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct WatchConfig {
    pub roots: Vec<PathBuf>,
    pub excluded_paths: Vec<PathBuf>,
    pub excluded_extensions: Vec<String>,
    pub debounce: Duration,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            excluded_paths: Vec::new(),
            excluded_extensions: Vec::new(),
            debounce: Duration::from_millis(300),
        }
    }
}

impl WatchConfig {
    pub fn with_debounce(mut self, debounce: Duration) -> Self {
        self.debounce = debounce;
        self
    }
}

pub struct FileWatcher {
    stop: Arc<Mutex<bool>>,
    thread: Option<JoinHandle<()>>,
}

/// 串行处理增量索引写入，确保同一个 SQLite 连接不被多个线程并发使用。
pub struct FileIndexUpdateService {
    sender: Sender<IndexMessage>,
    stop: Arc<Mutex<bool>>,
    thread: Option<JoinHandle<()>>,
    status: Arc<Mutex<IndexStatus>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum IndexStatus { Idle, Rebuilding, IncrementalUpdating, Stale }

impl std::fmt::Display for IndexStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:?}", self) }
}

impl FileIndexUpdateService {
    pub fn start() -> Self {
        let (sender, receiver) = mpsc::channel::<IndexMessage>();
        let stop = Arc::new(Mutex::new(false));
        let thread_stop = Arc::clone(&stop);
        let status = Arc::new(Mutex::new(IndexStatus::Idle));
        let thread_status = Arc::clone(&status);
        let thread = thread::spawn(move || run_index_service(receiver, thread_stop, thread_status));
        Self {
            sender,
            stop,
            thread: Some(thread),
            status,
        }
    }

    pub fn sender(&self) -> Sender<IndexMessage> {
        self.sender.clone()
    }

    pub fn begin_rebuild(&self) -> Result<(), String> { self.sender.send(IndexMessage::BeginRebuild).map_err(|e| e.to_string()) }
    pub fn finish_rebuild(&self) -> Result<(), String> { self.sender.send(IndexMessage::FinishRebuild).map_err(|e| e.to_string()) }
    pub fn status(&self) -> IndexStatus { *self.status.lock().unwrap() }
    pub fn apply_batches_sync(&self, batches: Vec<Vec<FileIndexChange>>) -> Result<(), String> {
        for batch in batches {
            self.sender.send(IndexMessage::Batch(batch)).map_err(|e| e.to_string())?;
        }
        let (tx, rx) = mpsc::channel();
        self.sender.send(IndexMessage::Flush(tx)).map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())
    }
}

pub enum IndexMessage { Batch(Vec<FileIndexChange>), BeginRebuild, FinishRebuild, Flush(Sender<()>), MarkStale }

impl Drop for FileIndexUpdateService {
    fn drop(&mut self) {
        *self.stop.lock().unwrap() = true;
        drop(self.sender.clone());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run_index_service(receiver: Receiver<IndexMessage>, stop: Arc<Mutex<bool>>, status: Arc<Mutex<IndexStatus>>) {
    let mut index = crate::utils::database::IndexSQL::new();
    let mut rebuilding = false;
    let mut pending = Vec::new();
    while !*stop.lock().unwrap() {
        match receiver.recv_timeout(Duration::from_millis(250)) {
            Ok(IndexMessage::BeginRebuild) => { rebuilding = true; *status.lock().unwrap() = IndexStatus::Rebuilding; }
            Ok(IndexMessage::Batch(changes)) if rebuilding => pending.extend(changes),
            Ok(IndexMessage::Batch(changes)) if !changes.is_empty() => {
                *status.lock().unwrap() = IndexStatus::IncrementalUpdating;
                println!("[FileWatcher] 开始写入 {} 个增量索引变更", changes.len());
                let mut failed = false;
                for chunk in changes.chunks(5000) {
                    if let Err(error) = index.apply_file_changes(chunk) {
                        eprintln!("[FileWatcher] 增量索引写入失败: {error}");
                        failed = true;
                        break;
                    }
                }
                *status.lock().unwrap() = if failed { IndexStatus::Stale } else { IndexStatus::Idle };
            }
            Ok(IndexMessage::FinishRebuild) => {
                rebuilding = false;
                if !pending.is_empty() {
                    if let Err(error) = index.apply_file_changes(&pending) {
                        eprintln!("[FileWatcher] 重建后补写增量索引失败: {error}");
                        *status.lock().unwrap() = IndexStatus::Stale;
                    }
                    pending.clear();
                }
                if *status.lock().unwrap() != IndexStatus::Stale {
                    *status.lock().unwrap() = IndexStatus::Idle;
                }
            }
            Ok(IndexMessage::Flush(done)) => { let _ = done.send(()); }
            Ok(IndexMessage::MarkStale) => { *status.lock().unwrap() = IndexStatus::Stale; }
            Ok(IndexMessage::Batch(_)) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

impl FileWatcher {
    pub fn start<F>(config: WatchConfig, on_batch: F) -> notify::Result<Self>
    where
        F: Fn(Vec<FileEvent>) + Send + Sync + 'static,
    {
        if config.roots.is_empty() {
            return Err(notify::Error::generic("文件监听未配置任何根目录"));
        }
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let stop = Arc::new(Mutex::new(false));
        let thread_stop = Arc::clone(&stop);
        let callback = Arc::new(on_batch);
        let thread = thread::spawn(move || run(config, thread_stop, callback, ready_tx));
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                stop,
                thread: Some(thread),
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error)
            }
            Err(_) => Err(notify::Error::generic("文件监听线程初始化失败")),
        }
    }

    pub fn stop(mut self) {
        *self.stop.lock().unwrap() = true;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }

    pub fn stop_in_place(&mut self) {
        *self.stop.lock().unwrap() = true;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        *self.stop.lock().unwrap() = true;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run<F>(
    config: WatchConfig,
    stop: Arc<Mutex<bool>>,
    callback: Arc<F>,
    ready: mpsc::SyncSender<notify::Result<()>>,
) where
    F: Fn(Vec<FileEvent>) + Send + Sync + 'static,
{
    let (tx, rx) = mpsc::channel();
    let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
        Ok(w) => w,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };
    for root in &config.roots {
        if let Err(error) = watcher.watch(root, RecursiveMode::Recursive) {
            let _ = ready.send(Err(error));
            return;
        }
    }
    let _ = ready.send(Ok(()));
    let mut pending = Vec::new();
    loop {
        if *stop.lock().unwrap() {
            break;
        }
        match rx.recv_timeout(config.debounce) {
            Ok(Ok(event)) => {
                let normalized = normalize(event.clone(), &config);
            if !normalized.is_empty() {
                    println!("[FileWatcher] 原始事件: {:?}, paths={:?}", event.kind, event.paths);
                    pending.extend(normalized);
                }
            }
            Ok(Err(error)) => eprintln!("[FileWatcher] 监听事件错误: {error}"),
            Err(RecvTimeoutError::Timeout) => {
                let batch = coalesce(std::mem::take(&mut pending));
                if !batch.is_empty() {
                    callback(batch);
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn normalize(event: Event, config: &WatchConfig) -> Vec<FileEvent> {
    if let EventKind::Modify(ModifyKind::Name(RenameMode::Both)) = event.kind {
        if event.paths.len() >= 2 {
            let from = event.paths[0].clone();
            let to = event.paths[1].clone();
            let from_excluded = is_excluded(&from, config);
            let to_excluded = is_excluded(&to, config);
            return match (from_excluded, to_excluded) {
                (false, false) => vec![FileEvent {
                    kind: FileEventKind::Renamed { from },
                    path: to,
                }],
                (false, true) => vec![FileEvent {
                    kind: FileEventKind::Removed,
                    path: from,
                }],
                (true, false) => vec![FileEvent {
                    kind: FileEventKind::Created,
                    path: to,
                }],
                (true, true) => Vec::new(),
            };
        }
        return Vec::new();
    }
    let kind = match event.kind {
        EventKind::Create(_) => FileEventKind::Created,
        // Windows 常把重命名拆成 From/To 两个事件；分别映射为删除和新增，
        // 避免把旧路径当成普通修改再次写回索引。
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => FileEventKind::Removed,
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => FileEventKind::Created,
        EventKind::Modify(ModifyKind::Name(RenameMode::Any)) => FileEventKind::Changed,
        EventKind::Modify(_) => FileEventKind::Changed,
        EventKind::Remove(_) => FileEventKind::Removed,
        _ => return Vec::new(),
    };
    event
        .paths
        .into_iter()
        .filter(|path| !is_excluded(path, config))
        .map(|path| FileEvent {
            kind: kind.clone(),
            path,
        })
        .collect()
}

pub fn is_excluded(path: &Path, config: &WatchConfig) -> bool {
    if path.components().any(|component| match component {
        std::path::Component::Normal(name) => name
            .to_str()
            .is_some_and(|name| name.starts_with('$') || name.starts_with('.')),
        _ => false,
    }) {
        return true;
    }
    if config.excluded_paths.iter().any(|excluded| path_is_under(path, excluded)) {
        return true;
    }
    path.extension().and_then(|e| e.to_str()).is_some_and(|ext| {
        config.excluded_extensions.iter().any(|x| {
            x.trim().trim_start_matches('.').eq_ignore_ascii_case(ext)
        })
    })
}

pub(crate) fn path_is_under(path: &Path, excluded: &Path) -> bool {
    let path = normalize_compare_path(path);
    let excluded = normalize_compare_path(excluded);

    if let Some(relative) = excluded.strip_prefix("*\\") {
        if relative.is_empty() {
            return false;
        }
        let needle = format!("\\{relative}");
        return path == relative
            || path.ends_with(&needle)
            || path.contains(&(needle + "\\"));
    }

    path == excluded || path.starts_with(&(excluded + "\\"))
}

fn normalize_compare_path(path: &Path) -> String {
    let mut value = path.to_string_lossy().replace('/', "\\");
    while value.ends_with('\\') && value.len() > 3 { value.pop(); }
    if value.starts_with(r"\\?\") { value = value[4..].to_string(); }
    value.to_ascii_lowercase()
}

/// 将监听事件转换为数据库变更；不存在的新增/修改路径会被忽略。
pub fn events_to_index_changes(
    events: &[FileEvent],
    to_index: impl Fn(&Path) -> Option<FileIndex>,
) -> Vec<FileIndexChange> {
    let mut changes = Vec::new();
    for event in events {
        match &event.kind {
            FileEventKind::Created | FileEventKind::Changed => {
                append_path_changes(&mut changes, &event.path, &to_index);
            }
            FileEventKind::Removed => {
                changes.push(FileIndexChange::Remove {
                    path: event.path.to_string_lossy().into_owned(),
                    recursive: true,
                });
            }
            FileEventKind::Renamed { from } => {
                if let Some(to) = to_index(&event.path) {
                    if Path::new(&to.path).is_dir() {
                        changes.push(FileIndexChange::Remove { path: from.to_string_lossy().into_owned(), recursive: true });
                        append_path_changes(&mut changes, &event.path, &to_index);
                    } else {
                        changes.push(FileIndexChange::Rename { from: from.to_string_lossy().into_owned(), to });
                    }
                } else {
                    // Destination disappeared or could not be read: remove the old subtree.
                    changes.push(FileIndexChange::Remove { path: from.to_string_lossy().into_owned(), recursive: true });
                }
            }
        }
    }
    println!("[FileWatcher] 事件转换结果: {} 个变更", changes.len());
    changes
}

fn append_path_changes(
    changes: &mut Vec<FileIndexChange>,
    path: &Path,
    to_index: &impl Fn(&Path) -> Option<FileIndex>,
) {
    let Some(index) = to_index(path) else { return };
    let is_dir = Path::new(&index.path).is_dir();
    changes.push(FileIndexChange::Upsert(index));
    if is_dir {
        for entry in walkdir::WalkDir::new(path).follow_links(false).into_iter().filter_map(Result::ok) {
            if entry.path() == path || entry.file_type().is_dir() { continue; }
            if let Some(index) = to_index(entry.path()) {
                changes.push(FileIndexChange::Upsert(index));
            }
        }
    }

}

fn coalesce(events: Vec<FileEvent>) -> Vec<FileEvent> {
    let mut final_events = HashMap::<PathBuf, FileEvent>::new();
    for event in events {
        let path = event.path.clone();
        match final_events.get(&path) {
            Some(previous)
                if matches!(previous.kind, FileEventKind::Removed)
                    && matches!(event.kind, FileEventKind::Changed) => {}
            _ => {
                final_events.insert(path, event);
            }
        }
    }
    final_events.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_file_events_to_index_changes() {
        let events = vec![
            FileEvent { kind: FileEventKind::Created, path: PathBuf::from("a.txt") },
            FileEvent { kind: FileEventKind::Removed, path: PathBuf::from("gone.txt") },
            FileEvent { kind: FileEventKind::Renamed { from: PathBuf::from("old.txt") }, path: PathBuf::from("new.txt") },
        ];
        let changes = events_to_index_changes(&events, |path| {
            (path == Path::new("a.txt") || path == Path::new("new.txt")).then(|| FileIndex {
                title: path.file_name().unwrap().to_string_lossy().into_owned(),
                path: path.to_string_lossy().into_owned(),
                ..Default::default()
            })
        });
        assert_eq!(changes.len(), 3);
        assert!(matches!(changes[0], FileIndexChange::Upsert(_)));
        assert!(matches!(changes[1], FileIndexChange::Remove { .. }));
        assert!(matches!(changes[2], FileIndexChange::Rename { .. }));
    }

    #[test]
    fn rename_events_respect_excluded_paths() {
        let config = WatchConfig {
            excluded_paths: vec![PathBuf::from(r"C:\included\excluded")],
            ..Default::default()
        };

        let into_excluded = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(PathBuf::from(r"C:\included\file.txt"))
            .add_path(PathBuf::from(r"C:\included\excluded\file.txt"));
        assert!(matches!(
            normalize(into_excluded, &config).as_slice(),
            [FileEvent { kind: FileEventKind::Removed, path }]
                if path == Path::new(r"C:\included\file.txt")
        ));

        let out_of_excluded = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(PathBuf::from(r"C:\included\excluded\file.txt"))
            .add_path(PathBuf::from(r"C:\included\file.txt"));
        assert!(matches!(
            normalize(out_of_excluded, &config).as_slice(),
            [FileEvent { kind: FileEventKind::Created, path }]
                if path == Path::new(r"C:\included\file.txt")
        ));
    }

    #[test]
    fn remove_events_in_excluded_paths_are_filtered() {
        let config = WatchConfig {
            excluded_paths: vec![PathBuf::from(r"C:\ProgramData")],
            ..Default::default()
        };
        let event = Event::new(EventKind::Remove(notify::event::RemoveKind::Any))
            .add_path(PathBuf::from(r"C:\ProgramData\Windhawk\mod-status"));

        assert!(normalize(event, &config).is_empty());
    }
    #[test]
    fn wildcard_excluded_paths_match_component_sequences() {
        assert!(path_is_under(
            Path::new(r"D:\Project\Lark\src-tauri\target\debug\config"),
            Path::new(r"*/src-tauri/target"),
        ));
        assert!(path_is_under(
            Path::new(r"D:\Project\Lark\node_modules\pkg"),
            Path::new(r"*/node_modules"),
        ));
        assert!(!path_is_under(
            Path::new(r"D:\Project\Lark\src-tauri\target2\debug"),
            Path::new(r"*/src-tauri/target"),
        ));
    }

    #[test]
    fn excluded_paths_and_extensions_are_filtered() {
        let config = WatchConfig { excluded_paths: vec![PathBuf::from("target")], excluded_extensions: vec!["tmp".into()], ..Default::default() };
        assert!(is_excluded(Path::new("target/a.txt"), &config));
        assert!(is_excluded(Path::new("a.TMP"), &config));
        assert!(is_excluded(Path::new(r"project\.git\config"), &config));
        assert!(is_excluded(Path::new(r"project\.cache\data.txt"), &config));
        assert!(!is_excluded(Path::new(r"project\visible\data.txt"), &config));
        assert!(!is_excluded(Path::new("a.rs"), &config));
    }
}
