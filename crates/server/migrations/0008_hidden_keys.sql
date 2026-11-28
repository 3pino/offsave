-- 「今日の利用」で、初めは隠すアプリ・サイト。ユーザーごと（本人と友達が同じ一覧を見る）。表示だけの設定で、記録・制限には関係しない。
-- 本人も友達も変えられる（隠したものは、友達も「非表示アプリを表示」で見られる）。
CREATE TABLE hidden_keys (
    owner_id TEXT NOT NULL REFERENCES users(id),
    key TEXT NOT NULL,
    author_id TEXT NOT NULL REFERENCES users(id),
    created_at INTEGER NOT NULL,
    PRIMARY KEY (owner_id, key)
);
