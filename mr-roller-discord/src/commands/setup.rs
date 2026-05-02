use poise::CreateReply;
use serenity::all::{ChannelType, GuildChannel, Mentionable, Permissions};

use crate::{Context, Error};

use super::resolve_game;

#[poise::command(slash_command)]
pub async fn setup(
    ctx: Context<'_>,
    #[description = "Text channel that should host this Mr Roller game"] channel: GuildChannel,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            CreateReply::default()
                .content("Mr Roller games must be set up inside a server channel.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    if channel.kind != ChannelType::Text {
        ctx.send(
            CreateReply::default()
                .content("Please choose a text channel for the game.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    if !caller_can_setup(ctx).await {
        ctx.send(
            CreateReply::default()
                .content("You need Administrator, Manage Server, or Manage Channels to set up Mr Roller.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let (discord_game, created) = ctx
        .data()
        .games
        .setup_game(guild_id, channel.id, ctx.author().id)
        .await?;

    let setup_message = if created {
        format!(
            "Mr Roller is ready in {}.\nYou are now an admin for this game. Run `/start` in that channel to receive your starter dice.",
            channel.mention()
        )
    } else {
        format!(
            "A Mr Roller game already exists in {}.\nYou have been added as an admin for that game. Run `/start` in that channel to receive your starter dice if you have not already joined.",
            channel.mention()
        )
    };

    ctx.send(
        CreateReply::default()
            .content(setup_message)
            .ephemeral(true),
    )
    .await?;

    if created {
        if let Err(error) = channel
            .id
            .say(
                &ctx.serenity_context().http,
                "🎲 Mr Roller has started a new game in this channel!\nRun `/start` to join.",
            )
            .await
        {
            ctx.send(
                CreateReply::default()
                    .content(format!(
                        "The game was created, but I could not post in {}: {error}",
                        channel.mention()
                    ))
                    .ephemeral(true),
            )
            .await?;
        }
    }

    tracing::info!(
        game_id = %discord_game.game_id,
        guild_id = guild_id.get(),
        channel_id = channel.id.get(),
        created,
        "Discord game setup completed"
    );
    Ok(())
}

#[poise::command(slash_command)]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let Some(resolved) = resolve_game(ctx).await? else {
        return Ok(());
    };

    let player_count = ctx
        .data()
        .games
        .player_count(resolved.discord_game.game_id)
        .await?;
    ctx.send(
        CreateReply::default()
            .content(format!(
                "Mr Roller is configured for this channel.\nGame ID: `{}`\nEvents: {}\nPlayers: {}",
                resolved.discord_game.game_id,
                if resolved.discord_game.events_enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                player_count
            ))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

#[allow(deprecated)]
async fn caller_can_setup(ctx: Context<'_>) -> bool {
    let Some(member) = ctx.author_member().await else {
        return false;
    };
    let Ok(permissions) = member.permissions(ctx.cache()) else {
        return false;
    };
    permissions.contains(Permissions::ADMINISTRATOR)
        || permissions.contains(Permissions::MANAGE_GUILD)
        || permissions.contains(Permissions::MANAGE_CHANNELS)
}
