CREATE TABLE IF NOT EXISTS item_use_history (
    game_id UUID NOT NULL REFERENCES discord_games(game_id) ON DELETE CASCADE,
    id UUID NOT NULL,
    player_id TEXT NOT NULL,
    item_id UUID NOT NULL,
    item_name TEXT NOT NULL,
    item_kind TEXT NOT NULL,
    item_json JSONB NOT NULL,
    response_kind TEXT NOT NULL,
    roll BIGINT,
    used_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (game_id, id),
    FOREIGN KEY (game_id, player_id) REFERENCES players(game_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_item_use_history_player_used_at
ON item_use_history(game_id, player_id, used_at DESC);

CREATE INDEX IF NOT EXISTS idx_item_use_history_kind
ON item_use_history(game_id, item_kind);
