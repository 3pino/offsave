-- デバイスが送る、アプリの表示名（友達の画面でパッケージ名の代わりに出す）
CREATE TABLE app_labels (
    user_id TEXT NOT NULL REFERENCES users(id),
    key TEXT NOT NULL,
    label TEXT NOT NULL,
    PRIMARY KEY (user_id, key)
);
