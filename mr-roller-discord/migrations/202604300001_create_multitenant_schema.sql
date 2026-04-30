CREATE TABLE IF NOT EXISTS discord_games (
    game_id UUID PRIMARY KEY,
    guild_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    created_by_user_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    events_enabled BOOLEAN NOT NULL DEFAULT true,
    UNIQUE (guild_id, channel_id)
);

CREATE TABLE IF NOT EXISTS players (
    game_id UUID NOT NULL REFERENCES discord_games(game_id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    last_roll_at TIMESTAMPTZ,
    luck BIGINT NOT NULL DEFAULT 0,
    coins BIGINT NOT NULL DEFAULT 0,
    xp BIGINT NOT NULL DEFAULT 0,
    is_admin BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (game_id, id)
);

CREATE TABLE IF NOT EXISTS inventory_items (
    game_id UUID NOT NULL REFERENCES discord_games(game_id) ON DELETE CASCADE,
    item_id UUID NOT NULL,
    player_id TEXT NOT NULL,
    item_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (game_id, item_id),
    FOREIGN KEY (game_id, player_id) REFERENCES players(game_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS leaderboard_scores (
    game_id UUID NOT NULL REFERENCES discord_games(game_id) ON DELETE CASCADE,
    player_id TEXT NOT NULL,
    xp BIGINT NOT NULL DEFAULT 0,
    coins BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (game_id, player_id),
    FOREIGN KEY (game_id, player_id) REFERENCES players(game_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS active_events (
    game_id UUID NOT NULL REFERENCES discord_games(game_id) ON DELETE CASCADE,
    id UUID NOT NULL,
    kind TEXT NOT NULL,
    event_json JSONB NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (game_id, id)
);

CREATE INDEX IF NOT EXISTS idx_discord_games_events_enabled ON discord_games(events_enabled);
CREATE INDEX IF NOT EXISTS idx_inventory_items_player ON inventory_items(game_id, player_id);
CREATE INDEX IF NOT EXISTS idx_active_events_status ON active_events(game_id, status);
