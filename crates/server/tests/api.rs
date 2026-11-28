//! REST と、デバイスの WebSocket の結合テスト（開発モードのサーバーを in-process で起動する）。

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use offsave_core::protocol::{
    DeviceFlags, DeviceMsg, Event, EventKind, Friend, FriendState, KeyLabel, Policy, ServerMsg,
    UserStatus,
};
use offsave_server::db::now_ms;
use offsave_server::testing::DevServer;
use offsave_server::watchdog;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

struct Client<'a> {
    server: &'a DevServer,
    http: reqwest::Client,
    user: &'static str,
}

impl<'a> Client<'a> {
    fn new(server: &'a DevServer, user: &'static str) -> Self {
        Self {
            server,
            http: reqwest::Client::new(),
            user,
        }
    }

    async fn send(&self, method: reqwest::Method, path: &str, body: Option<Value>) -> (u16, Value) {
        let mut req = self
            .http
            .request(method, self.server.url(&format!("/api{path}")))
            .header("x-dev-user", self.user);
        if let Some(b) = body {
            req = req.json(&b);
        }
        let res = req.send().await.unwrap();
        let status = res.status().as_u16();
        (status, res.json().await.unwrap_or(Value::Null))
    }

    async fn get(&self, path: &str) -> (u16, Value) {
        self.send(reqwest::Method::GET, path, None).await
    }
    /// JSON でない応答（画像）: (状態, Content-Type, 本文)
    async fn get_raw(&self, path: &str) -> (u16, String, Vec<u8>) {
        let res = self
            .http
            .get(self.server.url(&format!("/api{path}")))
            .header("x-dev-user", self.user)
            .send()
            .await
            .unwrap();
        let code = res.status().as_u16();
        let ct = res.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_owned();
        (code, ct, res.bytes().await.unwrap().to_vec())
    }
    async fn post(&self, path: &str, body: Value) -> (u16, Value) {
        self.send(reqwest::Method::POST, path, Some(body)).await
    }
    async fn put(&self, path: &str, body: Value) -> (u16, Value) {
        self.send(reqwest::Method::PUT, path, Some(body)).await
    }
    async fn delete(&self, path: &str) -> (u16, Value) {
        self.send(reqwest::Method::DELETE, path, None).await
    }

    async fn id(&self) -> String {
        self.get("/me").await.1["user"]["id"].as_str().unwrap().to_owned()
    }

    async fn status(&self, user_id: &str) -> UserStatus {
        let (code, v) = self.get(&format!("/users/{user_id}/status")).await;
        assert_eq!(code, 200, "{v}");
        serde_json::from_value(v).unwrap()
    }

    /// デバイスを登録して WebSocket につなぐ。
    async fn device(&self, name: &str) -> Device {
        let (code, v) = self
            .post("/devices", json!({ "name": name, "platform": "android" }))
            .await;
        assert_eq!(code, 200, "{v}");
        let id = v["device"]["id"].as_str().unwrap().to_owned();
        let token = v["token"].as_str().unwrap().to_owned();
        let mut req = self.server.ws_url("/api/ws/device", self.user).into_client_request().unwrap();
        req.headers_mut().insert("x-device-token", token.parse().unwrap());
        let (ws, _) = tokio_tungstenite::connect_async(req).await.unwrap();
        Device { id, token, ws }
    }
}

struct Device {
    id: String,
    #[allow(dead_code)]
    token: String,
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl Device {
    async fn send(&mut self, msg: &DeviceMsg) {
        self.ws
            .send(Message::Text(serde_json::to_string(msg).unwrap().into()))
            .await
            .unwrap();
    }

    async fn recv(&mut self) -> ServerMsg {
        loop {
            let m = tokio::time::timeout(Duration::from_secs(5), self.ws.next())
                .await
                .expect("サーバーからの応答がありません")
                .unwrap()
                .unwrap();
            if let Message::Text(t) = m {
                return serde_json::from_str(&t).unwrap();
            }
        }
    }

    async fn heartbeat(&mut self, flags: DeviceFlags) -> Policy {
        self.send(&DeviceMsg::Heartbeat {
            flags,
            foreground: Some("android:com.example.game".into()),
            app_version: "0.1.0".into(),
            last_event_id: None,
        })
        .await;
        loop {
            if let ServerMsg::Policy(p) = self.recv().await {
                return p;
            }
        }
    }

    /// 方針の更新は非同期にも届くので、条件を満たす方針が来るまでハートビートを送る。
    async fn heartbeat_until(&mut self, flags: DeviceFlags, ok: impl Fn(&Policy) -> bool) -> Policy {
        for _ in 0..20 {
            let p = self.heartbeat(flags.clone()).await;
            if ok(&p) {
                return p;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("条件を満たす方針が来ません");
    }

    async fn next_event(&mut self) -> Event {
        loop {
            if let ServerMsg::Event { event } = self.recv().await {
                return event;
            }
        }
    }
}

fn protected() -> DeviceFlags {
    DeviceFlags {
        accessibility: true,
        device_admin: true,
        battery_unrestricted: true,
        paused: false,
    }
}

fn game_rule(seconds: u32) -> Value {
    json!({
        "name": "ゲーム",
        "targets": [{ "label": "ゲーム", "keys": ["android:com.example.game"] }],
        "limits": [{ "type": "daily_total", "seconds": seconds }],
    })
}

/// alice と bob を友達にする。
async fn make_friends(a: &Client<'_>, b: &Client<'_>) -> (String, String) {
    let (a_id, b_id) = (a.id().await, b.id().await);
    let (code, v) = a.post("/friends", json!({ "login_name": b.user })).await;
    assert_eq!(code, 200, "{v}");
    let (code, _) = b.post(&format!("/friends/{a_id}/accept"), json!({})).await;
    assert_eq!(code, 200);
    (a_id, b_id)
}

#[tokio::test]
async fn friend_request_and_accept() {
    let s = DevServer::start().await;
    let (alice, bob) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"));
    let (a_id, b_id) = (alice.id().await, bob.id().await);

    // 相手が先にアプリを開いていないと申請できない
    let (code, _) = alice.post("/friends", json!({ "login_name": "nobody@t" })).await;
    assert_eq!(code, 404);

    alice.post("/friends", json!({ "login_name": "bob@t" })).await;
    let friends: Vec<Friend> = serde_json::from_value(alice.get("/friends").await.1).unwrap();
    assert_eq!(friends[0].state, FriendState::PendingOut);
    let friends: Vec<Friend> = serde_json::from_value(bob.get("/friends").await.1).unwrap();
    assert_eq!(friends[0].state, FriendState::PendingIn);

    // 申請した本人は承認できない
    let (code, _) = alice.post(&format!("/friends/{b_id}/accept"), json!({})).await;
    assert_eq!(code, 404);
    let (code, _) = bob.post(&format!("/friends/{a_id}/accept"), json!({})).await;
    assert_eq!(code, 200);
    let friends: Vec<Friend> = serde_json::from_value(alice.get("/friends").await.1).unwrap();
    assert_eq!(friends[0].state, FriendState::Accepted);
}

#[tokio::test]
async fn permissions_on_rules_and_status() {
    let s = DevServer::start().await;
    let (alice, bob, carol) = (
        Client::new(&s, "alice@t"),
        Client::new(&s, "bob@t"),
        Client::new(&s, "carol@t"),
    );
    let (a_id, _) = make_friends(&alice, &bob).await;
    carol.id().await;

    // 友達でなければ見られない・足せない
    assert_eq!(carol.get(&format!("/users/{a_id}/status")).await.0, 403);
    assert_eq!(carol.post(&format!("/users/{a_id}/rules"), game_rule(60)).await.0, 403);

    // 友達（bob）が alice のルールを決める
    let (code, rule) = bob.post(&format!("/users/{a_id}/rules"), game_rule(3600)).await;
    assert_eq!(code, 200, "{rule}");
    let rule_id = rule["id"].as_str().unwrap().to_owned();

    // 本人は、緩める変更・削除はできない（友達だけ）。厳しくする変更はできる
    assert_eq!(alice.put(&format!("/rules/{rule_id}"), game_rule(7200)).await.0, 403);
    assert_eq!(alice.delete(&format!("/rules/{rule_id}")).await.0, 403);
    let stricter = alice.put(&format!("/rules/{rule_id}"), game_rule(1800)).await;
    assert_eq!(stricter.0, 200, "{}", stricter.1);
    let mut renamed = game_rule(1800);
    renamed["name"] = json!("名前だけ変えた");
    renamed["targets"] = json!([{ "label": "ゲーム", "keys": ["android:com.example.game"] }, { "label": "もう一つ", "keys": ["android:com.other"] }]);
    assert_eq!(alice.put(&format!("/rules/{rule_id}"), renamed).await.0, 200);
    // 変更は友達に通知される
    let events: Vec<Event> = serde_json::from_value(bob.get("/events").await.1).unwrap();
    assert!(events.iter().any(|e| e.kind == EventKind::RuleChanged && e.message.contains("変更")));
    // 友達が元の強さに戻す
    assert_eq!(bob.put(&format!("/rules/{rule_id}"), game_rule(3600)).await.0, 200);
    assert_eq!(
        alice
            .post(&format!("/users/{a_id}/overrides"), json!({ "date": "2099-01-01" }))
            .await
            .0,
        403
    );

    // 本人でも、自分に新しいルールを足すのはよい（厳しくする方向）
    assert_eq!(alice.post(&format!("/users/{a_id}/rules"), game_rule(600)).await.0, 200);

    // 不正なルールは弾く
    let (code, _) = bob
        .post(
            &format!("/users/{a_id}/rules"),
            json!({ "name": "x", "targets": [{"label":"a","keys":["bad key"]}], "limits": [{"type":"daily_total","seconds":1}] }),
        )
        .await;
    assert_eq!(code, 400);
    // 制限が 1 つもないルールも弾く
    let (code, _) = bob
        .post(
            &format!("/users/{a_id}/rules"),
            json!({ "name": "x", "targets": [{"label":"a","keys":["android:a"]}], "limits": [] }),
        )
        .await;
    assert_eq!(code, 400);

    // 友達は変更・削除でき、オーバーライドも作れる
    assert_eq!(bob.put(&format!("/rules/{rule_id}"), game_rule(7200)).await.0, 200);
    let (code, o) = bob
        .post(&format!("/users/{a_id}/overrides"), json!({ "date": "2099-01-01", "rule_id": rule_id }))
        .await;
    assert_eq!(code, 200, "{o}");
    assert_eq!(bob.post(&format!("/users/{a_id}/overrides"), json!({ "date": "2000-01-01" })).await.0, 400);

    let status = bob.status(&a_id).await;
    assert_eq!(status.rules.len(), 2);
    assert_eq!(status.overrides.len(), 1);
    assert_eq!(status.rules[0].author.login_name, "bob@t");

    assert_eq!(bob.delete(&format!("/rules/{rule_id}")).await.0, 200);
}

#[tokio::test]
async fn dissolving_friendship_removes_that_friends_rules() {
    let s = DevServer::start().await;
    let (alice, bob) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"));
    let (a_id, b_id) = make_friends(&alice, &bob).await;
    bob.post(&format!("/users/{a_id}/rules"), game_rule(60)).await;
    assert_eq!(alice.status(&a_id).await.rules.len(), 1);

    // 本人から一方的に解消できる
    assert_eq!(alice.delete(&format!("/friends/{b_id}")).await.0, 200);
    assert_eq!(alice.status(&a_id).await.rules.len(), 0);
    assert_eq!(bob.get(&format!("/users/{a_id}/status")).await.0, 403);
    // 解消のイベントは両方に残る
    let events: Vec<Event> = serde_json::from_value(bob.get("/events").await.1).unwrap();
    assert!(events.iter().any(|e| e.kind == EventKind::FriendRemoved));
}

#[tokio::test]
async fn heartbeat_returns_policy_and_usage_is_aggregated_across_devices() {
    let s = DevServer::start().await;
    let (alice, bob) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"));
    let (a_id, _) = make_friends(&alice, &bob).await;
    bob.post(&format!("/users/{a_id}/rules"), game_rule(120)).await;

    let mut phone = alice.device("phone").await;
    let mut tablet = alice.device("tablet").await;
    let policy = phone.heartbeat(protected()).await;
    assert_eq!(policy.rules.len(), 1);
    assert_eq!(policy.clock.utc_offset_min, 540);

    // 電話 60 秒、タブレット 60 秒（30 秒重なる）→ 合計 90 秒
    let now = now_ms();
    let seg = |start: i64, end: i64| offsave_core::intervals::Segment {
        key: "android:com.example.game".into(),
        start,
        end,
    };
    phone
        .send(&DeviceMsg::Usage {
            segments: vec![seg(now - 90_000, now - 30_000)],
            labels: vec![
                KeyLabel { key: "android:com.example.game".into(), label: "ゲーム".into() },
                // 不正な識別子の名前は捨てられる
                KeyLabel { key: "bad key".into(), label: "x".into() },
            ],
        })
        .await;
    tablet.send(&DeviceMsg::Usage { segments: vec![seg(now - 60_000, now)], labels: vec![] }).await;
    // 不正な区間（未来・長すぎ）は捨てられる
    tablet
        .send(&DeviceMsg::Usage { segments: vec![seg(now + 3_600_000, now + 3_700_000)], labels: vec![] })
        .await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let status = bob.status(&a_id).await;
    assert_eq!(status.rules[0].used_seconds, 90);
    assert_eq!(status.usage_today[0].seconds, 90);
    assert_eq!(
        status.labels,
        vec![KeyLabel { key: "android:com.example.game".into(), label: "ゲーム".into() }]
    );
    assert_eq!(status.devices.len(), 2);
    assert_eq!(status.devices[0].state, "online");
    assert!(status.devices[0].flags.protected());

    // タブレットには、電話の分が「ほかのデバイス」として渡る
    let policy = tablet.heartbeat(protected()).await;
    let others: i64 = policy.others.iter().map(|s| s.end - s.start).sum();
    assert_eq!(others, 60_000);
}

#[tokio::test]
async fn unlock_notifies_friend_with_reason_and_updates_policy() {
    let s = DevServer::start().await;
    let (alice, bob) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"));
    let (a_id, _) = make_friends(&alice, &bob).await;
    let mut alice_phone = alice.device("phone").await;
    let mut bob_phone = bob.device("phone").await;
    alice_phone.heartbeat(protected()).await;
    bob_phone.heartbeat(protected()).await;

    let now = now_ms();
    alice_phone
        .send(&DeviceMsg::Unlock {
            reason: "  明日の授業の資料を調べるため  ".into(),
            started_at: now,
            until: now + 15 * 60_000,
        })
        .await;
    // 友達の端末に、理由が本文のイベントが届く
    let event = loop {
        let e = bob_phone.next_event().await;
        if e.kind != EventKind::DeviceAdded {
            break e;
        }
    };
    assert_eq!(event.kind, EventKind::TempUnlock);
    assert_eq!(event.message, "明日の授業の資料を調べるため");
    assert_eq!(event.user.login_name, "alice@t");
    // 本人のデバイスにも新しい方針（解除の終わり）が届く
    loop {
        if let ServerMsg::Policy(p) = alice_phone.recv().await {
            assert_eq!(p.unlock_until, now + 15 * 60_000);
            break;
        }
    }
    assert_eq!(bob.status(&a_id).await.unlock_until, now + 15 * 60_000);

    // 長すぎる解除・理由なしは受け付けない
    alice_phone
        .send(&DeviceMsg::Unlock { reason: "x".into(), started_at: now, until: now + 3_600_000 })
        .await;
    assert!(matches!(alice_phone.recv().await, ServerMsg::Error { .. }));
    alice_phone
        .send(&DeviceMsg::Unlock { reason: "  ".into(), started_at: now, until: now + 60_000 })
        .await;
    assert!(matches!(alice_phone.recv().await, ServerMsg::Error { .. }));
}

#[tokio::test]
async fn rule_keeps_several_limits_with_weekdays() {
    let s = DevServer::start().await;
    let (alice, bob) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"));
    let (a_id, _) = make_friends(&alice, &bob).await;

    // 合計 1 時間、6:00–7:00 は禁止、平日は 15:00–23:00 も禁止
    let limits = json!([
        { "type": "daily_total", "seconds": 3600 },
        { "type": "window", "days": 127, "start_min": 360, "end_min": 420 },
        { "type": "window", "days": 31, "start_min": 900, "end_min": 1380 },
    ]);
    let (code, body) = bob
        .post(
            &format!("/users/{a_id}/rules"),
            json!({ "name": "ゲーム", "targets": [{"label":"a","keys":["android:a"]}], "limits": limits }),
        )
        .await;
    assert_eq!(code, 200);
    // days を省略した合計は「毎日」になる
    assert_eq!(body["limits"][0], json!({ "type": "daily_total", "days": 127, "seconds": 3600 }));

    let status: UserStatus = serde_json::from_value(alice.get(&format!("/users/{a_id}/status")).await.1).unwrap();
    assert_eq!(status.rules[0].rule.limits.len(), 3);

    // 曜日が空の制限は弾く
    let (code, _) = bob
        .post(
            &format!("/users/{a_id}/rules"),
            json!({ "name": "x", "targets": [{"label":"a","keys":["android:a"]}], "limits": [{"type":"window","days":0,"start_min":0,"end_min":60}] }),
        )
        .await;
    assert_eq!(code, 400);
}

#[tokio::test]
async fn resent_unlock_is_notified_only_once() {
    let s = DevServer::start().await;
    let (alice, bob) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"));
    make_friends(&alice, &bob).await;
    let mut phone = alice.device("phone").await;
    phone.heartbeat(protected()).await;

    // 届いたか分からないため、デバイスは同じ解除を再送する
    let now = now_ms();
    let unlock = DeviceMsg::Unlock {
        reason: "調べ物".into(),
        started_at: now,
        until: now + 15 * 60_000,
    };
    phone.send(&unlock).await;
    phone.send(&unlock).await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    let events: Vec<Event> = serde_json::from_value(bob.get("/events").await.1).unwrap();
    assert_eq!(events.iter().filter(|e| e.kind == EventKind::TempUnlock).count(), 1);
}

#[tokio::test]
async fn client_reports_unsupported_browser_and_clock_skew() {
    let s = DevServer::start().await;
    let (alice, bob) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"));
    make_friends(&alice, &bob).await;
    let mut phone = alice.device("phone").await;
    phone.heartbeat(protected()).await;
    phone
        .send(&DeviceMsg::Report {
            kind: EventKind::UnsupportedBrowser,
            message: "対応していないブラウザ「X」".into(),
        })
        .await;
    phone
        .send(&DeviceMsg::Report { kind: EventKind::ClockSkew, message: "時計が 10 分ずれています".into() })
        .await;
    // デバイスから送れない種類（友達になりすました通知）は拒否する
    phone
        .send(&DeviceMsg::Report { kind: EventKind::DeviceOnline, message: "x".into() })
        .await;
    assert!(matches!(phone.recv().await, ServerMsg::Event { .. } | ServerMsg::Error { .. }));
    tokio::time::sleep(Duration::from_millis(300)).await;

    let events: Vec<Event> = serde_json::from_value(bob.get("/events").await.1).unwrap();
    assert!(events.iter().any(|e| e.kind == EventKind::UnsupportedBrowser));
    assert!(events.iter().any(|e| e.kind == EventKind::ClockSkew));
    assert!(!events.iter().any(|e| e.kind == EventKind::DeviceOnline && e.message == "x"));
}

#[tokio::test]
async fn silent_device_is_reported_then_recovers() {
    let s = DevServer::start().await;
    let (alice, bob) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"));
    make_friends(&alice, &bob).await;
    let mut phone = alice.device("phone").await;
    phone.heartbeat(protected()).await;

    // 3 分以内なら何も起きない
    watchdog::tick(&s.state, now_ms() + 100_000).await.unwrap();
    // 4 分途絶えたら「疑い」になり、友達にイベントが行く
    watchdog::tick(&s.state, now_ms() + 240_000).await.unwrap();
    // 二重には通知しない
    watchdog::tick(&s.state, now_ms() + 300_000).await.unwrap();
    let events: Vec<Event> = serde_json::from_value(bob.get("/events").await.1).unwrap();
    assert_eq!(events.iter().filter(|e| e.kind == EventKind::DeviceOffline).count(), 1);

    // 戻ったら「再接続」が通知される
    phone.heartbeat(protected()).await;
    let events: Vec<Event> = serde_json::from_value(bob.get("/events").await.1).unwrap();
    assert!(events.iter().any(|e| e.kind == EventKind::DeviceOnline));
}

#[tokio::test]
async fn protection_loss_and_clean_shutdown() {
    let s = DevServer::start().await;
    let (alice, bob) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"));
    make_friends(&alice, &bob).await;
    let mut bob_phone = bob.device("phone").await;
    bob_phone.heartbeat(protected()).await;
    let mut phone = alice.device("phone").await;
    phone.heartbeat(protected()).await;

    // Accessibility が外れたら、友達の端末にイベントが届く
    phone
        .heartbeat(DeviceFlags { accessibility: false, ..protected() })
        .await;
    let e = loop {
        let e = bob_phone.next_event().await;
        if e.kind != EventKind::DeviceAdded {
            break e;
        }
    };
    assert_eq!(e.kind, EventKind::ProtectionDisabled);
    assert!(e.message.contains("Accessibility"));

    // 正常終了ならウォッチドッグは通知しない
    phone.send(&DeviceMsg::Bye).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    watchdog::tick(&s.state, now_ms() + 1_000_000).await.unwrap();
    let events: Vec<Event> = serde_json::from_value(bob.get("/events").await.1).unwrap();
    // bob 自身の端末は途絶えた扱いになるので、alice についてのものだけを見る
    assert!(!events.iter().any(|e| e.kind == EventKind::DeviceOffline && e.user.login_name == "alice@t"));
}

#[tokio::test]
async fn only_a_friend_sets_the_apps_that_pause_protection_and_pausing_is_not_a_loss() {
    let s = DevServer::start().await;
    let (alice, bob, carol) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"), Client::new(&s, "carol@t"));
    let (a_id, _) = make_friends(&alice, &bob).await;
    let mut bob_phone = bob.device("phone").await;
    bob_phone.heartbeat(protected()).await;
    let mut phone = alice.device("phone").await;
    assert!(phone.heartbeat(protected()).await.pause_apps.is_empty());

    let path = format!("/users/{a_id}/pause-apps");
    let pay = json!({ "keys": ["android:jp.ne.paypay.android.app"] });
    // 本人・友達でない人は決められない。サイトや不正な識別子も不可
    assert_eq!(alice.put(&path, pay.clone()).await.0, 403);
    assert_eq!(carol.put(&path, pay.clone()).await.0, 403);
    assert_eq!(bob.put(&path, json!({ "keys": ["domain:example.com"] })).await.0, 400);
    // 友達は決められ、デバイスの方針と状態に出る
    assert_eq!(bob.put(&path, pay).await.0, 200);
    phone.heartbeat_until(protected(), |p| p.pause_apps == ["android:jp.ne.paypay.android.app"]).await;
    assert_eq!(alice.status(&a_id).await.pause_apps.len(), 1);
    // 本人にも伝わる（変更は通知される）
    let events: Vec<Event> = serde_json::from_value(alice.get("/events").await.1).unwrap();
    assert!(events.iter().any(|e| e.kind == EventKind::RuleChanged && e.message.contains("Accessibility")));

    // 一時停止中（Accessibility を自分で外した）は、監視が外れたとは見なさない
    phone.heartbeat(DeviceFlags { accessibility: false, paused: true, ..protected() }).await;
    phone.heartbeat(protected()).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let events: Vec<Event> = serde_json::from_value(bob.get("/events").await.1).unwrap();
    assert!(!events.iter().any(|e| e.kind == EventKind::ProtectionDisabled));
    // でも、一時停止のふりをしない Accessibility の欠落は通知される
    phone.heartbeat(DeviceFlags { accessibility: false, ..protected() }).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let events: Vec<Event> = serde_json::from_value(bob.get("/events").await.1).unwrap();
    assert!(events.iter().any(|e| e.kind == EventKind::ProtectionDisabled));

    // デバイスが一時停止と再開を友達に報告できる
    phone
        .send(&DeviceMsg::Report { kind: EventKind::ProtectionPaused, message: "PayPayのため、監視を一時停止しました".into() })
        .await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let events: Vec<Event> = serde_json::from_value(bob.get("/events").await.1).unwrap();
    assert!(events.iter().any(|e| e.kind == EventKind::ProtectionPaused));

    // 外すと、ポリシーからも消える
    assert_eq!(bob.put(&path, json!({ "keys": [] })).await.0, 200);
    phone.heartbeat_until(protected(), |p| p.pause_apps.is_empty()).await;
}

#[tokio::test]
async fn a_blip_in_protection_is_not_reported_but_a_lasting_loss_is() {
    let s = DevServer::start_with(|c| c.protection_grace_ms = 700).await;
    let (alice, bob) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"));
    make_friends(&alice, &bob).await;
    let mut phone = alice.device("phone").await;
    phone.heartbeat(protected()).await;

    // アプリの更新などで一瞬外れてすぐ戻る。どちらも知らせない
    phone.heartbeat(DeviceFlags { accessibility: false, ..protected() }).await;
    phone.heartbeat(protected()).await;
    tokio::time::sleep(Duration::from_millis(1_200)).await;
    let kinds = |v: Value| -> Vec<EventKind> {
        serde_json::from_value::<Vec<Event>>(v).unwrap().into_iter().map(|e| e.kind).collect()
    };
    let seen = kinds(bob.get("/events").await.1);
    assert!(!seen.contains(&EventKind::ProtectionDisabled), "{seen:?}");
    assert!(!seen.contains(&EventKind::ProtectionRestored), "{seen:?}");

    // 猶予を超えて外れたままなら、知らせる。戻ったときも知らせる
    phone.heartbeat(DeviceFlags { accessibility: false, ..protected() }).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!kinds(bob.get("/events").await.1).contains(&EventKind::ProtectionDisabled));
    tokio::time::sleep(Duration::from_millis(900)).await;
    assert!(kinds(bob.get("/events").await.1).contains(&EventKind::ProtectionDisabled));
    phone.heartbeat(protected()).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(kinds(bob.get("/events").await.1).contains(&EventKind::ProtectionRestored));
}

#[tokio::test]
async fn device_removal_is_reported_and_keeps_usage() {
    let s = DevServer::start().await;
    let (alice, bob) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"));
    let (a_id, _) = make_friends(&alice, &bob).await;
    bob.post(&format!("/users/{a_id}/rules"), game_rule(3600)).await;
    let mut phone = alice.device("phone").await;
    phone.heartbeat(protected()).await;
    let now = now_ms();
    phone
        .send(&DeviceMsg::Usage {
            segments: vec![offsave_core::intervals::Segment {
                key: "android:com.example.game".into(),
                start: now - 60_000,
                end: now,
            }],
            labels: vec![],
        })
        .await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_eq!(alice.delete(&format!("/devices/{}", phone.id)).await.0, 200);
    let status = bob.status(&a_id).await;
    assert!(status.devices.is_empty());
    // デバイスを消しても、その日の利用時間は減らない
    assert_eq!(status.rules[0].used_seconds, 60);
    let events: Vec<Event> = serde_json::from_value(bob.get("/events").await.1).unwrap();
    assert!(events.iter().any(|e| e.kind == EventKind::DeviceRemoved));

    // 削除したデバイスのトークンでは、もう接続できない
    let mut req = s.ws_url("/api/ws/device", "alice@t").into_client_request().unwrap();
    req.headers_mut().insert("x-device-token", phone.token.parse().unwrap());
    assert!(tokio_tungstenite::connect_async(req).await.is_err());
}

#[tokio::test]
async fn people_lists_users_who_are_not_yet_friends() {
    let s = DevServer::start().await;
    let (alice, bob, carol) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"), Client::new(&s, "carol@t"));
    let _ = carol.id().await;
    make_friends(&alice, &bob).await;

    let logins = |v: Value| -> Vec<String> {
        v.as_array().unwrap().iter().map(|u| u["login_name"].as_str().unwrap().to_owned()).collect()
    };
    // alice: bob はもう友達なので出ない。自分も出ない
    assert_eq!(logins(alice.get("/people").await.1), vec!["carol@t"]);
    // carol からは、alice と bob が見える
    assert_eq!(logins(carol.get("/people").await.1), vec!["alice@t", "bob@t"]);
    // 申請中の人も出ない
    assert_eq!(carol.post("/friends", json!({ "login_name": "alice@t" })).await.0, 200);
    assert_eq!(logins(carol.get("/people").await.1), vec!["bob@t"]);
}

#[tokio::test]
async fn a_friend_can_remove_a_device_but_a_stranger_cannot() {
    let s = DevServer::start().await;
    let (alice, bob, carol) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"), Client::new(&s, "carol@t"));
    let (a_id, _) = make_friends(&alice, &bob).await;
    let phone = alice.device("phone").await;

    assert_eq!(carol.delete(&format!("/devices/{}", phone.id)).await.0, 403);
    assert_eq!(bob.delete(&format!("/devices/{}", phone.id)).await.0, 200);
    assert!(bob.status(&a_id).await.devices.is_empty());
    // 本人にも、だれが消したかが届く
    let events: Vec<Event> = serde_json::from_value(alice.get("/events").await.1).unwrap();
    let removed = events.iter().find(|e| e.kind == EventKind::DeviceRemoved).unwrap();
    assert!(removed.message.contains("bob"), "{}", removed.message);
    // すでに消えたデバイスは、もう消せない
    assert_eq!(bob.delete(&format!("/devices/{}", phone.id)).await.0, 404);
}

#[tokio::test]
async fn app_icons_are_shared_and_sites_never_reach_private_hosts() {
    use base64::Engine as _;
    let s = DevServer::start().await;
    let (alice, bob) = (Client::new(&s, "alice@t"), Client::new(&s, "bob@t"));
    let mut phone = alice.device("phone").await;
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    png.extend_from_slice(&[1, 2, 3, 4]);
    let b64 = |b: &[u8]| base64::engine::general_purpose::STANDARD.encode(b);
    use offsave_core::protocol::KeyIcon;
    phone
        .send(&DeviceMsg::Icons {
            icons: vec![
                KeyIcon { key: "android:com.example.game".into(), png: b64(&png) },
                // PNG でない・サイトのキー・大きすぎるものは受け取らない
                KeyIcon { key: "android:com.example.bad".into(), png: b64(b"<svg/>") },
                KeyIcon { key: "domain:example.com".into(), png: b64(&png) },
                KeyIcon { key: "android:com.example.big".into(), png: b64(&[png.clone(), vec![0; 20_000]].concat()) },
            ],
        })
        .await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 友達でなくても、ログインしていれば取れる（アイコンは公開情報）
    let (code, ct, body) = bob.get_raw("/icons?key=android:com.example.game").await;
    assert_eq!((code, ct.as_str(), body), (200, "image/png", png));
    assert_eq!(bob.get_raw("/icons?key=android:com.example.bad").await.0, 404);
    assert_eq!(bob.get_raw("/icons?key=android:com.example.big").await.0, 404);
    // 内部向けの名前・ IP アドレスは、取りに行かずに 404
    for host in ["localhost", "127.0.0.1", "100.83.29.24", "printer.local"] {
        assert_eq!(bob.get_raw(&format!("/icons?key=domain:{host}")).await.0, 404, "{host}");
    }
    assert_eq!(bob.get_raw("/icons?key=nonsense").await.0, 400);
}

#[tokio::test]
async fn device_token_must_match_user() {
    let s = DevServer::start().await;
    let alice = Client::new(&s, "alice@t");
    let phone = alice.device("phone").await;
    let mut req = s.ws_url("/api/ws/device", "mallory@t").into_client_request().unwrap();
    req.headers_mut().insert("x-device-token", phone.token.parse().unwrap());
    assert!(tokio_tungstenite::connect_async(req).await.is_err());
    // トークンなしも拒否
    let req = s.ws_url("/api/ws/device", "alice@t").into_client_request().unwrap();
    assert!(tokio_tungstenite::connect_async(req).await.is_err());
}
