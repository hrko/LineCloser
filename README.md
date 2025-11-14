# LineCloser

[![Latest Release](https://img.shields.io/github/v/release/hrko/LineCloser)](https://github.com/hrko/LineCloser/releases/latest)

Windowsの起動時に自動で表示されてしまうLINEのウィンドウを、自動的に閉じるためのユーティリティツールです。

## インストールと使い方

1.  リポジトリの[Releasesページ](https://github.com/hrko/LineCloser/releases/latest)にアクセスします。
2.  最新リリースの`Assets`セクションから`LineCloser.exe`をダウンロードします。
3.  `Windows`キー + `R`キーを押して、「ファイル名を指定して実行」ダイアログを開きます。
4.  `shell:startup`と入力し、`Enter`キーを押します。
5.  スタートアップフォルダが開きます。
6.  ダウンロードした`LineCloser.exe`を、このフォルダにドラッグ＆ドロップします。

これで、次回のWindows起動時から、`LineCloser`が自動で実行されるようになり、自動的にLINEのウィンドウが閉じられます。

## 設定

### タイムアウト時間

LINEの起動が完了するまで待機する時間を変更できます。デフォルトは **5分 (300秒)** です。PCの起動が遅い場合など、必要に応じてこの時間を調整してください。

設定を変更するには、スタートアップフォルダに作成したショートカットのプロパティを開き、「リンク先」の末尾に引数を追加します。

**例：タイムアウトを10分 (600秒) に変更する場合**

```
C:\path\to\LineCloser.exe --timeout 600
```

| フラグ      | 短縮形 | 説明                                   |
| :---------- | :----- | :------------------------------------- |
| `--timeout` | `-t`   | タイムアウト時間を秒単位で指定します。 |

## ソースからビルド

本ツールを自身でビルドする場合は、以下の手順に従ってください。

1.  [Mise](https://mise.jdx.dev/getting-started.html)をインストールします。
2.  このリポジトリをクローンします。
    ```sh
    git clone https://github.com/hrko/LineCloser.git
    cd LineCloser
    ```
3.  必要な依存関係をインストールします。
    ```sh
    mise install
    ```
4.  ビルドを実行します。
    ```sh
    mise run build
    ```
5.  実行ファイルは`target/release/LineCloser.exe`に生成されます。

## ライセンス

このプロジェクトは、[MIT License](LICENSE)の下で公開されています。
