//! 接続中のデバイスの一覧。イベントと方針の更新を、WebSocket のタスクに渡す。

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use offsave_core::protocol::{Event, ServerMsg};
use tokio::sync::mpsc;

/// WebSocket のタスクへの指示。
#[derive(Debug)]
pub enum Outgoing {
    Msg(ServerMsg),
    /// 方針を作り直して送る（友達がルールを変えたとき）。
    RefreshPolicy,
}

struct Conn {
    conn_id: u64,
    user_id: String,
    tx: mpsc::UnboundedSender<Outgoing>,
}

#[derive(Default)]
pub struct Hub {
    next: AtomicU64,
    conns: Mutex<HashMap<String, Conn>>,
}

impl Hub {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Conn>> {
        self.conns.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// デバイスの接続を登録する。同じデバイスの古い接続は置き換わる（古い側の受信が閉じて終わる）。
    pub fn register(
        &self,
        device_id: &str,
        user_id: &str,
        tx: mpsc::UnboundedSender<Outgoing>,
    ) -> u64 {
        let conn_id = self.next.fetch_add(1, Ordering::Relaxed);
        self.lock().insert(
            device_id.to_owned(),
            Conn {
                conn_id,
                user_id: user_id.to_owned(),
                tx,
            },
        );
        conn_id
    }

    pub fn unregister(&self, device_id: &str, conn_id: u64) {
        let mut conns = self.lock();
        if conns.get(device_id).is_some_and(|c| c.conn_id == conn_id) {
            conns.remove(device_id);
        }
    }

    /// `recipients` のユーザーの接続中のデバイスすべてに、イベントを送る。
    pub fn push_event(&self, event: &Event, recipients: &[String]) {
        for c in self.lock().values() {
            if recipients.contains(&c.user_id) {
                let _ = c.tx.send(Outgoing::Msg(ServerMsg::Event {
                    event: event.clone(),
                }));
            }
        }
    }

    /// そのユーザーの接続中のデバイスに、方針の再送を頼む。
    pub fn refresh_policy(&self, user_id: &str) {
        for c in self.lock().values() {
            if c.user_id == user_id {
                let _ = c.tx.send(Outgoing::RefreshPolicy);
            }
        }
    }

    pub fn is_connected(&self, device_id: &str) -> bool {
        self.lock().contains_key(device_id)
    }
}
