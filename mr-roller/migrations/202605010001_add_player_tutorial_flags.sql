ALTER TABLE players ADD COLUMN has_started INTEGER NOT NULL DEFAULT 0;
ALTER TABLE players ADD COLUMN tutorial_completed INTEGER NOT NULL DEFAULT 0;

-- Existing players with inventory almost certainly joined through /start before
-- this flag existed. Mark them as started so they cannot claim another starter
-- dice after the migration. Setup-created Discord admins have no inventory and
-- remain unstarted so /start can grant their onboarding reward.
UPDATE players
SET has_started = 1
WHERE EXISTS (
    SELECT 1
    FROM inventory_items
    WHERE inventory_items.player_id = players.id
);
