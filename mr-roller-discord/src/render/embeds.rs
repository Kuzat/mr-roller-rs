use mr_roller::response::{Response, ResponseKind};
use serenity::all::CreateEmbed;

pub fn response_embed(response: &Response) -> Option<CreateEmbed> {
    match response.kind {
        ResponseKind::Inventory => Some(inventory_embed(response)),
        ResponseKind::Shop => Some(shop_embed(response)),
        ResponseKind::Leaderboard => Some(leaderboard_embed(response)),
        ResponseKind::DiceRoll => Some(dice_roll_embed(response)),
        ResponseKind::Event => Some(event_embed(response)),
        ResponseKind::Success | ResponseKind::Error => None,
    }
}

pub fn inventory_embed(response: &Response) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("🎒 Inventory")
        .description(format!("{}\nUse `/use item:<item>`", response.message));

    if let Some(items) = response.data.as_ref().and_then(|data| data.as_array()) {
        for item in items.iter().take(25) {
            let name = item["name"].as_str().unwrap_or("Unknown item");
            let id = item["id"].as_str().unwrap_or("unknown");
            let description = item["description"].as_str().unwrap_or("");
            embed = embed.field(name, format!("`{}`\n{}", short_id(id), description), false);
        }
    }

    embed
}

pub fn shop_embed(response: &Response) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("🛒 Shop")
        .description("Use `/buy item:<item>` to buy an item.");

    if let Some(items) = response.data.as_ref().and_then(|data| data.as_array()) {
        for item in items {
            let key = item["key"].as_str().unwrap_or("unknown");
            let name = item["name"].as_str().unwrap_or("Unknown item");
            let price = item["price"].as_u64().unwrap_or(0);
            let description = item["description"].as_str().unwrap_or("");
            embed = embed.field(
                format!("{name} — `{key}`"),
                format!("{} coins\n{}", price, description),
                false,
            );
        }
    }

    embed
}

pub fn leaderboard_embed(response: &Response) -> CreateEmbed {
    let mut embed = CreateEmbed::new().title("🏆 Leaderboard");

    if let Some(entries) = response.data.as_ref().and_then(|data| data.as_array()) {
        if entries.is_empty() {
            embed = embed.description(&response.message);
        } else {
            for (index, entry) in entries.iter().enumerate() {
                let player_id = entry["player_id"].as_u64().unwrap_or_default();
                let xp = entry["xp"].as_u64().unwrap_or_default();
                let coins = entry["coins"].as_u64().unwrap_or_default();
                embed = embed.field(
                    format!("#{} <@{}>", index + 1, player_id),
                    format!("XP: {xp} · Coins: {coins}"),
                    false,
                );
            }
        }
    }

    embed
}

pub fn dice_roll_embed(response: &Response) -> CreateEmbed {
    let roll = response
        .data
        .as_ref()
        .and_then(|data| data.get("roll"))
        .and_then(|roll| roll.as_u64());

    CreateEmbed::new()
        .title("🎲 Dice Roll")
        .description(match roll {
            Some(roll) => format!("{}\nYou rolled **{roll}**.", response.message),
            None => response.message.clone(),
        })
}

pub fn event_embed(response: &Response) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("🎉 Random Item Spawn")
        .description(&response.message);

    if let Some(data) = response.data.as_ref().filter(|data| data.is_object()) {
        if let Some(event_id) = data["id"].as_str() {
            embed = embed.field("Event ID", format!("`{event_id}`"), false);
        }
        if let Some(item_name) = data["item_name"].as_str() {
            embed = embed.field("Item", item_name, true);
        }
        if let Some(status) = data["status"].as_str() {
            embed = embed.field("Status", status, true);
        }
        if let Some(expires_at) = data["expires_at"].as_str() {
            embed = embed.field("Expires at", expires_at, false);
        }
    }

    embed
}

pub fn event_id(response: &Response) -> Option<&str> {
    response
        .data
        .as_ref()?
        .get("id")?
        .as_str()
        .filter(|id| !id.is_empty())
}

fn short_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}
