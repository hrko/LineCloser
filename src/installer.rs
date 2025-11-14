use directories::ProjectDirs;
use eframe::{egui, NativeOptions};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use winreg::enums::*;
use winreg::RegKey;

const APP_NAME: &str = "LineCloser";

struct InstallerApp {
    timeout_sec: String,
    status_message: String,
}

impl Default for InstallerApp {
    fn default() -> Self {
        Self {
            timeout_sec: "300".to_string(),
            status_message: "".to_string(),
        }
    }
}

impl eframe::App for InstallerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("LineCloser Installer");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Timeout (seconds):");
                ui.text_edit_singleline(&mut self.timeout_sec);
            });

            ui.separator();

            if ui.button("Install / Update").clicked() {
                self.status_message = match self.timeout_sec.parse::<u64>() {
                    Ok(timeout) => match install(timeout) {
                        Ok(path) => format!("Installed/Updated successfully to:\n{}", path.display()),
                        Err(e) => format!("Installation failed: {}", e),
                    },
                    Err(_) => "Invalid timeout value. Please enter a number.".to_string(),
                };
            }

            if ui.button("Uninstall").clicked() {
                self.status_message = match uninstall() {
                    Ok(msg) => msg,
                    Err(e) => format!("Uninstallation failed: {}", e),
                };
                // Quit the app to allow self-deletion
                if self.status_message.starts_with("Uninstalling...") {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }

            ui.separator();
            ui.label(&self.status_message);
        });
    }
}

pub fn run_gui() {
    let native_options = NativeOptions::default();
    eframe::run_native(
        "LineCloser Installer",
        native_options,
        Box::new(|_cc| Ok(Box::new(InstallerApp::default()))),
    )
    .unwrap();
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

    let exe_path = env::current_exe().map_err(|e| format!("Failed to get current exe path: {}", e))?;
    let dest_path = install_dir.join(format!("{}.exe", APP_NAME));

    fs::copy(&exe_path, &dest_path).map_err(|e| format!("Failed to copy executable: {}", e))?;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = PathBuf::from(r"Software\Microsoft\Windows\CurrentVersion\Run");
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
    let path = PathBuf::from(r"Software\Microsoft\Windows\CurrentVersion\Run");

    let key = hkcu
        .open_subkey_with_flags(&path, KEY_WRITE)
        .map_err(|e| format!("Failed to open registry key: {}", e))?;
    key.delete_value(APP_NAME)
        .map_err(|e| format!("Failed to delete registry key (maybe not installed?): {}", e))?;

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

    Ok("Uninstalled successfully (Registry key removed).".to_string())
}
