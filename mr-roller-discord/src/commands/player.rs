use std::str::FromStr;

use mr_roller::{
    command::{
        BuyItemCommand, InventoryCommand, LeaderboardCommand, ListActiveEventsCommand, ShopCommand,
        ShopItemKind, StartCommand, UseItemCommand,
    },
    game::{inventory::ItemId, player::PlayerId},
    response::Response,
    store::{InventoryStore, PlayerStore},
};
use poise::CreateReply;
use serenity::all::AutocompleteChoice;

use crate::{render::embeds, storage::ResolvedDiscordGame, Context, Error};

fn player_id(ctx: Context<'_>) -> PlayerId {
    PlayerId::new(ctx.author().id.get())
}

async fn resolve_game(ctx: Context<'_>) -> Result<Option<ResolvedDiscordGame>, Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            CreateReply::default()
                .content("Mr Roller games must be used inside a server channel.")
                .ephemeral(true),
        )
        .await?;
        return Ok(None);
    };

    let Some(resolved) = ctx
        .data()
        .games
        .game_for_channel(guild_id, ctx.channel_id())
        .await?
    else {
        ctx.send(
            CreateReply::default()
                .content("No Mr Roller game is configured for this channel.\nAsk a server manager to run `/setup channel:#this-channel`.")
                .ephemeral(true),
        )
        .await?;
        return Ok(None);
    };
    Ok(Some(resolved))
}

async fn send_response(ctx: Context<'_>, response: Response) -> Result<(), Error> {
    let mut reply = CreateReply::default();
    if let Some(embed) = embeds::response_embed(&response) {
        reply = reply.embed(embed);
    } else {
        reply = reply.content(response.message.clone());
    }
    if response.kind == mr_roller::response::ResponseKind::Error {
        reply = reply.ephemeral(true);
    }
    ctx.send(reply).await?;
    Ok(())
}

async fn send_private_response(ctx: Context<'_>, response: Response) -> Result<(), Error> {
    let mut reply = CreateReply::default().ephemeral(true);
    if let Some(embed) = embeds::response_embed(&response) {
        reply = reply.embed(embed);
    } else {
        reply = reply.content(response.message.clone());
    }
    ctx.send(reply).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn start(ctx: Context<'_>) -> Result<(), Error> {
    let Some(resolved) = resolve_game(ctx).await? else {
        return Ok(());
    };
    let response = resolved
        .game
        .execute(StartCommand {
            player_id: player_id(ctx),
        })
        .await;
    send_response(ctx, response).await
}

#[poise::command(slash_command)]
pub async fn inventory(ctx: Context<'_>) -> Result<(), Error> {
    let Some(resolved) = resolve_game(ctx).await? else {
        return Ok(());
    };
    let response = resolved
        .game
        .execute(InventoryCommand {
            player_id: player_id(ctx),
        })
        .await;
    send_private_response(ctx, response).await
}

#[poise::command(slash_command)]
pub async fn shop(ctx: Context<'_>) -> Result<(), Error> {
    let Some(resolved) = resolve_game(ctx).await? else {
        return Ok(());
    };
    let response = resolved.game.execute(ShopCommand).await;
    let coins = resolved
        .store
        .get(player_id(ctx))
        .await
        .ok()
        .map(|player| player.coins);

    let reply = CreateReply::default()
        .embed(embeds::shop_embed_with_coins(&response, coins))
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn buy(
    ctx: Context<'_>,
    #[description = "Shop item key"]
    #[autocomplete = "autocomplete_shop_item"]
    item: String,
) -> Result<(), Error> {
    let item = match ShopItemKind::from_str(&item) {
        Ok(item) => item,
        Err(error) => {
            ctx.send(CreateReply::default().content(error).ephemeral(true))
                .await?;
            return Ok(());
        }
    };
    let Some(resolved) = resolve_game(ctx).await? else {
        return Ok(());
    };
    let response = resolved
        .game
        .execute(BuyItemCommand {
            player_id: player_id(ctx),
            item,
        })
        .await;
    send_response(ctx, response).await
}

#[poise::command(slash_command)]
pub async fn leaderboard(ctx: Context<'_>) -> Result<(), Error> {
    let Some(resolved) = resolve_game(ctx).await? else {
        return Ok(());
    };
    let response = resolved
        .game
        .execute(LeaderboardCommand { limit: 10 })
        .await;
    send_response(ctx, response).await
}

#[poise::command(slash_command, rename = "use")]
pub async fn use_item(
    ctx: Context<'_>,
    #[description = "Inventory item"]
    #[autocomplete = "autocomplete_inventory_item"]
    item: String,
) -> Result<(), Error> {
    let item_id: ItemId = match item.parse() {
        Ok(item_id) => item_id,
        Err(_) => {
            ctx.send(
                CreateReply::default()
                    .content("Invalid item ID.")
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
    };
    let Some(resolved) = resolve_game(ctx).await? else {
        return Ok(());
    };
    let response = resolved
        .game
        .execute(UseItemCommand {
            player_id: player_id(ctx),
            item_id,
        })
        .await;
    send_response(ctx, response).await
}

#[poise::command(slash_command)]
pub async fn events(ctx: Context<'_>) -> Result<(), Error> {
    let Some(resolved) = resolve_game(ctx).await? else {
        return Ok(());
    };
    let response = resolved.game.execute(ListActiveEventsCommand).await;
    send_response(ctx, response).await
}

#[poise::command(slash_command, subcommands("claim", "trash"))]
pub async fn event(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command)]
pub async fn claim(
    ctx: Context<'_>,
    #[description = "Event ID"]
    #[autocomplete = "autocomplete_event_id"]
    event: String,
) -> Result<(), Error> {
    let Some(event_id) = parse_uuid_reply(ctx, &event).await? else {
        return Ok(());
    };
    let Some(resolved) = resolve_game(ctx).await? else {
        return Ok(());
    };
    let response = resolved
        .game
        .execute(mr_roller::command::ClaimEventCommand {
            player_id: player_id(ctx),
            event_id,
        })
        .await;
    send_response(ctx, response).await
}

#[poise::command(slash_command)]
pub async fn trash(
    ctx: Context<'_>,
    #[description = "Event ID"]
    #[autocomplete = "autocomplete_event_id"]
    event: String,
) -> Result<(), Error> {
    let Some(event_id) = parse_uuid_reply(ctx, &event).await? else {
        return Ok(());
    };
    let Some(resolved) = resolve_game(ctx).await? else {
        return Ok(());
    };
    let response = resolved
        .game
        .execute(mr_roller::command::TrashEventCommand {
            player_id: player_id(ctx),
            event_id,
        })
        .await;
    send_response(ctx, response).await
}

async fn parse_uuid_reply(ctx: Context<'_>, value: &str) -> Result<Option<uuid::Uuid>, Error> {
    match value.parse() {
        Ok(id) => Ok(Some(id)),
        Err(_) => {
            ctx.send(
                CreateReply::default()
                    .content("Invalid event ID.")
                    .ephemeral(true),
            )
            .await?;
            Ok(None)
        }
    }
}

async fn autocomplete_shop_item(_ctx: Context<'_>, partial: &str) -> Vec<String> {
    ShopItemKind::keys()
        .into_iter()
        .filter(|key| key.contains(partial))
        .map(str::to_string)
        .collect()
}

async fn autocomplete_inventory_item(ctx: Context<'_>, partial: &str) -> Vec<AutocompleteChoice> {
    let player_id = player_id(ctx);
    let partial = partial.to_ascii_lowercase();
    let Some(guild_id) = ctx.guild_id() else {
        return Vec::new();
    };
    let Ok(Some(resolved)) = ctx
        .data()
        .games
        .game_for_channel(guild_id, ctx.channel_id())
        .await
    else {
        return Vec::new();
    };
    let Ok(items) = resolved.store.list_items(player_id).await else {
        return Vec::new();
    };

    items
        .into_iter()
        .filter_map(|(id, item)| {
            let id = id.to_string();
            let name = item.name().to_string();
            let label = format!("{} — {}", name, &id[..8]);
            let matches = partial.is_empty()
                || name.to_ascii_lowercase().contains(&partial)
                || id.contains(&partial);
            matches.then_some(AutocompleteChoice::new(label, id))
        })
        .take(25)
        .collect()
}

async fn autocomplete_event_id(ctx: Context<'_>, partial: &str) -> Vec<AutocompleteChoice> {
    let Some(guild_id) = ctx.guild_id() else {
        return Vec::new();
    };
    let Ok(Some(resolved)) = ctx
        .data()
        .games
        .game_for_channel(guild_id, ctx.channel_id())
        .await
    else {
        return Vec::new();
    };
    let response = resolved.game.execute(ListActiveEventsCommand).await;
    let Some(events) = response.data.and_then(|data| data.as_array().cloned()) else {
        return Vec::new();
    };
    events
        .into_iter()
        .filter_map(|event| {
            let id = event["id"].as_str()?.to_string();
            if !partial.is_empty() && !id.contains(partial) {
                return None;
            }
            let item = event["item_name"].as_str().unwrap_or("event");
            Some(AutocompleteChoice::new(
                format!("{} — {}", item, &id[..8]),
                id,
            ))
        })
        .take(25)
        .collect()
}
