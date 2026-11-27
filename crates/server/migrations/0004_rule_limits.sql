-- ルールが複数の制限を持てるようにする。limit_json は、制限 1 つのオブジェクトから、制限の配列にする。
UPDATE rules SET limit_json = '[' || limit_json || ']' WHERE substr(limit_json, 1, 1) = '{';
