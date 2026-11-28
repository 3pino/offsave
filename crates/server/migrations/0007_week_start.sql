-- 週の始まりの曜日（0 = 月 … 6 = 日）。週の合計と棒グラフの表示にだけ使う。
ALTER TABLE users ADD COLUMN week_start INTEGER NOT NULL DEFAULT 0;
