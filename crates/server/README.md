# offsave-server

offsave のサーバー。方針（ルール）の配布、利用の集計、友達への通知を行う。制限の強制は、デバイスの中で行う。動かし方の概要は、リポジトリ直下の [README.md](../../README.md) にある。

## 環境変数

| 環境変数 | 既定値 | 内容 |
|---|---|---|
| `OFFSAVE_BIND` | `127.0.0.1:8090` | 待ち受けるアドレス。本番では Tailscale の IP（`tailscale ip -4`） |
| `OFFSAVE_DATA_DIR` | `./data` | DB（`offsave.db`）の置き場所 |
| `OFFSAVE_DEV` | なし | `1` で開発モード（認証をしない。`X-Dev-User` ヘッダーで誰にでもなれる）。**本番では使わない** |
| `OFFSAVE_DEFAULT_TZ` | `Asia/Tokyo` | 新しいユーザーのタイムゾーン |
| `OFFSAVE_SUSPECT_AFTER_SECS` | `600` | ハートビートがこの秒数途絶えたら、友達に「連絡が途絶えた」と通知する |
| `OFFSAVE_PROTECTION_GRACE_SECS` | `15` | 監視（Accessibility・デバイス管理者）が外れてから、この秒数のあいだに戻れば、通知しない（アプリの更新で一瞬外れるため） |
| `OFFSAVE_TAILSCALE_SOCKET` | 自動 | tailscaled の LocalAPI のソケット |
| `OFFSAVE_AUTO_UPDATE` | なし | `1` で自動更新。最新の正式版を 10 分おきに見て、sha256 と `--version` を確かめ、DB をバックアップしてから入れ替えて再起動する。systemd の `Restart=on-failure` が、入れ替えたあとの再起動をする |

## 注意

- 認証は Tailscale の whois（接続元の IP からユーザーを見分ける）。`OFFSAVE_DEV` は本番で使わない。
- 管理者は、全員のデータを見られ、DB も書き換えられる。相互監視の前提で、管理者を信頼する運用。
