use std::str::FromStr;

use mr_roller::{
    command::{
        AdminAdjustCoinsCommand, AdminGiveItemCommand, AdminItemKind, AdminSetAdminCommand,
        SpawnRandomItemEventCommand,
    },
    game::player::PlayerId,
};
use poise::CreateReply;
use serenity::all::{Mentionable, User};

use crate::{
    events::publisher::publish_event_response, render::embeds, storage::ResolvedDiscordGame,
    Context, Error,
};

fn author_id(ctx: Context<'_>) -> PlayerId {
    PlayerId::new(ctx.author().id.get())
}

fn discord_player_id(user: &User) -> PlayerId {
    PlayerId::new(user.id.get())
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

#[poise::command(
    slash_command,
    subcommands("give", "coins", "set_admin", "admin_event")
)]
pub async fn admin(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command)]
pub async fn give(
    ctx: Context<'_>,
    #[description = "Discord user"] user: User,
    #[description = "Item to grant"]
    #[autocomplete = "autocomplete_admin_item"]
    item: String,
    #[description = "Show the result to everyone instead of only you"] show_all: Option<bool>,
) -> Result<(), Error> {
    let item = match AdminItemKind::from_str(&item) {
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
        .execute(AdminGiveItemCommand {
            admin_id: author_id(ctx),
            target_player_id: discord_player_id(&user),
            item,
        })
        .await;
    send_admin_response(
        ctx,
        mention_target(response, &user),
        show_all.unwrap_or(false),
    )
    .await
}

#[poise::command(slash_command)]
pub async fn coins(
    ctx: Context<'_>,
    #[description = "Discord user"] user: User,
    #[description = "Coin delta. Negative values remove coins."] amount: i64,
    #[description = "Show the result to everyone instead of only you"] show_all: Option<bool>,
) -> Result<(), Error> {
    let Some(resolved) = resolve_game(ctx).await? else {
        return Ok(());
    };
    let response = resolved
        .game
        .execute(AdminAdjustCoinsCommand {
            admin_id: author_id(ctx),
            target_player_id: discord_player_id(&user),
            amount,
        })
        .await;
    send_admin_response(
        ctx,
        mention_target(response, &user),
        show_all.unwrap_or(false),
    )
    .await
}

#[poise::command(slash_command, rename = "set-admin")]
pub async fn set_admin(
    ctx: Context<'_>,
    #[description = "Discord user"] user: User,
    #[description = "Whether the user should be an admin"] is_admin: bool,
    #[description = "Show the result to everyone instead of only you"] show_all: Option<bool>,
) -> Result<(), Error> {
    let Some(resolved) = resolve_game(ctx).await? else {
        return Ok(());
    };
    let response = resolved
        .game
        .execute(AdminSetAdminCommand {
            admin_id: author_id(ctx),
            target_player_id: discord_player_id(&user),
            is_admin,
        })
        .await;
    send_admin_response(
        ctx,
        mention_target(response, &user),
        show_all.unwrap_or(false),
    )
    .await
}

#[poise::command(slash_command, rename = "event", subcommands("spawn_random_item"))]
pub async fn admin_event(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, rename = "spawn-random-item")]
pub async fn spawn_random_item(ctx: Context<'_>) -> Result<(), Error> {
    let Some(resolved) = resolve_game(ctx).await? else {
        return Ok(());
    };
    let response = resolved
        .game
        .execute(SpawnRandomItemEventCommand {
            admin_id: author_id(ctx),
        })
        .await;

    if response.kind == mr_roller::response::ResponseKind::Event
        && embeds::event_id(&response).is_some()
    {
        publish_event_response(
            &ctx.serenity_context().http,
            resolved.discord_game.channel_id,
            &response,
        )
        .await?;
        ctx.send(
            CreateReply::default()
                .content("Random item event posted.")
                .ephemeral(true),
        )
        .await?;
        Ok(())
    } else {
        send_admin_response(ctx, response, false).await
    }
}

async fn send_admin_response(
    ctx: Context<'_>,
    response: mr_roller::response::Response,
    show_all: bool,
) -> Result<(), Error> {
    let mut reply = CreateReply::default().ephemeral(!show_all);
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

fn mention_target(
    mut response: mr_roller::response::Response,
    user: &User,
) -> mr_roller::response::Response {
    response.message = response
        .message
        .replace(&user.id.get().to_string(), &user.mention().to_string());
    response
}

async fn autocomplete_admin_item(_ctx: Context<'_>, partial: &str) -> Vec<String> {
    AdminItemKind::keys()
        .iter()
        .copied()
        .filter(|key| key.contains(partial))
        .map(str::to_string)
        .collect()
}
