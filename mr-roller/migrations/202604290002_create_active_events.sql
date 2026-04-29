CREATE TABLE active_events (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    event_json TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE INDEX idx_active_events_status_expires_at
ON active_events(status, expires_at);
