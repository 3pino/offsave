-- デバイスは、届いたか分からない一時解除を再送する。同じ解除（デバイス + 開始時刻）は 1 回だけ記録する
CREATE UNIQUE INDEX unlocks_device_started ON unlocks(device_id, started_at);
