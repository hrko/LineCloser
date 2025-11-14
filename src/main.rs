#![windows_subsystem = "windows"]

use clap::Parser;
use std::time::{Duration, Instant};
use std::thread;
use sysinfo::{System};
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM},
    UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowThreadProcessId, IsWindowVisible, ShowWindow,
        GW_OWNER, SW_HIDE, GetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
    },
};

/// Hides the LINE main window on startup.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CliArgs {
    /// Timeout in seconds to wait for the LINE window.
    #[arg(short, long, default_value_t = 30)]
    timeout: u64,
}

// A struct to pass data to the EnumWindows callback.
struct EnumData {
    pid: u32,
    hwnd: Option<HWND>,
}

fn main() {
    let args = CliArgs::parse();
    let timeout_duration = Duration::from_secs(args.timeout);
    let start_time = Instant::now();

    loop {
        if start_time.elapsed() >= timeout_duration {
            std::process::exit(1); // Timeout
        }

        if let Some(hwnd) = find_line_window() {
            unsafe {
                ShowWindow(hwnd, SW_HIDE);
            }
            std::process::exit(0); // Success
        }

        thread::sleep(Duration::from_millis(500));
    }
}

fn find_line_window() -> Option<HWND> {
    let mut sys = System::new_all();
    sys.refresh_processes();

    let line_pid = sys.processes()
        .values()
        .find(|process| process.name().eq_ignore_ascii_case("LINE.exe"))
        .map(|process| process.pid().as_u32());

    if let Some(pid) = line_pid {
        let mut enum_data = EnumData { pid, hwnd: None };
        let lparam = LPARAM(&mut enum_data as *mut _ as isize);
        unsafe {
            EnumWindows(Some(enum_windows_proc), lparam);
        }
        return enum_data.hwnd;
    }

    None
}

extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let enum_data = unsafe { &mut *(lparam.0 as *mut EnumData) };

    let mut window_pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut window_pid));
    }

    if window_pid == enum_data.pid {
        let is_visible = unsafe { IsWindowVisible(hwnd) };
        let owner = unsafe { GetWindow(hwnd, GW_OWNER) };
        let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
        let is_tool_window = (ex_style & WS_EX_TOOLWINDOW.0 as isize) != 0;

        if is_visible.as_bool() && owner.is_err() && !is_tool_window {
            enum_data.hwnd = Some(hwnd);
            return BOOL(0); // Stop enumeration
        }
    }

    BOOL(1) // Continue enumeration
}