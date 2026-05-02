use crate::{game::player::PlayerId, store::leaderboard::Score};

pub(crate) fn leaderboard_score(player: &crate::game::player::Player) -> Score {
    Score {
        xp: player.xp,
        coins: player.coins,
    }
}

pub(crate) fn is_bootstrap_admin(player_id: PlayerId, bootstrap_admin_ids: &[PlayerId]) -> bool {
    bootstrap_admin_ids.contains(&player_id)
}
