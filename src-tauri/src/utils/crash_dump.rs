//! Best-effort native crash dump. DbgHelp runs in a separate short-lived process,
//! never on the damaged application's thread or while its loader lock is held.
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::windows::{
    io::{AsRawHandle, FromRawHandle, OwnedHandle},
    process::CommandExt,
};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};
use std::time::{Duration, Instant};
use windows::Win32::{
    Foundation::{BOOL, HANDLE},
    System::{
        Diagnostics::Debug::{
            MiniDumpNormal, MiniDumpWithThreadInfo, MiniDumpWithUnloadedModules, MiniDumpWriteDump,
            SetUnhandledExceptionFilter, EXCEPTION_CONTINUE_SEARCH, EXCEPTION_POINTERS,
            MINIDUMP_EXCEPTION_INFORMATION,
        },
        Threading::{
            GetCurrentProcessId, GetCurrentThreadId, OpenProcess, PROCESS_QUERY_INFORMATION,
            PROCESS_VM_READ,
        },
    },
};

const REPORTER_FLAG: &str = "--lark-write-crash-dump";
struct CrashContext {
    executable: PathBuf,
    dump: PathBuf,
    report: PathBuf,
}
static CONTEXT: OnceLock<CrashContext> = OnceLock::new();
static HANDLING: AtomicBool = AtomicBool::new(false);

pub fn initialize(dump: PathBuf) {
    if let Ok(executable) = std::env::current_exe() {
        let report = dump.with_extension("crash.txt");
        let _ = CONTEXT.set(CrashContext {
            executable,
            dump,
            report,
        });
        install_handler();
    }
}

pub fn install_handler() {
    if CONTEXT.get().is_some() {
        unsafe {
            SetUnhandledExceptionFilter(Some(unhandled_exception));
        }
    }
}

unsafe extern "system" fn unhandled_exception(exception: *const EXCEPTION_POINTERS) -> i32 {
    if exception.is_null() || HANDLING.swap(true, Ordering::SeqCst) {
        return EXCEPTION_CONTINUE_SEARCH;
    }
    let Some(context) = CONTEXT.get() else {
        return EXCEPTION_CONTINUE_SEARCH;
    };
    // Do not use the normal logger: the crashing thread may already hold its mutex.
    if let Ok(mut file) = File::create(&context.report) {
        let record = (*exception).ExceptionRecord;
        if !record.is_null() {
            let _ = writeln!(
                file,
                "native crash: pid={} thread={} code=0x{:08X} address={:p}",
                GetCurrentProcessId(),
                GetCurrentThreadId(),
                (*record).ExceptionCode.0 as u32,
                (*record).ExceptionAddress
            );
        }
        let _ = file.sync_all();
    }
    let outcome = Command::new(&context.executable)
        .arg(REPORTER_FLAG)
        .arg(GetCurrentProcessId().to_string())
        .arg(GetCurrentThreadId().to_string())
        .arg((exception as usize).to_string())
        .arg(&context.dump)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn();
    match outcome {
        Ok(mut helper) => {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                match helper.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(20))
                    }
                    _ => {
                        let _ = helper.kill();
                        let _ = helper.wait();
                        append_report(
                            &context.report,
                            "dump helper timed out or could not be waited on",
                        );
                        break;
                    }
                }
            }
        }
        Err(error) => append_report(
            &context.report,
            &format!("could not launch dump helper: {error}"),
        ),
    }
    EXCEPTION_CONTINUE_SEARCH
}

fn append_report(path: &std::path::Path, message: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
        let _ = file.sync_all();
    }
}

// Must run before any normal application initialization; the helper must not create
// another WebView, open the database, register shortcuts, or touch the user's config.
pub fn run_reporter_if_requested() -> bool {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_none_or(|arg| arg != REPORTER_FLAG) {
        return false;
    }
    if args.len() != 6 {
        return true;
    }
    let dump = PathBuf::from(&args[5]);
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let pid: u32 = args[2].to_string_lossy().parse()?;
        let thread_id: u32 = args[3].to_string_lossy().parse()?;
        let pointer: usize = args[4].to_string_lossy().parse()?;
        if pointer == 0 {
            return Err("missing exception pointer".into());
        }
        let handle =
            unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)? };
        let process = unsafe { OwnedHandle::from_raw_handle(handle.0) };
        let file = File::create(&dump)?;
        let exception = MINIDUMP_EXCEPTION_INFORMATION {
            ThreadId: thread_id,
            ExceptionPointers: pointer as *mut EXCEPTION_POINTERS,
            ClientPointers: BOOL(1), // The exception pointers live in the parent process.
        };
        unsafe {
            MiniDumpWriteDump(
                HANDLE(process.as_raw_handle()),
                pid,
                HANDLE(file.as_raw_handle()),
                MiniDumpNormal | MiniDumpWithThreadInfo | MiniDumpWithUnloadedModules,
                Some(&exception),
                None,
                None,
            )?;
        }
        file.sync_all()?;
        Ok(())
    })();
    match result {
        Ok(()) => append_report(
            &dump.with_extension("crash.txt"),
            "native dump written successfully",
        ),
        Err(error) => {
            append_report(
                &dump.with_extension("crash.txt"),
                &format!("native dump failed: {error}"),
            );
            let _ = std::fs::remove_file(&dump);
        }
    }
    true
}
