-- Accessibility を拒むアプリ（決済アプリなど）。これが前面にある間だけ、そのユーザーのデバイスは Accessibility を自分で外す。
-- 決められるのは友達だけ（本人は決められない）。
CREATE TABLE pause_apps (
    owner_id TEXT NOT NULL REFERENCES users(id),
    key TEXT NOT NULL,
    author_id TEXT NOT NULL REFERENCES users(id),
    created_at INTEGER NOT NULL,
    PRIMARY KEY (owner_id, key)
);
