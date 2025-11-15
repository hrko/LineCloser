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
    #[nwg_control(size: (600, 240), position: (300, 300), title: "LineCloser インストーラー", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [nwg::stop_thread_dispatch()] )]
    window: nwg::Window,

    #[nwg_layout(parent: window, spacing: 4, margin: [8, 8, 8, 8])]
    grid: nwg::GridLayout,

    #[nwg_control(text: "タイムアウト (秒):")]
    #[nwg_layout_item(layout: grid, row: 0, col: 0)]
    timeout_label: nwg::Label,

    #[nwg_control(text: "300")]
    #[nwg_layout_item(layout: grid, row: 0, col: 1)]
    timeout_input: nwg::TextInput,

    #[nwg_control(text: "インストール / 更新")]
    #[nwg_layout_item(layout: grid, row: 1, col: 0)]
    #[nwg_events( OnButtonClick: [InstallerGui::install_clicked] )]
    install_button: nwg::Button,

    #[nwg_control(text: "アンインストール")]
    #[nwg_layout_item(layout: grid, row: 1, col: 1)]
    #[nwg_events( OnButtonClick: [InstallerGui::uninstall_clicked] )]
    uninstall_button: nwg::Button,

    #[nwg_control(text: "", readonly: true)]
    #[nwg_layout_item(layout: grid, row: 2, col: 0, row_span: 4, col_span: 2)]
    log_box: nwg::TextBox,
}

impl InstallerGui {
    fn install_clicked(&self) {
        self.log_box.set_text("");
        let mut log = |msg: &str| {
            let current_text = self.log_box.text();
            if current_text.is_empty() {
                self.log_box.set_text(msg);
            } else {
                self.log_box
                    .set_text(&format!("{}\r\n{}", current_text, msg));
            }
            self.log_box.scroll_lastline();
        };

        let status = match self.timeout_input.text().parse::<u64>() {
            Ok(timeout) => match install(timeout, &mut log) {
                Ok(path) => format!("インストール/更新が成功しました: {}", path.display()),
                Err(e) => format!("インストールに失敗しました: {}", e),
            },
            Err(_) => "タイムアウトの値が無効です。数値を入力してください。".to_string(),
        };
        log(&status);
    }

    fn uninstall_clicked(&self) {
        self.log_box.set_text("");
        let mut log = |msg: &str| {
            let current_text = self.log_box.text();
            if current_text.is_empty() {
                self.log_box.set_text(msg);
            } else {
                self.log_box
                    .set_text(&format!("{}\r\n{}", current_text, msg));
            }
            self.log_box.scroll_lastline();
        };

        let status = match uninstall(&mut log) {
            Ok(msg) => msg,
            Err(e) => format!("アンインストールに失敗しました: {}", e),
        };
        log(&status);

        if status.starts_with("アンインストールしています...") {
            // Quit the app to allow self-deletion
            nwg::stop_thread_dispatch();
        }
    }
}

pub fn run_gui() {
    nwg::init().expect("Failed to init Native Windows GUI");
    let mut font = nwg::Font::default();
    nwg::Font::builder()
        .size(17)
        .family("Segoe UI")
        .build(&mut font)
        .expect("Failed to build font");
    nwg::Font::set_global_default(Some(font));
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

fn install<'a, F: FnMut(&str)>(timeout: u64, mut log: F) -> Result<PathBuf, String> {
    log("インストール先ディレクトリを取得しています...");
    let install_dir = get_install_path()?;
    log(&format!("  => {}", install_dir.display()));

    log("ディレクトリを作成しています...");
    fs::create_dir_all(&install_dir)
        .map_err(|e| format!("ディレクトリの作成に失敗しました: {}", e))?;

    let exe_path = env::current_exe()
        .map_err(|e| format!("現在の実行ファイルパスの取得に失敗しました: {}", e))?;
    let dest_path = install_dir.join(format!("{}.exe", APP_NAME));

    log("実行ファイルをコピーしています...");
    log(&format!(
        "  {} -> {}",
        exe_path.display(),
        dest_path.display()
    ));
    fs::copy(&exe_path, &dest_path)
        .map_err(|e| format!("実行ファイルのコピーに失敗しました: {}", e))?;

    log("スタートアップに登録しています...");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let (key, _) = hkcu
        .create_subkey(&path)
        .map_err(|e| format!("レジストリキーの作成またはオープンに失敗しました: {}", e))?;

    let command = format!("\"{}\" --timeout {}", dest_path.display(), timeout);
    key.set_value(APP_NAME, &command)
        .map_err(|e| format!("レジストリ値の設定に失敗しました: {}", e))?;

    Ok(dest_path)
}

fn uninstall<'a, F: FnMut(&str)>(mut log: F) -> Result<String, String> {
    log("スタートアップ登録を解除しています...");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Run";

    // Try to remove registry key
    if let Ok(key) = hkcu.open_subkey_with_flags(path, KEY_WRITE) {
        // Ignore error if value doesn't exist
        let _ = key.delete_value(APP_NAME);
        log("  => レジストリキーを削除しました (存在する場合)");
    } else {
        log("  => レジストリキーが見つからないか、開けませんでした");
    }

    log("インストール先ディレクトリを取得しています...");
    let install_dir = get_install_path()?;
    let dest_path = install_dir.join(format!("{}.exe", APP_NAME));
    log(&format!("  => {}", install_dir.display()));

    if dest_path.exists() {
        log("自己削除プロセスを起動しています...");
        // Self-delete by spawning a command prompt
        let script = format!(
            "timeout /t 2 /nobreak > NUL && del \"{}\" && rmdir \"{}\"",
            dest_path.display(),
            install_dir.display()
        );
        Command::new("cmd")
            .args(&["/C", &script])
            .spawn()
            .map_err(|e| format!("自己削除プロセスの起動に失敗しました: {}", e))?;

        return Ok("アンインストールしています... アプリケーションを終了します。".to_string());
    }

    Ok("アンインストールが成功しました (またはインストールされていませんでした)。".to_string())
}
