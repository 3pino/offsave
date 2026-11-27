-- アイコン。アプリ（android:...）はデバイスが送る。サイト（domain:...）はサーバーがファビコンを取る。
-- data が NULL のものは「取れなかった」ことの記録（しばらく取り直さない）
CREATE TABLE icons (
    key TEXT PRIMARY KEY,
    mime TEXT NOT NULL,
    data BLOB,
    updated_at INTEGER NOT NULL
);
