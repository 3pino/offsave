# offsave

[![CI](https://github.com/3pino/offsave/actions/workflows/ci.yml/badge.svg)](https://github.com/3pino/offsave/actions/workflows/ci.yml)

友達どうしで、お互いのスクリーンタイムを見守るアプリ

- 友達があなたの制限（アプリ・サイトの使用時間、使えない時間帯）を決めます。
- 友達には、使っているアプリ・見ているサイトと、制限の解除・監視の無効化が見えます。
- 制限を破るには、理由を書いて、友達に通知が行く「一時解除」をするしかありません。
- Tailscale の中だけで動きます。インターネットには公開しません。ログインは Tailscale のアカウントで自動です（[disnans](https://github.com/3pino/disnans) と同じ仕組み）。
- 対応: Android。Windows / Linux は現在ダッシュボードのみです。

## 機能

- アプリ単位・サイト単位での制限。
- グループ機能。例: YouTube アプリと youtube.com をグループ化して制限できます。
  - 1つのアプリ・サイトが入れるグループは、1つだけです。
- 1日ごと・1時間ごとの利用時間のグラフ。友達の分も見られます。
- 1日の合計時間の制限・使えない時間帯の設定。
- 原則として、制限を緩めることは自分では不可能。友達の許可が必要です。
  - 緊急時用の制限解除機能はあります。このときは友達にも通知されます。
- 抜け道防止
  - 無効化やアンインストールの対策。アプリ情報のページは閲覧できません。
  - 10分間通信できないと友達に通知されます
- 60秒の猶予システム。この間にアプリやWebサイトを正しく終了するためです。

## インストール（参加する人）

### 1. Tailscale に参加する

1. [Tailscale](https://tailscale.com/download) をインストールし、自分のアカウントでログインする
2. サーバーの管理者から届く「ノード共有」の招待を受け入れる

### 2. アプリを入れる

[最新のリリース](https://github.com/3pino/offsave/releases/latest)から、デバイスに合ったファイルをダウンロードします。ファイルは署名してあります。

| デバイス | ファイル | 入れ方 |
|---|---|---|
| Android（arm64） | `offsave_*_android-arm64.apk` | ダウンロードした APK を開く。「この提供元のアプリを許可」の確認が出たら許可する |
| Windows | `offsave_*_x64-setup.exe` | 実行してインストールする。「Windows によって PC が保護されました」と出たら「詳細情報」→「実行」 |
| Linux（Ubuntu / Debian） | `offsave_*_amd64.deb` | `sudo apt install ./offsave_*_amd64.deb` |
| Linux（その他） | `offsave_*_amd64.AppImage` | `chmod +x` で実行権限を付けて起動する |

入れたあとの更新は、アプリの「設定」の一番下の「バージョン」からできます。新しい版があると、「設定」に点が出ます。

### 3. サーバーに接続する

アプリを起動し、管理者から教わったサーバーのアドレス（例: `http://100.x.y.z:8090`）を入力して「同意して接続する」を押します。

### 4. Android の監視を有効にする

「設定」→「このデバイスの監視」の 6 項目を、上から順に有効にします。
ただし、「設定の書き込みの権限」と「使用状況へのアクセス」は、[決済アプリを使うとき](#決済アプリを使うとき) のみ必須です。「設定の書き込みの権限」を登録するには、別途 Shizuku アプリあるいはパソコンが必要です。

そのあと、「設定」→「友達」で相手を検索して申請し、承認してもらいます。

### 決済アプリを使うとき

PayPay など一部のアプリは、 offsave が Accessibility を使っていることを検知し動作しなくなります。offsave は、そのアプリを開いている間だけ、Accessibility を無効化することができます。対象のアプリは、友達が「ルール」タブの「監視を一時停止するアプリ」で決めます。本人は追加できません。

- 「監視を一時停止するアプリ」を開いているとき、利用時間はカウントされます。
- 「監視を一時停止するアプリ」を開いているとき、制限の表示（ブロック）はできません。
- 画面オフ・30 分経過・アプリの再起動のいずれかが起こると、監視を再開します。
- 使うデバイスに、「設定の書き込みの権限」と「使用状況へのアクセス」権限が必要です。Shizuku が動いていればボタン 1 つで付けられます。PC で付ける場合:
    ```
    adb shell pm grant dev.offsave.app android.permission.WRITE_SECURE_SETTINGS
    ```

## サーバーを動かす（管理者）

必要なもの: Rust（stable）、Tailscale

```sh
cargo build --release -p offsave-server
OFFSAVE_BIND=<Tailscale の IP>:8090 OFFSAVE_DATA_DIR=/var/lib/offsave ./target/release/offsave-server
```

- Tailscale の IP は `tailscale ip -4` で確認できます。
- 環境変数（自動更新など）は [crates/server/README.md](crates/server/README.md) を参照してください。
- 友達を招待するには、Tailscale の管理画面でサーバー機を[ノード共有](https://tailscale.com/kb/1084/sharing)します（disnans と同じ）。**共有で入った人は、ACL で許可したポートにしか届きません。** disnans を共有済みでも、offsave のポートは別に足す必要があります（足さないと、友達の側は接続が止まったままになります）。

```jsonc
// Access Controls（login.tailscale.com/admin/acls）の "acls" に足す。"server" は hosts に書いたサーバー機の名前
{ "action": "accept", "src": ["autogroup:shared"], "dst": ["server:8090"] }
```

サーバー管理者は、全員のデータを見られ、DB も書き換えられます。相互監視の前提で、管理者を信頼する運用です。

## 既知の限界

検知も通知もできないもの（[設計メモ](docs/design/overview.md#7-既知の限界検知も通知もできないもの)）:

- Android: 別プロファイル（Work profile・Private Space・アプリのクローン）の中での利用内容。
- 全 OS: 未登録のデバイスでの利用。
- 決済アプリ用の一時停止（上記）の間は、制限を表示できない。ほかの方法で監視が外れたときと違い、一時停止・再開は友達に通知されない（対象のアプリは友達が決めており、デバイスの一覧に「一時停止中」と出る）。
- ブラウザは、URL バーを読める対応ブラウザだけで、サイトを数える。対応外のブラウザを使うと、友達に通知される。
- 対応ブラウザのうち、実際に確認したのは Chrome と Firefox（161）。ほかは同じ系統の ID で書いてあるが、実機での確認が要る。

## 開発

開発・テスト・リリースの手順は [CONTRIBUTING.md](CONTRIBUTING.md)、設計の経緯と決定事項は [docs/design/overview.md](docs/design/overview.md) にあります。
