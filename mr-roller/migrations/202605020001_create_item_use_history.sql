CREATE TABLE IF NOT EXISTS item_use_history (
    id TEXT PRIMARY KEY NOT NULL,
    player_id INTEGER NOT NULL,
    item_id TEXT NOT NULL,
    item_name TEXT NOT NULL,
    item_kind TEXT NOT NULL,
    item_json TEXT NOT NULL,
    response_kind TEXT NOT NULL,
    roll INTEGER NULL,
    used_at TEXT NOT NULL,
    FOREIGN KEY(player_id) REFERENCES players(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_item_use_history_player_used_at
ON item_use_history(player_id, used_at DESC);

CREATE INDEX IF NOT EXISTS idx_item_use_history_item_kind
ON item_use_history(item_kind);
