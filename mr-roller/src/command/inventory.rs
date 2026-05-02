use async_trait::async_trait;

use crate::{
    command::{Command, Context},
    errors::MrRollerError,
    game::player::PlayerId,
    response::Response,
};

// ── Inventory ──────────────────────────────────────────────────────────────

pub struct InventoryCommand {
    pub player_id: PlayerId,
}

#[async_trait]
impl Command for InventoryCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        ctx.players.get(self.player_id).await?;

        let items = ctx.inventory.list_items(self.player_id).await?;
        let item_list: Vec<serde_json::Value> = items
            .iter()
            .map(|(id, item)| {
                serde_json::json!({
                    "id": id.to_string(),
                    "name": item.name(),
                    "description": item.description(),
                })
            })
            .collect();

        if item_list.is_empty() {
            Ok(Response::inventory(
                "Your inventory is empty.",
                serde_json::json!([]),
            ))
        } else {
            Ok(Response::inventory(
                "Your inventory:",
                serde_json::json!(item_list),
            ))
        }
    }
}
