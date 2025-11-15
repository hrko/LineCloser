#![windows_subsystem = "windows"]

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

use directories::ProjectDirs;
use nwd::NwgUi;
use nwg::NativeUi;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use winreg::enums::*;
use winreg::RegKey;

const APP_NAME: &str = "LineCloser";

#[derive(Default, NwgUi)]
pub struct InstallerGui {
    #[nwg_control(size: (600, 200), position: (300, 300), title: "LineCloser Installer", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [nwg::stop_thread_dispatch()] )]
    window: nwg::Window,

    #[nwg_layout(parent: window, spacing: 5, margin: [10, 10, 10, 10])]
    grid: nwg::GridLayout,

    #[nwg_control(text: "Timeout (seconds):")]
    #[nwg_layout_item(layout: grid, row: 0, col: 0)]
    timeout_label: nwg::Label,

    #[nwg_control(text: "300")]
    #[nwg_layout_item(layout: grid, row: 0, col: 1)]
    timeout_input: nwg::TextInput,

    #[nwg_control(text: "Install / Update")]
    #[nwg_layout_item(layout: grid, row: 1, col: 0)]
    #[nwg_events( OnButtonClick: [InstallerGui::install_clicked] )]
    install_button: nwg::Button,

    #[nwg_control(text: "Uninstall")]
    #[nwg_layout_item(layout: grid, row: 1, col: 1)]
    #[nwg_events( OnButtonClick: [InstallerGui::uninstall_clicked] )]
    uninstall_button: nwg::Button,

    #[nwg_control(text: "", readonly: true)]
    #[nwg_layout_item(layout: grid, row: 2, col: 0, row_span: 2, col_span: 2)]
    status_label: nwg::TextBox,
}

impl InstallerGui {
    fn install_clicked(&self) {
        let status = match self.timeout_input.text().parse::<u64>() {
            Ok(timeout) => match install(timeout) {
                Ok(path) => format!("Installed/Updated successfully to:\n{}", path.display()),
                Err(e) => format!("Installation failed: {}", e),
            },
            Err(_) => "Invalid timeout value. Please enter a number.".to_string(),
        };
        self.status_label.set_text(&status);
    }

    fn uninstall_clicked(&self) {
        let status = match uninstall() {
            Ok(msg) => msg,
            Err(e) => format!("Uninstallation failed: {}", e),
        };
        self.status_label.set_text(&status);

        if status.starts_with("Uninstalling...") {
            // Quit the app to allow self-deletion
            nwg::stop_thread_dispatch();
        }
    }
}

pub fn run_gui() {
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");
    let _app = InstallerGui::build_ui(Default::default()).expect("Failed to build UI");
    nwg::dispatch_thread_events();
}

fn get_install_path() -> Result<PathBuf, String> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "hrko", APP_NAME) {
        Ok(proj_dirs.data_dir().to_path_buf())
    } else {
        Err("Could not find user's data directory.".to_string())
    }
}

fn install(timeout: u64) -> Result<PathBuf, String> {
    let install_dir = get_install_path()?;
    fs::create_dir_all(&install_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    let exe_path =
        env::current_exe().map_err(|e| format!("Failed to get current exe path: {}", e))?;
    let dest_path = install_dir.join(format!("{}.exe", APP_NAME));

    fs::copy(&exe_path, &dest_path).map_err(|e| format!("Failed to copy executable: {}", e))?;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let (key, _) = hkcu
        .create_subkey(&path)
        .map_err(|e| format!("Failed to create or open registry key: {}", e))?;

    let command = format!("\"{}\" --timeout {}", dest_path.display(), timeout);
    key.set_value(APP_NAME, &command)
        .map_err(|e| format!("Failed to set registry value: {}", e))?;

    Ok(dest_path)
}

fn uninstall() -> Result<String, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Run";

    // Try to remove registry key
    if let Ok(key) = hkcu.open_subkey_with_flags(path, KEY_WRITE) {
        // Ignore error if value doesn't exist
        let _ = key.delete_value(APP_NAME);
    }

    let install_dir = get_install_path()?;
    let dest_path = install_dir.join(format!("{}.exe", APP_NAME));

    if dest_path.exists() {
        // Self-delete by spawning a command prompt
        let script = format!(
            "timeout /t 2 /nobreak > NUL && del \"{}\" && rmdir \"{}\"",
            dest_path.display(),
            install_dir.display()
        );
        Command::new("cmd")
            .args(&["/C", &script])
            .spawn()
            .map_err(|e| format!("Failed to spawn self-delete process: {}", e))?;

        return Ok("Uninstalling... The application will now close.".to_string());
    }

    Ok("Uninstalled successfully (or was not installed).".to_string())
}
