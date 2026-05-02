ALTER TABLE players ADD COLUMN has_started BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE players ADD COLUMN tutorial_completed BOOLEAN NOT NULL DEFAULT false;

-- Existing players with inventory almost certainly joined through /start before
-- this flag existed. Mark them as started so they cannot claim another starter
-- dice after the migration. Setup-created admins have no inventory and remain
-- unstarted so /start can grant their onboarding reward.
UPDATE players
SET has_started = true
WHERE EXISTS (
    SELECT 1
    FROM inventory_items
    WHERE inventory_items.game_id = players.game_id
      AND inventory_items.player_id = players.id
);
