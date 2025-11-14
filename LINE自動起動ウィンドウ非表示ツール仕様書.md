

# **\[仕様書\] Windows版LINE 起動時ウィンドウ自動非表示化ツール（"LineCloser"）**

---

## **1.0 仕様概要**

### **1.1 目的と背景**

本仕様書は、Windowsオペレーティングシステムにおいて、LINEデスクトップアプリケーションがユーザーログイン（スタートアップ）時にメインウィンドウを不必要に表示する挙動を是正するための、Rust製ユーティリティツール（以下、本ツール）の設計と仕様を定義する。

ユーザーの要求 は、LINEがタスクトレイに格納された状態（多くのアプリケーションが取る「トレイへの最小化」）で起動せず、メインウィンドウがデスクトップに表示されてしまう問題を解決することにある。

本ツールは、このLINEのメインウィンドウを能動的に監視し、検知次第、プロセスを終了させることなくウィンドウのみを非表示（$SW\\\_HIDE$）にし、LINEがタスクトレイでのみ動作する状態を実現する。

### **1.2 アーキテクチャ概要**

本ツールは、以下の3つの主要コンポーネントで構成される軽量なRust実行ファイル（.exe）として設計される。

1. **CLI（コマンドラインインターフェース）層:** clap クレート 1 を利用し、タイムアウト時間の設定など、外部からの設定を受け付ける。  
2. **プロセス監視層:** sysinfo クレート 3 を利用し、ターゲットプロセス（LINE.exe）のプロセスID ($PID$) を特定する。  
3. **ウィンドウ制御層:** windows-rs クレート 5 を通じてネイティブWin32 API（$EnumWindows$, $GetWindowThreadProcessId$, $ShowWindow$ 等）を呼び出し、$PID$ に紐づくメインウィンドウ（$HWND$）を特定し、非表示化する。

### **1.3 実行ライフサイクル**

本ツールは、Windowsのスタートアップに登録される。実行されると、本ツール自体はコンソールウィンドウを表示せず 7、バックグラウンドで動作する。

1. **起動:** ツールが起動し、$std::time::Instant$ 9 でタイマーを開始する。  
2. **ポーリング:** タイムアウト時間（CLIフラグで指定）に達するまで、定期的なポーリングループ 9 を実行する。  
3. **探索:** ループ内でLINEのプロセス（$PID$）とメインウィンドウ（$HWND$）の探索を試みる。  
4. **実行:** $HWND$ が特定できた場合、$ShowWindow(hwnd, SW\\\_HIDE)$ 10 を実行し、ウィンドウを非表示にする。  
5. **終了:** ウィンドウの非表示化に成功した場合、$std::process::exit(0)$ で正常終了する。タイムアウト時間内に $HWND$ を特定できなかった場合は $std::process::exit(1)$ で異常終了（タイムアウト）する。

---

## **2.0 技術スタックとプロジェクト構成**

### **2.1 依存クレート（Cargo.toml）**

本ツールの実装には、以下のRustクレートを必須とする。

| クレート名 | バージョン（推奨） | 用途 | 参照 |
| :---- | :---- | :---- | :---- |
| clap | 4.x | コマンドライン引数の解析（$--timeout$ の実装）。Derive機能（features \= \["derive"\]）を必須とする。 | 1 |
| sysinfo | 0.30.x | システム情報の取得。特にプロセス名（LINE.exe）からプロセスID ($PID$) を特定するために使用する。 | 3 |
| windows | 0.58.x | Microsoftによる公式Rustバインディング。Win32 APIの呼び出し（$EnumWindows$, $ShowWindow$ 等）に使用する。 | 5 |

### **2.2 windows-rs クレートのフィーチャー構成**

windows クレート 5 は、コンパイル時間とバイナリサイズを最適化するため、必要なAPIごとにフィーチャーフラグを明示的に指定する必要がある 15。本ツールの要求仕様に基づき、以下のフィーチャーが**最低限必要**となる。

* **Win32\_Foundation**: $HWND$, $BOOL$, $LPARAM$ などの基本的な型定義に必要 5。  
* **Win32\_UI\_WindowsAndMessaging**: ウィンドウ操作（$EnumWindows$, $GetWindowThreadProcessId$, $ShowWindow$, $IsWindowVisible$, $GetWindow$）のコアAPI群に必要 5。

したがって、Cargo.toml の dependencies.windows セクションは以下のように構成する。

Ini, TOML

\[dependencies.windows\]  
version \= "0.58"  
features \=

### **2.3 ビルド構成：コンソールウィンドウの非表示化**

* **仕様:** 本ツールはバックグラウンドユーティリティであり、実行時にコンソールウィンドウ（黒い画面）を表示してはならない。  
* **実装:** Rust 1.18以降でサポートされている $windows\\\_subsystem$ 属性をクレートのルート（通常 main.rs の先頭）に記述する 7。  
* **コード:** \#\!\[windows\_subsystem \= "windows"\]  
* 技術的考察:  
  ユーザー は、LINEのウィンドウ表示という「UIの妨害」を解決しようとしている。もし本ツール自体が起動時に一瞬でもコンソールウィンドウを表示させた場合、それは元の問題を別の問題に置き換えただけであり、ソリューションとして不完全である。  
  \#\!\[windows\_subsystem \= "windows"\] 7 は、OSに対してこの実行ファイルがCUI (Console UI) アプリケーションではなく、GUI (Graphical UI) アプリケーションであることを伝えるリンカディレクティブである。これにより、OSは本プロセスのためにコンソールセッションを割り当てることをスキップする。本ツールはウィンドウを一切作成しないため、結果として完全に不可視（サイレント）なバックグラウンドプロセスとして実行される。これは、ユーザー要件の「タスクトレイに格納した状態（を実現するツール）」という文脈から暗に要求される、最も重要な技術仕様の一つである。

---

## **3.0 コマンドラインインターフェース（CLI）仕様**

### **3.1 clap による引数定義**

$clap::Parser$ 2 のDerive API 16 を用いて、以下の構造体を定義する。

Rust

use clap::Parser;

\#  
\#\[command(version, about \= "Hides the LINE main window on startup.")\]  
struct CliArgs {  
    /// Timeout in seconds to wait for the LINE window.  
    \#\[arg(short, long, default\_value\_t \= 30)\]  
    timeout: u64,  
}

### **3.2 タイムアウトフラグ (--timeout)**

* **フラグ:** \--timeout  
* **短縮形:** \-t  
* **型:** $u64$ (符号なし64ビット整数)  
* **単位:** 秒  
* **デフォルト値:** 30 (秒)  
  * **根拠:** $default\\\_value\\\_t$ 属性の使用 2。30秒という値は、Windowsの起動が遅い場合や、LINEの自動起動が遅延した場合でも、十分待機できる現実的なデフォルト値として選定する。  
* **必須要件 への準拠:** 「タイムアウトまでの時間はコマンドラインフラグで指定できる（指定無しならデフォルト値）」という要件を完全に満たす。

---

## **4.0 ターゲットウィンドウの探索・特定ロジック**

本ツールの核心的な機能は、LINEの「メインウィンドウ」を正確に特定することにある。本仕様書は、ウィンドウタイトル 17 やウィンドウクラス名 19 といった不安定な要素に依存しない、プロセスID ($PID$) に基づく堅牢な2フェーズ戦略を採用する。

### **4.1 フェーズ1: プロセスID (PID) の特定（sysinfo）**

* **目的:** LINE.exe というプロセス名から、対応するプロセスID ($PID$) を取得する。  
* **使用クレート:** sysinfo 3  
* **アルゴリズム:**  
  1. $sysinfo::System$ のインスタンスを作成する（$System::new\\\_all()$）12。  
  2. $sys.refresh\\\_processes()$ を呼び出し、最新のプロセスリストを読み込む。  
  3. $sys.processes()$ 4 をイテレートする。  
  4. 各 $Process$ オブジェクトの $name()$ 4 を LINE.exe（大文字小文字を区別しない比較が望ましい）と比較する。  
  5. 一致した場合、その $Pid$ を取得し、フェーズ2の入力として渡す。  
* 技術的考察:  
  なぜ sysinfo を使うのか？ windows-rs だけでも、$CreateToolhelp32Snapshot$, $Process32FirstW$, $Process32NextW$ といったWin32 APIを駆使すればプロセス列挙は可能である。  
  しかし、CreateToolhelp32Snapshot を利用するアプローチ（C++の例 21 が示唆するように）は、スナップショットハンドルの管理、$PROCESSENTRY32W$ 構造体の手動イテレーション、ハンドルのクローズ処理など、非常に定型的（ボイラープレート）かつエラーが発生しやすいコードをRustで記述する必要がある。  
  sysinfo クレート 3 は、これらの複雑なWin32呼び出しを、$sys.processes().find(...)$ のような安全で高レベルなイディオムに抽象化する。本ツールの主目的はウィンドウ制御であり、プロセス列挙は前提条件に過ぎない。したがって、sysinfo という依存関係を追加するコストは、コードの可読性、保守性、および安全性が劇的に向上するというメリットによって十分に正当化される。

### **4.2 フェーズ2: ウィンドウ列挙とメインウィンドウ検証（windows-rs）**

* **目的:** フェーズ1で特定した$PID$に紐づき、かつ「メインウィンドウ」の条件を満たす $HWND$ を特定する。  
* **使用API:** $EnumWindows$, $GetWindowThreadProcessId$, $IsWindowVisible$, $GetWindow$  
* **アルゴリズム:**  
  1. $EnumWindows$ 22 を呼び出す。この際、フェーズ1で取得したLINEの $Pid$ と、結果を格納する $Option\<HWND\>$ をラップしたカスタムデータ構造を、$LPARAM$ 24 としてコールバック関数に渡す。  
  2. $EnumWindows$ は、存在するすべてのトップレベルウィンドウに対してコールバック関数（WindowEnumCallback）を実行する。  
  3. WindowEnumCallback(hwnd: HWND, lparam: LPARAM) の内部ロジック:  
     a. $lparam$ からLINEの $Pid$ と結果格納用の $\\\&mut Option\<HWND\>$ をデコードする（24 のパターン参照）。  
     b. $GetWindowThreadProcessId(hwnd,...)$ 26 を呼び出し、$hwnd$ を作成したプロセスのPID（$window\\\_pid$）を取得する。  
     c. 検証1（PID一致）: $window\\\_pid$ がLINEの $Pid$ と一致するか確認する。一致しなければ、$BOOL(1)$（true）を返し、列挙を続行する。  
     d. 検証2（可視性）: $IsWindowVisible(hwnd)$ 27 を呼び出す。$false$（非可視）であれば、それはメインウィンドウではない（例：バックグラウンドヘルパーウィンドウ）と判断し、$BOOL(1)$ を返し列挙を続行する。  
     e. 検証3（オーナー）: $GetWindow(hwnd, GW\\\_OWNER)$ 27 を呼び出す。戻り値が $HWND(0)$（NULL）でない場合、そのウィンドウは別のウィンドウに所有されている（例：ダイアログボックス、スプラッシュスクリーン）と判断し 25、$BOOL(1)$ を返し列挙を続行する。  
     f. 検証4（ツールウィンドウではないこと）: $GetWindowLong(hwnd, GWL\\\_EXSTYLE)$ 12 を呼び出し、拡張ウィンドウスタイルを取得する。取得したスタイルに $WS\\\_EX\\\_TOOLWINDOW$ フラグが含まれていないことを確認する。含まれていれば、$BOOL(1)$ を返し列挙を続行する。  
     g. 特定成功: 上記4つの検証をすべて通過した $hwnd$ は、LINEの可視トップレベル・非所有・非ツールウィンドウ、すなわち「メインウィンドウ」であると断定する。  
     h. $lparam$ 経由で $\\\&mut Option\<HWND\>$ に $Some(hwnd)$ をセットする。  
     i. $BOOL(0)$（false）を返し、$EnumWindows$ の列挙を即座に停止する 25。  
  4. $EnumWindows$ の呼び出し元は、$lparam$ 経由で渡した $Option\<HWND\>$ を確認し、$Some(hwnd)$ が入っていれば特定成功、None であれば特定失敗（今サイクルのポーリングでは見つからなかった）と判断する。  
* 技術的考察:  
  Win32 APIに「メインウィンドウ」という厳密な概念は存在しない 25。プロセスは複数のトップレベルウィンドウを持つことができる。したがって、「メインウィンドウ」の定義はヒューリスティック（経験則）に頼らざるを得ない。  
  議論されている 25 $GetWindow(GW\\\_OWNER) \== 0$ と $IsWindowVisible$ の組み合わせは、このヒューリスティックを実装する上で最も標準的かつ信頼性の高い方法である。本仕様書が採用する「PIDが一致し、可視であり、オーナーを持たない」という3条件の組み合わせは、LINEのスプラッシュスクリーンや通知ポップアップを誤って「メインウィンドウ」として捕捉し、非表示にしてしまうリスクを最小限に抑えるための、工学的に妥当な定義である。

---

## **5.0 ウィンドウ状態の操作（非表示化）**

### **5.1 「閉じる」の定義：WM\_CLOSE の採用**

* **要求分析:** ユーザーは当初「ウィンドウを閉じる」と表現しており、その目的は「タスクトレイに格納した状態」の実現であった。当初はプロセスを終了させない`ShowWindow(SW_HIDE)`が安全策として考えられた。しかし、`SW_HIDE`ではウィンドウを再表示できない問題が確認された。LINEアプリケーションは、メインウィンドウの「閉じる」ボタンが押された際に`WM_CLOSE`メッセージを受け取り、プロセスを終了するのではなくタスクトレイに自身を格納する挙動を取る。このネイティブな挙動を模倣することが、最も堅牢で互換性の高い方法であると判断される。
* **採用API:** `SendMessage` Win32 APIを使用して、`WM_CLOSE` メッセージを送信する。
* **不採用API:** `ShowWindow(SW_HIDE)`。ウィンドウを非表示にはできるが、タスクトレイからの再表示に問題が生じるため棄却する。
* **アルゴリズム:**
  1. 「4.0 ターゲットウィンドウの探索・特定ロジック」で特定した $HWND$（$line\\_hwnd$）を取得する。
  2. `unsafe { SendMessage(line_hwnd, WM_CLOSE, 0, 0) }` を呼び出す。
  3. このメッセージを受け取ったLINEプロセスは、通常の「閉じる」ボタンが押された際と同じ動作（タスクトレイへの格納）を実行する。
* 技術的考察:
  `SendMessage`と`PostMessage`の違いは、同期・非同期性にある。`SendMessage`は、ターゲットウィンドウのウィンドウプロシージャがメッセージを処理し終えるまで制御を返さない（同期的）。一方、`PostMessage`はメッセージキューにメッセージを投稿して即座に制御を返す（非同期的）。本ツールの目的は、ウィンドウが確実に「閉じられた」（タスクトレイに格納された）ことを確認してから自身が終了することであるため、同期的な`SendMessage`の採用がより適切である。これにより、メッセージが処理される前にツールが終了してしまう競合状態を回避できる。

### **5.2 実行と自己終了**

$ShowWindow$ の呼び出しが成功した後、本ツールはその責務を完了したとみなし、即座に $std::process::exit(0)$ （：「そのツール自身も終了する」）を呼び出して正常終了する。

---

## **6.0 実行ライフサイクル（ポーリングとタイムアウト）**

### **6.1 実行フローの定義**

本ツールの main 関数は、以下の制御フローを実行する。

Rust

// main.rs  
// \#\!\[windows\_subsystem \= "windows"\]\[7\]

use clap::Parser;  
//... (windows-rs の use 宣言)...

// (3.0 仕様の CliArgs 構造体定義)

// (4.0 仕様の find\_line\_window() 関数定義)  
//   \- (4.1 sysinfo を使用)  
//   \- (4.2 EnumWindows とコールバックを使用)

fn main() {  
    // 1\. CLI引数の解析 (clap)  
    let args \= CliArgs::parse();  
    let timeout\_duration \= std::time::Duration::from\_secs(args.timeout);

    // 2\. タイムアウトタイマーの開始  
    let start\_time \= std::time::Instant::now(); // \[9, 39\]

    // 3\. ポーリングループの開始  
    loop {  
        // 4\. タイムアウトチェック  
        if start\_time.elapsed() \>= timeout\_duration {  
            // タイムアウト。ログ（将来的な拡張）を残し、異常終了  
            std::process::exit(1);  
        }

        // 5\. ターゲット探索（4.0のロジック）  
        match find\_line\_window() { // \`find\_line\_window\` は 4.0 仕様の実装  
            Some(line\_hwnd) \=\> {  
                // 6\. 発見時：ウィンドウを非表示にする（5.0のロジック）  
                unsafe {  
                    // windows::Win32::UI::WindowsAndMessaging::ShowWindow;  
                    // windows::Win32::UI::WindowsAndMessaging::SW\_HIDE;  
                    ShowWindow(line\_hwnd, SW\_HIDE); //   
                }  
                // 7\. 成功：正常終了  
                std::process::exit(0);  
            }  
            None \=\> {  
                // 8\. 未発見時：ポーリング間隔を空ける  
                std::thread::sleep(std::time::Duration::from\_millis(500)); // \[40, 41\]  
            }  
        }  
    }  
}

### **6.2 ポーリング間隔**

* **仕様:** ループの各反復の終わり（ターゲット未発見時）に、$std::thread::sleep$ 40 を呼び出す。  
* **間隔:** 500ミリ秒。  
* **根拠:** CPUリソースの過剰な消費を防ぎつつ 9、LINEウィンドウの出現に迅速（最悪でも0.5秒遅れ）に対応可能な、バランスの取れた値として選定する。

### **6.3 タイムアウト**

* **仕様:** 起動時に $std::time::Instant::now()$ 9 で開始時刻を記録。ループの先頭で $start\\\_time.elapsed()$ 41 をチェックし、CLIで指定された $timeout\\\_duration$ を超えていた場合は、$std::process::exit(1)$ でタイムアウト終了する。  
* **根拠:** の「一定時間内にLINEのウィンドウを確認できなければ、タイムアウトして終了する」という要件を満たす。これにより、LINEが起動しなかった場合や、LINEのアップデート等で LINE.exe の名前が変わった場合に、本ツールが無限にリソースを消費し続けることを防ぐ。

### **6.4 技術的考察：ポーリング vs イベントフック**

なぜCPU（たとえ低負荷でも）を消費するポーリング 9 を選ぶのか？ Win32 APIには $SetWindowsHookEx$ を用いたグローバルイベントフック（例：$WH\\\_SHELL$ で $HSHELL\\\_WINDOWCREATED$ を補足する）という、より「イベント駆動」的なアプローチが存在する。

しかし、グローバルイベントフックは、以下の深刻な欠点を持つ。

1. **複雑性:** $SetWindowsHookEx$ は、しばしばDLLインジェクションを伴うか、複雑なプロセス間メッセージングを要求され、実装が極めて困難になる。  
2. **セキュリティ:** 他のプロセスのイベントを監視するため、アンチウイルスソフトウェアによって脅威としてフラグ付けされるリスクが非常に高い。  
3. **堅牢性:** フックプロシージャがクラッシュすると、システム全体が不安定になる可能性がある。

本ツールの要件は「スタートアップ時に起動するLINE」という非常に限定された時間枠（起動後30～60秒）でのみ動作すればよい。この限定されたライフサイクルにおいては、ポーリングは圧倒的にシンプルかつ安全で、実装・デバッグが容易なソリューションである。$std::thread::sleep$ 40 を適切に使用すれば、システムへの負荷は無視できるレベルに抑えられる。したがって、本仕様書は、工学的な「単純性」と「堅牢性」を優先し、ポーリング方式を意図的に選択する。

---

## **7.0 デプロイメント（導入）手順**

本仕様書は、ツールの *機能* を定義するものであり、インストーラーの作成はスコープ外とする。ユーザー（開発者）は、以下の手順で本ツールをデプロイ（配備）する必要がある。

### **7.1 ビルド**

1. Rustプロジェクトを cargo build \--release でコンパイルする。  
2. target/release/ ディレクトリに生成された実行ファイル（例：line\_closer.exe）を取得する。

### **7.2 スタートアップへの登録（推奨）**

* **方法:** ユーザー個別の「スタートアップ」フォルダに、実行ファイルのショートカットを配置する。  
* **手順:**  
  1. Windows \+ R キーを押し、「ファイル名を指定して実行」ダイアログを開く。  
  2. shell:startup 43 と入力し、Enterキーを押す。  
  3. ユーザーのスタートアップフォルダ（例：%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs\\Startup 44）が開く。  
  4. line\_closer.exe へのショートカットを、このフォルダに配置する。  
* **利点:** 導入と削除が容易であり、特別な管理者権限を必要としない。

### **7.3 （代替）レジストリによる登録**

* **方法:** Windowsレジストリの Run キーに登録する。  
* **キー:** HKEY\_CURRENT\_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run 48  
* **手順:**  
  1. regedit などを起動する。  
  2. 上記のキーに移動する。  
  3. 新しい「文字列値」を作成する（例：LineCloser）。  
  4. その値のデータとして、line\_closer.exe へのフルパス（および必要な \--timeout 引数）を書き込む（Rustでレジストリを操作するクレートの例 52）。  
* **利点:** shell:startup フォルダよりも「隠れた」登録が可能だが、管理が煩雑になる可能性がある。  
* 技術的考察：ショートカット作成の自動化（IShellLink）の棄却  
  Rustプログラム自身に、shell:startup フォルダへショートカット（.lnk）を自動作成させることはできないか？  
  Windowsで .lnk ファイルを作成するには、Win32 COMインターフェース、特に $IShellLinkW$ 54 を使用する必要がある。RustからCOMを扱うのは非常に複雑である 55。lnk 58 や mslnk 59 といった純Rustクレートも存在するが、議論 60 によれば、書き込み機能が不安定であったり、機能が限定的であったりする。  
  本ツールのコア機能（ウィンドウの非表示化）はCOMを必要としない。インストーラーのような「利便性」機能のために、COMという巨大な技術的負債と不安定要素を導入することは、ツールのシンプルさという設計思想に反する。したがって、デプロイメント（ショートカットの配置）は、ユーザー（開発者）の手動操作に委ねるのが最も堅牢な仕様である。

---

## **8.0 結論と将来の拡張性**

本仕様書は、RustとWin32 APIを組み合わせて、Windows版LINEの起動時ウィンドウを自動的に非表示にするための、堅牢かつ軽量なユーティリティツールの設計を定義した。

中核的な設計判断は以下の通りである。

1. **ターゲット特定:** 不安定なウィンドウタイトルやクラス名ではなく、sysinfo と windows-rs を組み合わせた「$PID$ベースのメインウィンドウ検証」を採用した。  
2. **ウィンドウ操作:** プロセスを終了する $WM\\\_CLOSE$ ではなく、タスクトレイ動作の挙動に合致する $ShowWindow(SW\\\_HIDE)$ を採用した。  
3. **ライフサイクル:** グローバルフックの複雑性と危険性を避け、$std::time::Instant$ と $std::thread::sleep$ を用いた、タイムアウト付きのシンプルなポーリングループを採用した。  
4. **不可視性:** \#\!\[windows\_subsystem \= "windows"\] を使用し、ツール自体のコンソールウィンドウを抑制、シームレスなユーザー体験を実現する。

**将来的な拡張案（本仕様のスコープ外）:**

* **ロギング:** $windows\\\_subsystem$ は $stdout$ を無効化するため、デバッグが困難になる。log クレートと simple\_logger などを組み合わせ、ファイルベースのロギング（例：タイムアウト時やエラー時にログを書き出す）を実装することが考えられる。  
* **ターゲットの一般化:** LINE.exe というハードコードされたプロセス名を、CLI引数（例：--process-name "another.exe"）で指定可能にし、汎用的なウィンドウ非表示化ツールとして拡張する。  
* **設定ファイル:** ポーリング間隔やターゲットプロセス名をTOMLファイルなどで管理できるようにする。

#### **引用文献**

1. Getting Started with Clap: A Beginner's Guide to Rust CLI Apps \- DEV Community, 11月 15, 2025にアクセス、 [https://dev.to/moseeh\_52/getting-started-with-clap-a-beginners-guide-to-rust-cli-apps-1n3f](https://dev.to/moseeh_52/getting-started-with-clap-a-beginners-guide-to-rust-cli-apps-1n3f)  
2. clap \- Rust \- Docs.rs, 11月 15, 2025にアクセス、 [https://docs.rs/clap](https://docs.rs/clap)  
3. sysinfo \- crates.io: Rust Package Registry, 11月 15, 2025にアクセス、 [https://crates.io/crates/sysinfo](https://crates.io/crates/sysinfo)  
4. Process in sysinfo \- Rust \- Docs.rs, 11月 15, 2025にアクセス、 [https://docs.rs/sysinfo/latest/sysinfo/struct.Process.html](https://docs.rs/sysinfo/latest/sysinfo/struct.Process.html)  
5. Find Window Handle (HWND) using Rust \- DEV Community, 11月 15, 2025にアクセス、 [https://dev.to/raeisi/find-window-handle-hwnd-using-rust-1dfj](https://dev.to/raeisi/find-window-handle-hwnd-using-rust-1dfj)  
6. microsoft/windows-rs: Rust for Windows \- GitHub, 11月 15, 2025にアクセス、 [https://github.com/microsoft/windows-rs](https://github.com/microsoft/windows-rs)  
7. How to make a program that does not display the console window? \- Stack Overflow, 11月 15, 2025にアクセス、 [https://stackoverflow.com/questions/29763647/how-to-make-a-program-that-does-not-display-the-console-window](https://stackoverflow.com/questions/29763647/how-to-make-a-program-that-does-not-display-the-console-window)  
8. How do I disable the terminal from opening up when executing my Rust code? · Issue \#1016 · microsoft/windows-rs \- GitHub, 11月 15, 2025にアクセス、 [https://github.com/microsoft/windows-rs/issues/1016](https://github.com/microsoft/windows-rs/issues/1016)  
9. How do you make an infinite loop in Rust and make it run with a delay? \- Rust Users Forum, 11月 15, 2025にアクセス、 [https://users.rust-lang.org/t/how-do-you-make-an-infinite-loop-in-rust-and-make-it-run-with-a-delay/80296](https://users.rust-lang.org/t/how-do-you-make-an-infinite-loop-in-rust-and-make-it-run-with-a-delay/80296)  
10. ShowWindow function (winuser.h) \- Win32 apps | Microsoft Learn, 11月 15, 2025にアクセス、 [https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow)  
11. Clap \- default values for CLI command line parameters in Rust, 11月 15, 2025にアクセス、 [https://rust.code-maven.com/clap-default-values](https://rust.code-maven.com/clap-default-values)  
12. Rust Process List Update \- CodePal, 11月 15, 2025にアクセス、 [https://codepal.ai/code-generator/query/wPKaCYJE/rust-code-to-get-process-list-and-update-on-change](https://codepal.ai/code-generator/query/wPKaCYJE/rust-code-to-get-process-list-and-update-on-change)  
13. GetProcessId by name in Rust \- help \- The Rust Programming Language Forum, 11月 15, 2025にアクセス、 [https://users.rust-lang.org/t/getprocessid-by-name-in-rust/46056](https://users.rust-lang.org/t/getprocessid-by-name-in-rust/46056)  
14. Rust for Windows, and the windows crate \- Microsoft Learn, 11月 15, 2025にアクセス、 [https://learn.microsoft.com/en-us/windows/dev-environment/rust/rust-for-windows](https://learn.microsoft.com/en-us/windows/dev-environment/rust/rust-for-windows)  
15. Rust get executable path of process by window handle or process id \- Stack Overflow, 11月 15, 2025にアクセス、 [https://stackoverflow.com/questions/76496834/rust-get-executable-path-of-process-by-window-handle-or-process-id](https://stackoverflow.com/questions/76496834/rust-get-executable-path-of-process-by-window-handle-or-process-id)  
16. clap::\_derive::\_tutorial \- Rust \- Docs.rs, 11月 15, 2025にアクセス、 [https://docs.rs/clap/latest/clap/\_derive/\_tutorial/index.html](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html)  
17. Title bar (design) \- Windows \- Microsoft Learn, 11月 15, 2025にアクセス、 [https://learn.microsoft.com/en-us/windows/apps/design/basics/titlebar-design](https://learn.microsoft.com/en-us/windows/apps/design/basics/titlebar-design)  
18. Find all window titles of application through command line \- Super User, 11月 15, 2025にアクセス、 [https://superuser.com/questions/1328345/find-all-window-titles-of-application-through-command-line](https://superuser.com/questions/1328345/find-all-window-titles-of-application-through-command-line)  
19. About Window Classes \- Win32 apps | Microsoft Learn, 11月 15, 2025にアクセス、 [https://learn.microsoft.com/en-us/windows/win32/winmsg/about-window-classes](https://learn.microsoft.com/en-us/windows/win32/winmsg/about-window-classes)  
20. net class name from a window handle \- Stack Overflow, 11月 15, 2025にアクセス、 [https://stackoverflow.com/questions/10850485/net-class-name-from-a-window-handle](https://stackoverflow.com/questions/10850485/net-class-name-from-a-window-handle)  
21. \[RESOLVED\] How to get window's HWND from it's process handle?, 11月 15, 2025にアクセス、 [https://forums.codeguru.com/showthread.php?392273-RESOLVED-How-to-get-window-s-HWND-from-it-s-process-handle](https://forums.codeguru.com/showthread.php?392273-RESOLVED-How-to-get-window-s-HWND-from-it-s-process-handle)  
22. EnumWindows in windows\_win::sys \- Rust \- Docs.rs, 11月 15, 2025にアクセス、 [https://docs.rs/windows-win/latest/windows\_win/sys/fn.EnumWindows.html](https://docs.rs/windows-win/latest/windows_win/sys/fn.EnumWindows.html)  
23. EnumWindows in windows::Win32::UI::WindowsAndMessaging \- Rust, 11月 15, 2025にアクセス、 [https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/fn.EnumWindows.html](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/fn.EnumWindows.html)  
24. hello, i am trying to use windows-rs crate on command::child : r/rust \- Reddit, 11月 15, 2025にアクセス、 [https://www.reddit.com/r/rust/comments/yohm7u/hello\_i\_am\_trying\_to\_use\_windowsrs\_crate\_on/](https://www.reddit.com/r/rust/comments/yohm7u/hello_i_am_trying_to_use_windowsrs_crate_on/)  
25. How to get main window handle from process id? \- Stack Overflow, 11月 15, 2025にアクセス、 [https://stackoverflow.com/questions/1888863/how-to-get-main-window-handle-from-process-id](https://stackoverflow.com/questions/1888863/how-to-get-main-window-handle-from-process-id)  
26. GetWindowThreadProcessId function (winuser.h) \- Win32 apps \- Microsoft Learn, 11月 15, 2025にアクセス、 [https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowthreadprocessid](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowthreadprocessid)  
27. Window Features \- Win32 apps \- Microsoft Learn, 11月 15, 2025にアクセス、 [https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features)  
28. IsWindowVisible function (winuser.h) \- Win32 apps | Microsoft Learn, 11月 15, 2025にアクセス、 [https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-iswindowvisible](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-iswindowvisible)  
29. GetWindow function (winuser.h) \- Win32 apps | Microsoft Learn, 11月 15, 2025にアクセス、 [https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindow)  
30. How can I tell if the Window Handle is main window handle? \- Stack Overflow, 11月 15, 2025にアクセス、 [https://stackoverflow.com/questions/62767668/how-can-i-tell-if-the-window-handle-is-main-window-handle](https://stackoverflow.com/questions/62767668/how-can-i-tell-if-the-window-handle-is-main-window-handle)  
31. ShowWindow in windows\_win::sys \- Rust \- Docs.rs, 11月 15, 2025にアクセス、 [https://docs.rs/windows-win/latest/windows\_win/sys/fn.ShowWindow.html](https://docs.rs/windows-win/latest/windows_win/sys/fn.ShowWindow.html)  
32. Show/Hide icon of my application in the Window taskbar at run time \- NI Community, 11月 15, 2025にアクセス、 [https://forums.ni.com/t5/LabVIEW/Show-Hide-icon-of-my-application-in-the-Window-taskbar-at-run/td-p/3593854](https://forums.ni.com/t5/LabVIEW/Show-Hide-icon-of-my-application-in-the-Window-taskbar-at-run/td-p/3593854)  
33. Hide/Show the application icon of the Windows taskbar (LabVIEW \- WINAPI), 11月 15, 2025にアクセス、 [https://stackoverflow.com/questions/42554842/hide-show-the-application-icon-of-the-windows-taskbar-labview-winapi](https://stackoverflow.com/questions/42554842/hide-show-the-application-icon-of-the-windows-taskbar-labview-winapi)  
34. Hide the custom applicaion window but it should show the application icon on the taskbar(not as a tray icon) \- Stack Overflow, 11月 15, 2025にアクセス、 [https://stackoverflow.com/questions/42802403/hide-the-custom-applicaion-window-but-it-should-show-the-application-icon-on-the](https://stackoverflow.com/questions/42802403/hide-the-custom-applicaion-window-but-it-should-show-the-application-icon-on-the)  
35. c\# \- Hide window created by process \- Stack Overflow, 11月 15, 2025にアクセス、 [https://stackoverflow.com/questions/29161326/hide-window-created-by-process](https://stackoverflow.com/questions/29161326/hide-window-created-by-process)  
36. command line \- Windows \- Run process on background after closing cmd \- Super User, 11月 15, 2025にアクセス、 [https://superuser.com/questions/1069972/windows-run-process-on-background-after-closing-cmd](https://superuser.com/questions/1069972/windows-run-process-on-background-after-closing-cmd)  
37. CWnd::ShowWindow Can't always Hide your Window\! \- Tech Leaves, 11月 15, 2025にアクセス、 [https://techleaves.wordpress.com/2011/07/18/cwndshowwindow-cant-always-hide-your-window/](https://techleaves.wordpress.com/2011/07/18/cwndshowwindow-cant-always-hide-your-window/)  
38. Win32: Hide to system tray – Part 1 \- Lotushints, 11月 15, 2025にアクセス、 [https://www.lotushints.com/2013/03/win32-hide-to-system-tray-part-1/](https://www.lotushints.com/2013/03/win32-hide-to-system-tray-part-1/)  
39. std::thread::sleep \- Rust \- MIT, 11月 15, 2025にアクセス、 [https://web.mit.edu/rust-lang\_v1.25/arch/amd64\_ubuntu1404/share/doc/rust/html/std/thread/fn.sleep.html](https://web.mit.edu/rust-lang_v1.25/arch/amd64_ubuntu1404/share/doc/rust/html/std/thread/fn.sleep.html)  
40. sleep in std::thread \- Rust, 11月 15, 2025にアクセス、 [https://doc.rust-lang.org/std/thread/fn.sleep.html](https://doc.rust-lang.org/std/thread/fn.sleep.html)  
41. rust \- How can I put the current thread to sleep? \- Stack Overflow, 11月 15, 2025にアクセス、 [https://stackoverflow.com/questions/28952938/how-can-i-put-the-current-thread-to-sleep](https://stackoverflow.com/questions/28952938/how-can-i-put-the-current-thread-to-sleep)  
42. Location of the Startup folder in Windows 10/8 \- Edulab, 11月 15, 2025にアクセス、 [https://edulab.unitn.it/tecnici/location-of-the-startup-folder-in-windows-10-8/](https://edulab.unitn.it/tecnici/location-of-the-startup-folder-in-windows-10-8/)  
43. Configure Startup Applications in Windows \- Microsoft Support, 11月 15, 2025にアクセス、 [https://support.microsoft.com/en-us/windows/configure-startup-applications-in-windows-115a420a-0bff-4a6f-90e0-1934c844e473](https://support.microsoft.com/en-us/windows/configure-startup-applications-in-windows-115a420a-0bff-4a6f-90e0-1934c844e473)  
44. Finding the startup folder on Windows operating systems \- Telos Alliance, 11月 15, 2025にアクセス、 [https://docs.telosalliance.com/docs/finding-the-startup-folder-on-windows-operating-systems](https://docs.telosalliance.com/docs/finding-the-startup-folder-on-windows-operating-systems)  
45. Adding Programs and Apps to the Startup Folder in Windows 10 | Dell US, 11月 15, 2025にアクセス、 [https://www.dell.com/support/kbdoc/en-us/000124550/how-to-add-app-to-startup-in-windows-10](https://www.dell.com/support/kbdoc/en-us/000124550/how-to-add-app-to-startup-in-windows-10)  
46. How to have a program startup when Windows loads? : r/Windows11 \- Reddit, 11月 15, 2025にアクセス、 [https://www.reddit.com/r/Windows11/comments/1lk54ex/how\_to\_have\_a\_program\_startup\_when\_windows\_loads/](https://www.reddit.com/r/Windows11/comments/1lk54ex/how_to_have_a_program_startup_when_windows_loads/)  
47. Boot or Logon Autostart Execution: Registry Run Keys / Startup Folder \- MITRE ATT\&CK®, 11月 15, 2025にアクセス、 [https://attack.mitre.org/techniques/T1547/001/](https://attack.mitre.org/techniques/T1547/001/)  
48. T1547.001 \- Boot or Logon Autostart Execution: Registry Run Keys / Startup Folder \- Atomic Red Team, 11月 15, 2025にアクセス、 [https://www.atomicredteam.io/atomic-red-team/atomics/T1547.001](https://www.atomicredteam.io/atomic-red-team/atomics/T1547.001)  
49. Run and RunOnce Registry Keys \- Win32 apps \- Microsoft Learn, 11月 15, 2025にアクセス、 [https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys](https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys)  
50. How do I auto-start a program for a specific user using the CurrentVersion\\Run key in the registry? \- Stack Overflow, 11月 15, 2025にアクセス、 [https://stackoverflow.com/questions/16185165/how-do-i-auto-start-a-program-for-a-specific-user-using-the-currentversion-run-k](https://stackoverflow.com/questions/16185165/how-do-i-auto-start-a-program-for-a-specific-user-using-the-currentversion-run-k)  
51. windows\_registry \- Rust \- Docs.rs, 11月 15, 2025にアクセス、 [https://docs.rs/windows-registry](https://docs.rs/windows-registry)  
52. How to make my .exe autorun in Windows? \- help \- Rust Users Forum, 11月 15, 2025にアクセス、 [https://users.rust-lang.org/t/how-to-make-my-exe-autorun-in-windows/49045](https://users.rust-lang.org/t/how-to-make-my-exe-autorun-in-windows/49045)  
53. Shell Links \- Win32 apps \- Microsoft Learn, 11月 15, 2025にアクセス、 [https://learn.microsoft.com/en-us/windows/win32/shell/links](https://learn.microsoft.com/en-us/windows/win32/shell/links)  
54. Windows shortcuts (lnk) help \- Rust Users Forum, 11月 15, 2025にアクセス、 [https://users.rust-lang.org/t/windows-shortcuts-lnk-help/28982](https://users.rust-lang.org/t/windows-shortcuts-lnk-help/28982)  
55. Using Windows COM Automation with Rust \- Stack Overflow, 11月 15, 2025にアクセス、 [https://stackoverflow.com/questions/72606367/using-windows-com-automation-with-rust](https://stackoverflow.com/questions/72606367/using-windows-com-automation-with-rust)  
56. How to use a windows COM object using Rust? \- Reddit, 11月 15, 2025にアクセス、 [https://www.reddit.com/r/rust/comments/wl0z41/how\_to\_use\_a\_windows\_com\_object\_using\_rust/](https://www.reddit.com/r/rust/comments/wl0z41/how_to_use_a_windows_com_object_using_rust/)  
57. lnk \- Rust \- Docs.rs, 11月 15, 2025にアクセス、 [https://docs.rs/lnk](https://docs.rs/lnk)  
58. mslnk \- crates.io: Rust Package Registry, 11月 15, 2025にアクセス、 [https://crates.io/crates/mslnk](https://crates.io/crates/mslnk)  
59. Is there a good way to generate shortcuts in pure Rust \- help, 11月 15, 2025にアクセス、 [https://users.rust-lang.org/t/is-there-a-good-way-to-generate-shortcuts-in-pure-rust/55896](https://users.rust-lang.org/t/is-there-a-good-way-to-generate-shortcuts-in-pure-rust/55896)