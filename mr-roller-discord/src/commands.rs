pub mod admin;
pub mod player;
pub mod setup;

use poise::CreateReply;

use crate::{storage::ResolvedDiscordGame, Context, Data, Error};

pub(crate) async fn resolve_game(ctx: Context<'_>) -> Result<Option<ResolvedDiscordGame>, Error> {
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

pub fn commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        player::ping(),
        setup::setup(),
        setup::status(),
        player::start(),
        player::inventory(),
        player::shop(),
        player::buy(),
        player::leaderboard(),
        player::use_item(),
        player::events(),
        player::event(),
        admin::admin(),
    ]
}
