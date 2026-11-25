CREATE TABLE users (
    id TEXT PRIMARY KEY,
    login_name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    timezone TEXT NOT NULL,
    day_boundary_min INTEGER NOT NULL DEFAULT 240,
    created_at INTEGER NOT NULL
);

-- user_a < user_b の順で 1 組 1 行
CREATE TABLE friendships (
    user_a TEXT NOT NULL REFERENCES users(id),
    user_b TEXT NOT NULL REFERENCES users(id),
    requested_by TEXT NOT NULL REFERENCES users(id),
    accepted INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (user_a, user_b)
);

CREATE TABLE devices (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    platform TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    last_seen INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL DEFAULT 'new',
    flags TEXT NOT NULL DEFAULT '{}',
    foreground TEXT,
    -- 削除は印を付けるだけにする（利用の記録を消して合計を減らす抜け道をふさぐ）
    removed_at INTEGER
);
CREATE INDEX devices_user ON devices(user_id);

CREATE TABLE rules (
    id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL REFERENCES users(id),
    author_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    targets TEXT NOT NULL,
    limit_json TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX rules_owner ON rules(owner_id);

CREATE TABLE overrides (
    id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL REFERENCES users(id),
    author_id TEXT NOT NULL REFERENCES users(id),
    date TEXT NOT NULL,
    rule_id TEXT REFERENCES rules(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL
);
CREATE INDEX overrides_owner ON overrides(owner_id, date);

CREATE TABLE usage_segments (
    device_id TEXT NOT NULL REFERENCES devices(id),
    user_id TEXT NOT NULL REFERENCES users(id),
    key TEXT NOT NULL,
    start INTEGER NOT NULL,
    "end" INTEGER NOT NULL,
    PRIMARY KEY (device_id, key, start)
);
CREATE INDEX usage_user_end ON usage_segments(user_id, "end");

CREATE TABLE unlocks (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    device_id TEXT NOT NULL REFERENCES devices(id),
    reason TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    until INTEGER NOT NULL
);
CREATE INDEX unlocks_user ON unlocks(user_id, until);

CREATE TABLE events (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    device_id TEXT REFERENCES devices(id),
    kind TEXT NOT NULL,
    message TEXT NOT NULL,
    at INTEGER NOT NULL
);
CREATE INDEX events_user ON events(user_id, id);
