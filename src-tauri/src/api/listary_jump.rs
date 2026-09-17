//! Listary 风格目录跳转 MVP。
//!
//! 启动后持续记住最近一次位于前台的资源管理器目录；主动触发时，
//! 仅当当前前台窗口被确认是文件对话框，才把该目录提交给对话框。

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::api::dialog_navigator::{navigate_foreground_dialog, DialogNavigationMethod};
use crate::api::explorer_listener::{start_listener, ExplorerListenerHandle};

/// MVP 的运行句柄。必须由应用长期持有；丢弃后监听器会停止。
pub struct ListaryJumpHandle {
    latest_explorer_path: Arc<RwLock<Option<PathBuf>>>,
    _listener: ExplorerListenerHandle,
}

impl ListaryJumpHandle {
    /// 启动资源管理器路径监听。
    pub fn start() -> Self {
        let latest_explorer_path = Arc::new(RwLock::new(None));
        let listener_path = Arc::clone(&latest_explorer_path);
        let listener = start_listener(move |path| {
            if let Ok(mut latest) = listener_path.write() {
                *latest = Some(path);
            }
        });

        Self {
            latest_explorer_path,
            _listener: listener,
        }
    }

    /// 主动执行一次跳转。
    ///
    /// 这是预留给全局 Ctrl+G 回调的入口。调用线程可能短暂阻塞，
    /// 正式接入快捷键时建议放入普通后台线程执行。
    pub fn trigger_ctrl_g(&self) -> Result<DialogNavigationMethod, String> {
        let path = self
            .latest_explorer_path
            .read()
            .map_err(|_| "资源管理器路径状态已损坏".to_string())?
            .clone()
            .ok_or_else(|| "尚未捕获到资源管理器目录，请先激活一次资源管理器".to_string())?;

        // navigator 内部会再次调用 dialog_probe，只对 Confirmed 对话框操作。
        navigate_foreground_dialog(path)
    }

    /// 返回当前缓存路径，便于界面显示和手工排查。
    pub fn latest_explorer_path(&self) -> Result<Option<PathBuf>, String> {
        self.latest_explorer_path
            .read()
            .map(|path| path.clone())
            .map_err(|_| "资源管理器路径状态已损坏".to_string())
    }
}

/// 手工测试入口：启动监听并返回句柄。
///
/// 测试流程：先激活一个资源管理器目录，再打开文件对话框，最后调用
/// `handle.trigger_ctrl_g()`。
pub fn start_test_mvp() -> ListaryJumpHandle {
    ListaryJumpHandle::start()
}
