extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

use std::env;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

use directories::ProjectDirs;
use nwd::NwgUi;
use nwg::NativeUi;
use sha2::{Digest, Sha256};
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

    #[nwg_control(text: "インストール / 変更")]
    #[nwg_layout_item(layout: grid, row: 1, col: 0, col_span: 2)]
    #[nwg_events( OnButtonClick: [InstallerGui::install_clicked] )]
    install_button: nwg::Button,

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
                Ok(path) => format!("インストール/変更が成功しました: {}", path.display()),
                Err(e) => format!("インストールに失敗しました: {}", e),
            },
            Err(_) => "タイムアウトの値が無効です。数値を入力してください。".to_string(),
        };
        log(&status);
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

fn calculate_sha256(path: &PathBuf) -> Result<Vec<u8>, String> {
    let mut file = fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut sha256 = Sha256::new();
    let mut buffer = [0; 1024];
    loop {
        let n = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        if n == 0 {
            break;
        }
        sha256.update(&buffer[..n]);
    }
    Ok(sha256.finalize().to_vec())
}

fn install<F: FnMut(&str)>(timeout: u64, mut log: F) -> Result<PathBuf, String> {
    log("インストール先ディレクトリを取得しています...");
    let install_dir = get_install_path()?;
    log(&format!("  => {}", install_dir.display()));

    log("ディレクトリを作成しています...");
    fs::create_dir_all(&install_dir)
        .map_err(|e| format!("ディレクトリの作成に失敗しました: {}", e))?;

    let exe_path = env::current_exe()
        .map_err(|e| format!("現在の実行ファイルパスの取得に失敗しました: {}", e))?;
    let dest_path = install_dir.join(format!("{}.exe", APP_NAME));

    let mut should_copy = true;
    if dest_path.exists() {
        log("既存のファイルをチェックしています...");
        match (calculate_sha256(&exe_path), calculate_sha256(&dest_path)) {
            (Ok(src_hash), Ok(dest_hash)) => {
                if src_hash == dest_hash {
                    log("実行ファイルは既に最新です。コピーをスキップします。");
                    should_copy = false;
                } else {
                    log("異なるバージョンのため、実行ファイルを上書きします。");
                }
            }
            (Err(e), _) | (_, Err(e)) => {
                log(&format!(
                    "ハッシュの計算に失敗しました ({}). ファイルを上書きします。",
                    e
                ));
            }
        }
    }

    if should_copy {
        log("実行ファイルをコピーしています...");
        log(&format!(
            "  {} -> {}",
            exe_path.display(),
            dest_path.display()
        ));
        fs::copy(&exe_path, &dest_path)
            .map_err(|e| format!("実行ファイルのコピーに失敗しました: {}", e))?;
    }

    log("アンインストールスクリプトを作成しています...");
    let uninstall_script_path = install_dir.join("uninstall.ps1");
    let uninstall_script = format!(
        r#"
Write-Host "Uninstalling LineCloser..."
Start-Sleep -Seconds 1

Write-Host "Stopping running LineCloser process..."
Get-Process -Name "{0}" -ErrorAction SilentlyContinue | Stop-Process -Force

Write-Host "Removing startup registry key..."
$runPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
if (Test-Path $runPath) {{
    Remove-ItemProperty -Path $runPath -Name "{0}" -ErrorAction SilentlyContinue
}}

Write-Host "Removing uninstall registry key..."
$uninstallPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{0}"
if (Test-Path $uninstallPath) {{
    Remove-Item -Path $uninstallPath -Recurse -Force
}}

Write-Host "Removing installation directory: {1}"
Remove-Item -Path "{1}" -Recurse -Force

Write-Host ""
Write-Host "LineCloser has been uninstalled successfully."
Write-Host "Press Enter to exit."
Read-Host | Out-Null
"#,
        APP_NAME,
        install_dir.display()
    );
    fs::write(&uninstall_script_path, uninstall_script)
        .map_err(|e| format!("アンインストールスクリプトの作成に失敗しました: {}", e))?;

    log("アンインストール情報をレジストリに登録しています...");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let uninstall_key_path = format!(
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{}",
        APP_NAME
    );
    let (key, _) = hkcu.create_subkey(&uninstall_key_path).map_err(|e| {
        format!(
            "アンインストール用レジストリキーの作成に失敗しました: {}",
            e
        )
    })?;

    key.set_value("DisplayName", &APP_NAME)
        .map_err(|e| format!("レジストリ値の設定に失敗しました: {}", e))?;
    key.set_value("DisplayVersion", &"0.1.6")
        .map_err(|e| format!("レジストリ値の設定に失敗しました: {}", e))?;
    key.set_value("Publisher", &"hrko")
        .map_err(|e| format!("レジストリ値の設定に失敗しました: {}", e))?;
    let uninstall_command = format!(
        "powershell.exe -ExecutionPolicy Bypass -File \"{}\"",
        uninstall_script_path.display()
    );
    key.set_value("UninstallString", &uninstall_command)
        .map_err(|e| format!("レジストリ値の設定に失敗しました: {}", e))?;
    key.set_value("DisplayIcon", &dest_path.to_str().unwrap_or(""))
        .map_err(|e| format!("レジストリ値の設定に失敗しました: {}", e))?;
    key.set_value("ModifyPath", &dest_path.to_str().unwrap_or(""))
        .map_err(|e| format!("レジストリ値の設定に失敗しました: {}", e))?;

    log("スタートアップに登録しています...");
    let run_key_path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let (key, _) = hkcu
        .create_subkey(&run_key_path)
        .map_err(|e| format!("レジストリキーの作成またはオープンに失敗しました: {}", e))?;

    let command = format!("\"{}\" --timeout {}", dest_path.display(), timeout);
    key.set_value(APP_NAME, &command)
        .map_err(|e| format!("レジストリ値の設定に失敗しました: {}", e))?;

    Ok(dest_path)
}
