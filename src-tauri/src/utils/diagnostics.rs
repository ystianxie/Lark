//! Local release diagnostics. Initialize before config/database/WebView startup.
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;
const BACKUPS: usize = 3;
const MAX_LOG_FILES: usize = 20;
const RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);
static LOGGER: OnceLock<LocalLogger> = OnceLock::new();
static LOG_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();

struct RotatingFile {
    path: PathBuf,
    file: Option<File>,
    bytes: u64,
    limit: u64,
}

impl RotatingFile {
    fn open(path: PathBuf, limit: u64) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let bytes = file.metadata()?.len();
        Ok(Self {
            path,
            file: Some(file),
            bytes,
            limit,
        })
    }

    fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        if self.bytes > 0 && self.bytes.saturating_add(bytes.len() as u64) > self.limit {
            // Close first: Windows cannot rename a file held without delete sharing.
            self.file.take();
            let oldest = self.path.with_extension(format!("{BACKUPS}.log"));
            if oldest.exists() {
                fs::remove_file(oldest)?;
            }
            for index in (1..BACKUPS).rev() {
                let from = self.path.with_extension(format!("{index}.log"));
                if from.exists() {
                    fs::rename(from, self.path.with_extension(format!("{}.log", index + 1)))?;
                }
            }
            fs::rename(&self.path, self.path.with_extension("1.log"))?;
            self.bytes = 0;
        }
        if self.file.is_none() {
            self.file = Some(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.path)?,
            );
        }
        self.file.as_mut().unwrap().write_all(bytes)?;
        self.bytes += bytes.len() as u64;
        Ok(())
    }
}

struct LocalLogger {
    writer: Mutex<RotatingFile>,
}

impl log::Log for LocalLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Debug
    }
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            write_record(
                &format!("{} {}", record.level(), record.target()),
                format!("{}", record.args()).as_bytes(),
            );
        }
    }
    fn flush(&self) {
        if let Ok(mut writer) = self.writer.lock() {
            if let Some(file) = &mut writer.file {
                let _ = file.flush();
            }
        }
    }
}

fn write_record(source: &str, bytes: &[u8]) {
    if let Some(logger) = LOGGER.get() {
        if let Ok(mut writer) = logger.writer.lock() {
            let prefix = format!(
                "{} [{source}] ",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f%:z")
            );
            let mut entry = prefix.into_bytes();
            entry.extend_from_slice(bytes);
            if !entry.ends_with(b"\n") {
                entry.push(b'\n');
            }
            // Logging errors cannot be sent back to stderr (it is captured by this logger).
            let _ = writer.write(&entry);
        }
    }
}

fn prune_logs(directory: &Path, now: SystemTime) -> io::Result<()> {
    let mut logs = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("lark-") || !name.ends_with(".log") || !entry.file_type()?.is_file() {
            continue;
        }
        let modified = entry.metadata()?.modified()?;
        logs.push((modified, entry.path()));
    }
    logs.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    for (index, (modified, path)) in logs.into_iter().enumerate() {
        if index >= MAX_LOG_FILES || now.duration_since(modified).unwrap_or_default() > RETENTION {
            let _ = fs::remove_file(path);
        }
    }
    Ok(())
}

pub fn initialize() -> io::Result<()> {
    let executable = std::env::current_exe()?;
    let install_logs = executable
        .parent()
        .unwrap_or(Path::new("."))
        .join("resources/data/logs");
    let mut candidates = vec![install_logs];
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        candidates.push(PathBuf::from(local).join("Lark/logs"));
    }
    candidates.push(std::env::temp_dir().join("Lark/logs"));
    let session = format!(
        "lark-{}-{}",
        chrono::Local::now().format("%Y%m%d-%H%M%S"),
        std::process::id()
    );
    let mut last_error = None;
    for directory in candidates {
        let opened = fs::create_dir_all(&directory).and_then(|_| {
            RotatingFile::open(directory.join(format!("{session}.log")), MAX_FILE_BYTES)
        });
        match opened {
            Ok(writer) => {
                let _ = LOG_DIRECTORY.set(directory.clone());
                LOGGER
                    .set(LocalLogger {
                        writer: Mutex::new(writer),
                    })
                    .map_err(|_| io::Error::other("logger already initialized"))?;
                let _ = log::set_logger(LOGGER.get().unwrap());
                log::set_max_level(log::LevelFilter::Debug);
                let _ = prune_logs(&directory, SystemTime::now());
                install_panic_hook(directory.join(format!("{session}.panic.log")));
                #[cfg(windows)]
                {
                    if let Err(error) = redirect_windows_streams() {
                        log::error!("捕获 stdout/stderr 失败: {error}");
                    }
                    super::crash_dump::initialize(directory.join(format!("{session}.dmp")));
                    if let Err(error) =
                        start_resource_monitor(directory.join(format!("{session}.resources.log")))
                    {
                        log::warn!("Resource monitoring could not start: {error}");
                    }
                }
                log::info!(
                    "session start: version={} pid={} executable={} logs={}",
                    env!("CARGO_PKG_VERSION"),
                    std::process::id(),
                    executable.display(),
                    directory.display()
                );
                return Ok(());
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| io::Error::other("no writable log directory")))
}

fn install_panic_hook(path: PathBuf) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Independent file and no logger lock: the panic may occur while logging holds that lock.
        let thread = std::thread::current();
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(
                file,
                "{} [PANIC] pid={} thread={:?} name={:?}\n{info}",
                chrono::Local::now().to_rfc3339(),
                std::process::id(),
                thread.id(),
                thread.name()
            );
            #[cfg(windows)]
            let _ = writeln!(
                file,
                "[resources] {}",
                super::resource_diagnostics::snapshot()
            );
            // Save resource evidence before symbol resolution, which can itself fail or stall.
            let _ = file.sync_all();
            let _ = writeln!(file, "{}", std::backtrace::Backtrace::force_capture());
            let _ = file.sync_all();
        }
        previous(info);
    }));
}

#[cfg(windows)]
fn write_resource_sample(writer: &mut RotatingFile, reason: &str) -> io::Result<()> {
    let mut sample = super::resource_diagnostics::snapshot();
    sample["reason"] = reason.into();
    writer.write(format!("{sample}\n").as_bytes())
}

#[cfg(windows)]
fn start_resource_monitor(path: PathBuf) -> io::Result<()> {
    let mut writer = RotatingFile::open(path, MAX_FILE_BYTES)?;
    write_resource_sample(&mut writer, "startup")?;
    std::thread::Builder::new()
        .name("resource-diagnostics".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(60));
            if let Err(error) = write_resource_sample(&mut writer, "periodic") {
                log::warn!("Resource snapshot could not be written: {error}");
            }
        })?;
    Ok(())
}

pub fn write_frontend_log(label: &str, level: &str, message: &str) {
    let level = match level {
        "error" => "ERROR",
        "warn" => "WARN",
        "debug" => "DEBUG",
        _ => "INFO",
    };
    let mut end = message.len().min(16384);
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    write_record(
        &format!("webview:{label} {level}"),
        message[..end].as_bytes(),
    );
}

#[cfg(any(windows, test))]
fn decode_stream_chunk(pending: &mut Vec<u8>) -> String {
    let mut text = String::new();
    let mut consumed = 0;
    while consumed < pending.len() {
        match std::str::from_utf8(&pending[consumed..]) {
            Ok(valid) => {
                text.push_str(valid);
                consumed = pending.len();
            }
            Err(error) => {
                let valid_end = consumed + error.valid_up_to();
                // The prefix is guaranteed to be valid by Utf8Error::valid_up_to.
                text.push_str(std::str::from_utf8(&pending[consumed..valid_end]).unwrap());
                consumed = valid_end;
                if let Some(length) = error.error_len() {
                    text.push('\u{fffd}');
                    consumed += length;
                } else {
                    break;
                }
            }
        }
    }
    pending.drain(..consumed);
    text
}

#[cfg(windows)]
fn redirect_windows_streams() -> io::Result<()> {
    use std::io::Read;
    use std::os::windows::io::{FromRawHandle, IntoRawHandle};
    use windows::Win32::{
        Foundation::HANDLE,
        System::{
            Console::{SetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE},
            Pipes::CreatePipe,
        },
    };
    for (kind, name) in [(STD_OUTPUT_HANDLE, "stdout"), (STD_ERROR_HANDLE, "stderr")] {
        let mut read = HANDLE::default();
        let mut write = HANDLE::default();
        unsafe { CreatePipe(&mut read, &mut write, None, 0) }.map_err(io::Error::other)?;
        let read_file = unsafe { File::from_raw_handle(read.0) };
        let write_file = unsafe { File::from_raw_handle(write.0) };
        // Keep the original console visible in development; release has no console to preserve.
        #[cfg(debug_assertions)]
        let original = unsafe { windows::Win32::System::Console::GetStdHandle(kind) }
            .ok()
            .map(|h| h.0 as usize);
        std::thread::Builder::new()
            .name(format!("log-{name}"))
            .spawn(move || {
                let mut reader = read_file;
                let mut buffer = [0u8; 8192];
                let mut pending = Vec::new();
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) | Err(_) => {
                            if !pending.is_empty() {
                                write_record(name, String::from_utf8_lossy(&pending).as_bytes());
                            }
                            break;
                        }
                        Ok(count) => {
                            pending.extend_from_slice(&buffer[..count]);
                            let text = decode_stream_chunk(&mut pending);
                            if !text.is_empty() {
                                write_record(name, text.as_bytes());
                            }
                            #[cfg(debug_assertions)]
                            if let Some(handle) = original {
                                use windows::Win32::Storage::FileSystem::WriteFile;
                                if handle != 0 && handle != usize::MAX {
                                    let _ = unsafe {
                                        WriteFile(
                                            HANDLE(handle as *mut _),
                                            Some(&buffer[..count]),
                                            None,
                                            None,
                                        )
                                    };
                                }
                            }
                        }
                    }
                }
            })?;
        unsafe { SetStdHandle(kind, write) }.map_err(io::Error::other)?;
        // Win32 owns the process standard handle for the remainder of this session.
        let _ = write_file.into_raw_handle();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temporary(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "lark-diagnostics-test-{name}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        directory
    }
    #[test]
    fn stream_preserves_split_unicode_and_flushes_text_without_newline() {
        let bytes = "hello 你好".as_bytes();
        let mut pending = bytes[..8].to_vec();
        let mut result = decode_stream_chunk(&mut pending);
        pending.extend_from_slice(&bytes[8..]);
        result.push_str(&decode_stream_chunk(&mut pending));
        assert_eq!(result, "hello 你好");
        assert!(pending.is_empty());
        let mut invalid = vec![0xff, b'x'];
        assert_eq!(decode_stream_chunk(&mut invalid), "\u{fffd}x");
    }
    #[test]
    fn rotation_keeps_new_output_and_bounds_backups() {
        let directory = temporary("rotation");
        let path = directory.join("lark-test.log");
        let mut writer = RotatingFile::open(path.clone(), 8).unwrap();
        for message in ["first\n", "second\n", "third\n", "fourth\n", "last\n"] {
            writer.write(message.as_bytes()).unwrap();
        }
        drop(writer);
        assert_eq!(fs::read_to_string(&path).unwrap(), "last\n");
        assert_eq!(
            fs::read_to_string(path.with_extension("1.log")).unwrap(),
            "fourth\n"
        );
        assert_eq!(fs::read_dir(&directory).unwrap().count(), BACKUPS + 1);
        fs::remove_dir_all(directory).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn resource_samples_are_readable_json_after_rotation() {
        let directory = temporary("resources");
        let path = directory.join("lark-test.resources.log");
        let mut writer = RotatingFile::open(path.clone(), 1).unwrap();
        write_resource_sample(&mut writer, "startup").unwrap();
        write_resource_sample(&mut writer, "periodic").unwrap();
        drop(writer);
        for (file, reason) in [
            (path.with_extension("1.log"), "startup"),
            (path, "periodic"),
        ] {
            let text = fs::read_to_string(file).unwrap();
            assert_eq!(text.lines().count(), 1);
            let sample: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(sample["reason"], reason);
            assert_eq!(sample["pid"], std::process::id());
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn pruning_removes_only_our_old_logs_and_preserves_crash_evidence() {
        let directory = temporary("pruning");
        for name in [
            "lark-old.log",
            "other.log",
            "lark-evidence.dmp",
            "lark-evidence.crash.txt",
        ] {
            fs::write(directory.join(name), b"evidence").unwrap();
        }
        prune_logs(
            &directory,
            SystemTime::now() + RETENTION + Duration::from_secs(2),
        )
        .unwrap();
        assert!(!directory.join("lark-old.log").exists());
        for name in ["other.log", "lark-evidence.dmp", "lark-evidence.crash.txt"] {
            assert!(directory.join(name).exists());
        }
        fs::remove_dir_all(directory).unwrap();
    }
}
