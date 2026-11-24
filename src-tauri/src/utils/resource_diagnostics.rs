//! Windows resource snapshots for diagnosing long-running native drawing failures.
use serde_json::{json, Value};
use std::mem::size_of;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows::Win32::{
    Foundation::{GetLastError, SetLastError, ERROR_NO_MORE_FILES, ERROR_SUCCESS, HANDLE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
        },
        ProcessStatus::{
            GetPerformanceInfo, GetProcessMemoryInfo, PERFORMANCE_INFORMATION,
            PROCESS_MEMORY_COUNTERS_EX,
        },
        Threading::{
            GetCurrentProcess, GetGuiResources, GetProcessHandleCount, GET_GUI_RESOURCES_FLAGS,
            GR_GDIOBJECTS, GR_GDIOBJECTS_PEAK, GR_USEROBJECTS, GR_USEROBJECTS_PEAK,
        },
    },
    UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CMONITORS, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_REMOTESESSION,
    },
};

fn api_error(error: windows::core::Error) -> Value {
    json!({"error": error.to_string(), "hresult": error.code().0})
}

fn gui_resources(process: HANDLE, flag: GET_GUI_RESOURCES_FLAGS) -> Value {
    // Zero also means a process has no GUI objects; distinguish a failed query.
    unsafe {
        SetLastError(ERROR_SUCCESS);
        let count = GetGuiResources(process, flag);
        let error = GetLastError();
        if count == 0 && error != ERROR_SUCCESS {
            json!({"win32_error": error.0})
        } else {
            json!(count)
        }
    }
}

fn thread_count() -> windows::core::Result<u32> {
    unsafe {
        let snapshot =
            OwnedHandle::from_raw_handle(CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0)?.0);
        let handle = HANDLE(snapshot.as_raw_handle());
        let mut entry = THREADENTRY32 {
            dwSize: size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        Thread32First(handle, &mut entry)?;
        let mut count = 0;
        loop {
            if entry.th32OwnerProcessID == std::process::id() {
                count += 1;
            }
            if let Err(error) = Thread32Next(handle, &mut entry) {
                if error.code() == ERROR_NO_MORE_FILES.to_hresult() {
                    return Ok(count);
                }
                return Err(error);
            }
        }
    }
}

pub fn snapshot() -> Value {
    unsafe {
        let process = GetCurrentProcess();
        let mut memory = PROCESS_MEMORY_COUNTERS_EX {
            cb: size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
            ..Default::default()
        };
        let memory = match GetProcessMemoryInfo(process, &mut memory as *mut _ as _, memory.cb) {
            Ok(()) => json!({
                "working_set_bytes": memory.WorkingSetSize,
                "peak_working_set_bytes": memory.PeakWorkingSetSize,
                "private_bytes": memory.PrivateUsage,
                "peak_commit_bytes": memory.PeakPagefileUsage,
                "paged_pool_bytes": memory.QuotaPagedPoolUsage,
                "nonpaged_pool_bytes": memory.QuotaNonPagedPoolUsage,
            }),
            Err(error) => api_error(error),
        };
        let mut handles = 0;
        let handles = match GetProcessHandleCount(process, &mut handles) {
            Ok(()) => json!(handles),
            Err(error) => api_error(error),
        };
        let threads = match thread_count() {
            Ok(count) => json!(count),
            Err(error) => api_error(error),
        };
        let mut system = PERFORMANCE_INFORMATION::default();
        let system = match GetPerformanceInfo(
            &mut system,
            size_of::<PERFORMANCE_INFORMATION>() as u32,
        ) {
            Ok(()) => json!({
                "physical_total_bytes": system.PhysicalTotal as u64 * system.PageSize as u64,
                "physical_available_bytes": system.PhysicalAvailable as u64 * system.PageSize as u64,
                "commit_bytes": system.CommitTotal as u64 * system.PageSize as u64,
                "commit_limit_bytes": system.CommitLimit as u64 * system.PageSize as u64,
                "peak_commit_bytes": system.CommitPeak as u64 * system.PageSize as u64,
                "handles": system.HandleCount,
                "processes": system.ProcessCount,
                "threads": system.ThreadCount,
            }),
            Err(error) => api_error(error),
        };
        json!({
            "timestamp": chrono::Local::now().to_rfc3339(),
            "pid": std::process::id(),
            "process": {
                "memory": memory,
                "handles": handles,
                "threads": threads,
                "gdi": gui_resources(process, GR_GDIOBJECTS),
                "gdi_peak": gui_resources(process, GR_GDIOBJECTS_PEAK),
                "user": gui_resources(process, GR_USEROBJECTS),
                "user_peak": gui_resources(process, GR_USEROBJECTS_PEAK),
            },
            "system": system,
            "display": {
                "remote_session": GetSystemMetrics(SM_REMOTESESSION) != 0,
                "monitors": GetSystemMetrics(SM_CMONITORS),
                "virtual_width": GetSystemMetrics(SM_CXVIRTUALSCREEN),
                "virtual_height": GetSystemMetrics(SM_CYVIRTUALSCREEN),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};

    #[test]
    fn failed_gui_query_is_not_reported_as_zero_usage() {
        let result = gui_resources(HANDLE::default(), GR_GDIOBJECTS);
        assert!(result["win32_error"].as_u64().unwrap() > 0);
    }

    #[test]
    fn snapshot_reads_live_windows_counters_without_leaking_handles() {
        let handles = || {
            let mut count = 0;
            unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count).unwrap() };
            count
        };
        // Exclude one-time initialization in the Windows/time libraries from the leak check.
        let _ = snapshot();
        let before = handles();
        for _ in 0..20 {
            let sample = snapshot();
            assert_eq!(sample["pid"], std::process::id());
            assert!(sample["timestamp"].as_str().is_some());
            assert!(
                sample["process"]["memory"]["working_set_bytes"]
                    .as_u64()
                    .unwrap()
                    > 0
            );
            assert!(
                sample["process"]["memory"]["private_bytes"]
                    .as_u64()
                    .unwrap()
                    > 0
            );
            assert!(sample["process"]["handles"].as_u64().unwrap() > 0);
            assert!(sample["process"]["threads"].as_u64().unwrap() > 0);
            for field in ["gdi", "gdi_peak", "user", "user_peak"] {
                assert!(sample["process"][field].is_u64(), "{field}: {sample}");
            }
            assert!(sample["system"]["physical_total_bytes"].as_u64().unwrap() > 0);
            assert!(
                sample["system"]["commit_limit_bytes"].as_u64().unwrap()
                    >= sample["system"]["commit_bytes"].as_u64().unwrap()
            );
            assert!(sample["display"]["remote_session"].is_boolean());
            assert!(sample["display"]["monitors"].as_i64().unwrap() > 0);
            serde_json::to_string(&sample).unwrap();
        }
        let after = handles();
        assert!(
            after <= before + 2,
            "sampling leaked process handles: {before} -> {after}"
        );
    }
}
