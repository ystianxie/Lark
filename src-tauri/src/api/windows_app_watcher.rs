#[cfg(target_os = "windows")]
use std::{
    ptr,
    sync::{mpsc, Mutex, OnceLock},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[cfg(target_os = "windows")]
use tauri::AppHandle;
#[cfg(target_os = "windows")]
use windows::{
    core::w,
    Win32::{
        Foundation::{BOOL, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        System::{
            Com::{CoInitialize, CoTaskMemFree, CoUninitialize},
            LibraryLoader::GetModuleHandleW,
            Threading::GetCurrentThreadId,
        },
        UI::{
            Shell::{
                Common::ITEMIDLIST, SHCNRF_InterruptLevel, SHCNRF_ShellLevel,
                SHChangeNotifyDeregister, SHChangeNotifyEntry, SHChangeNotifyRegister,
                SHParseDisplayName, SHCNE_ASSOCCHANGED, SHCNE_CREATE, SHCNE_DELETE, SHCNE_MKDIR,
                SHCNE_RENAMEFOLDER, SHCNE_RENAMEITEM, SHCNE_RMDIR, SHCNE_UPDATEITEM,
            },
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
                PostThreadMessageW, RegisterClassW, TranslateMessage, HWND_MESSAGE, MSG,
                WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WNDCLASSW,
            },
        },
    },
};

#[cfg(target_os = "windows")]
const SHELL_CHANGE_MESSAGE: u32 = WM_APP + 1;
#[cfg(target_os = "windows")]
const STOP_MESSAGE: u32 = WM_APP + 2;
#[cfg(target_os = "windows")]
const DEBOUNCE: Duration = Duration::from_secs(2);

#[cfg(target_os = "windows")]
const APP_INDEX_SHELL_EVENTS: u32 = SHCNE_ASSOCCHANGED.0
    | SHCNE_CREATE.0
    | SHCNE_DELETE.0
    | SHCNE_MKDIR.0
    | SHCNE_RENAMEFOLDER.0
    | SHCNE_RENAMEITEM.0
    | SHCNE_RMDIR.0
    | SHCNE_UPDATEITEM.0;

#[cfg(target_os = "windows")]
static CHANGE_SENDER: OnceLock<Mutex<Option<mpsc::SyncSender<RefreshRequest>>>> = OnceLock::new();

#[cfg(target_os = "windows")]
#[derive(Clone, Copy, Debug)]
enum RefreshRequest {
    ShellChanged,
    Immediate,
}

#[cfg(target_os = "windows")]
pub struct WindowsAppWatcher {
    refresh_sender: Option<mpsc::SyncSender<RefreshRequest>>,
    listener_thread_id: u32,
    listener_thread: Option<JoinHandle<()>>,
    refresh_thread: Option<JoinHandle<()>>,
}

#[cfg(target_os = "windows")]
impl WindowsAppWatcher {
    pub fn start(app_handle: AppHandle) -> Result<Self, String> {
        let (refresh_sender, refresh_receiver) = mpsc::sync_channel(1);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);

        *CHANGE_SENDER
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap() = Some(refresh_sender.clone());

        let listener_thread = thread::spawn(move || run_shell_listener(ready_sender));
        let listener_thread_id = match ready_receiver.recv() {
            Ok(Ok(thread_id)) => thread_id,
            Ok(Err(error)) => {
                clear_change_sender();
                let _ = listener_thread.join();
                return Err(error);
            }
            Err(_) => {
                clear_change_sender();
                let _ = listener_thread.join();
                return Err("AppsFolder 监听线程初始化失败".to_string());
            }
        };

        let refresh_thread = thread::spawn(move || run_refresh_loop(app_handle, refresh_receiver));
        Ok(Self {
            refresh_sender: Some(refresh_sender),
            listener_thread_id,
            listener_thread: Some(listener_thread),
            refresh_thread: Some(refresh_thread),
        })
    }

    pub fn request_refresh(&self) {
        if let Some(sender) = &self.refresh_sender {
            let _ = sender.try_send(RefreshRequest::Immediate);
        }
    }

    pub fn stop_in_place(&mut self) {
        clear_change_sender();
        unsafe {
            let _ = PostThreadMessageW(self.listener_thread_id, STOP_MESSAGE, WPARAM(0), LPARAM(0));
        }
        if let Some(thread) = self.listener_thread.take() {
            let _ = thread.join();
        }
        self.refresh_sender.take();
        if let Some(thread) = self.refresh_thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for WindowsAppWatcher {
    fn drop(&mut self) {
        self.stop_in_place();
    }
}

#[cfg(target_os = "windows")]
fn clear_change_sender() {
    if let Some(sender) = CHANGE_SENDER.get() {
        *sender.lock().unwrap() = None;
    }
}

#[cfg(target_os = "windows")]
fn run_refresh_loop(app_handle: AppHandle, receiver: mpsc::Receiver<RefreshRequest>) {
    while let Ok(request) = receiver.recv() {
        if matches!(request, RefreshRequest::ShellChanged) {
            let mut deadline = Instant::now() + DEBOUNCE;
            loop {
                let timeout = deadline.saturating_duration_since(Instant::now());
                match receiver.recv_timeout(timeout) {
                    Ok(RefreshRequest::ShellChanged) => deadline = Instant::now() + DEBOUNCE,
                    Ok(RefreshRequest::Immediate) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
        }

        println!("[AppIndexWatcher] 开始刷新应用索引");
        if let Err(error) = crate::api::explorer::create_app_index_to_sql(app_handle.clone()) {
            eprintln!("[AppIndexWatcher] 应用索引刷新失败，保留原有索引: {error}");
        }
    }
}

#[cfg(target_os = "windows")]
fn run_shell_listener(ready: mpsc::SyncSender<Result<u32, String>>) {
    unsafe {
        if let Err(error) = CoInitialize(None).ok() {
            let _ = ready.send(Err(format!("初始化 AppsFolder 监听 COM 失败: {error}")));
            return;
        }

        let result = create_shell_listener();
        match result {
            Ok((hwnd, registration_id, pidl)) => {
                let thread_id = GetCurrentThreadId();
                if ready.send(Ok(thread_id)).is_err() {
                    let _ = SHChangeNotifyDeregister(registration_id);
                    let _ = DestroyWindow(hwnd);
                    CoTaskMemFree(Some(pidl.cast()));
                    CoUninitialize();
                    return;
                }
                println!("[AppIndexWatcher] AppsFolder Shell 监听已启动");
                let mut message = MSG::default();
                loop {
                    let result = GetMessageW(&mut message, None, 0, 0);
                    if result.0 == -1 {
                        eprintln!("[AppIndexWatcher] Windows 消息循环读取失败");
                        break;
                    }
                    if result.0 == 0 || message.message == STOP_MESSAGE {
                        break;
                    }
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                let _ = SHChangeNotifyDeregister(registration_id);
                let _ = DestroyWindow(hwnd);
                CoTaskMemFree(Some(pidl.cast()));
                println!("[AppIndexWatcher] AppsFolder Shell 监听已停止");
            }
            Err(error) => {
                let _ = ready.send(Err(error));
            }
        }
        CoUninitialize();
    }
}

#[cfg(target_os = "windows")]
unsafe fn create_shell_listener() -> Result<(HWND, u32, *mut ITEMIDLIST), String> {
    let module = GetModuleHandleW(None).map_err(|error| error.to_string())?;
    let instance = HINSTANCE(module.0);
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        lpszClassName: w!("LarkAppsFolderWatcher"),
        ..Default::default()
    };
    RegisterClassW(&class);

    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("LarkAppsFolderWatcher"),
        w!(""),
        WINDOW_STYLE::default(),
        0,
        0,
        0,
        0,
        HWND_MESSAGE,
        None,
        instance,
        None,
    )
    .map_err(|error| format!("创建 AppsFolder 监听窗口失败: {error}"))?;

    let mut pidl = ptr::null_mut();
    if let Err(error) = SHParseDisplayName(w!("shell:AppsFolder"), None, &mut pidl, 0, None) {
        let _ = DestroyWindow(hwnd);
        return Err(format!("解析 shell:AppsFolder 失败: {error}"));
    }
    let entry = SHChangeNotifyEntry {
        pidl,
        fRecursive: BOOL(1),
    };
    let registration_id = SHChangeNotifyRegister(
        hwnd,
        SHCNRF_ShellLevel | SHCNRF_InterruptLevel,
        APP_INDEX_SHELL_EVENTS as i32,
        SHELL_CHANGE_MESSAGE,
        1,
        &entry,
    );
    if registration_id == 0 {
        CoTaskMemFree(Some(pidl.cast()));
        let _ = DestroyWindow(hwnd);
        return Err("注册 AppsFolder Shell 变化通知失败".to_string());
    }
    Ok((hwnd, registration_id, pidl))
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == SHELL_CHANGE_MESSAGE {
        // Shell notifications share one window message. Only changes that can
        // alter the AppsFolder contents should trigger the expensive rebuild;
        // disk/free-space and unrelated visual notifications are ignored.
        let event = wparam.0 as u32;
        if event & APP_INDEX_SHELL_EVENTS == 0 {
            return LRESULT(0);
        }
        if let Some(sender) = CHANGE_SENDER.get().and_then(|value| value.lock().ok()) {
            if let Some(sender) = sender.as_ref() {
                let _ = sender.try_send(RefreshRequest::ShellChanged);
            }
        }
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}

#[cfg(not(target_os = "windows"))]
pub struct WindowsAppWatcher;
