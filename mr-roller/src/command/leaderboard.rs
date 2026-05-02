use async_trait::async_trait;

use crate::{
    command::{Command, Context},
    errors::MrRollerError,
    response::Response,
};

// ── Leaderboard ────────────────────────────────────────────────────────────

pub struct LeaderboardCommand {
    pub limit: usize,
}

#[async_trait]
impl Command for LeaderboardCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        let scores = ctx.leaderboard.get_scores(self.limit).await?;

        let entries: Vec<serde_json::Value> = scores
            .iter()
            .map(|(player_id, score)| {
                serde_json::json!({
                    "player_id": player_id.0,
                    "xp": score.xp,
                    "coins": score.coins,
                })
            })
            .collect();

        if entries.is_empty() {
            Ok(Response::leaderboard(
                "No scores yet.",
                serde_json::json!([]),
            ))
        } else {
            Ok(Response::leaderboard(
                "Leaderboard:",
                serde_json::json!(entries),
            ))
        }
    }
}
