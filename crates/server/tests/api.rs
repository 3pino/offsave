//! REST と、デバイスの WebSocket の結合テスト（開発モードのサーバーを in-process で起動する）。

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use offsave_core::protocol::{
    DeviceFlags, DeviceMsg, Event, EventKind, Friend, FriendState, Policy, ServerMsg, UserStatus,
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
    }
}

fn game_rule(seconds: u32) -> Value {
    json!({
        "name": "ゲーム",
        "targets": [{ "label": "ゲーム", "keys": ["android:com.example.game"] }],
        "limit": { "type": "daily_total", "seconds": seconds },
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

    // 本人は変更・削除できない（緩める操作は友達だけ）
    assert_eq!(alice.put(&format!("/rules/{rule_id}"), game_rule(99999)).await.0, 403);
    assert_eq!(alice.delete(&format!("/rules/{rule_id}")).await.0, 403);
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
            json!({ "name": "x", "targets": [{"label":"a","keys":["bad key"]}], "limit": {"type":"daily_total","seconds":1} }),
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
    phone.send(&DeviceMsg::Usage { segments: vec![seg(now - 90_000, now - 30_000)] }).await;
    tablet.send(&DeviceMsg::Usage { segments: vec![seg(now - 60_000, now)] }).await;
    // 不正な区間（未来・長すぎ）は捨てられる
    tablet
        .send(&DeviceMsg::Usage { segments: vec![seg(now + 3_600_000, now + 3_700_000)] })
        .await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let status = bob.status(&a_id).await;
    assert_eq!(status.rules[0].used_seconds, 90);
    assert_eq!(status.usage_today[0].seconds, 90);
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
