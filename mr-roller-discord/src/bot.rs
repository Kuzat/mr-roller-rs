use mr_roller::{
    command::{ClaimEventCommand, TrashEventCommand},
    game::player::PlayerId,
};
use poise::serenity_prelude as serenity;
use serenity::all::{
    ActivityData, CreateInteractionResponse, CreateInteractionResponseMessage, FullEvent,
    GatewayIntents, Interaction, OnlineStatus,
};
use tracing::{error, info};

use crate::{
    commands,
    config::DiscordRuntimeConfig,
    events::{publisher::update_event_interaction_message, scheduler::spawn_event_scheduler},
    render::components::{parse_event_custom_id, EventButtonAction},
    Data, Error,
};

pub async fn run_bot(
    config: DiscordRuntimeConfig,
    data: Data,
    check_interval_seconds: u64,
) -> Result<(), Error> {
    let token = config.token.clone();
    let guild_id = config.guild_id;
    let scheduler_game = data.game.clone();
    let home_channel_id = data.home_channel_id;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::commands(),
            event_handler: |ctx, event, _framework, data| {
                Box::pin(async move { handle_event(ctx, event, data).await })
            },
            ..Default::default()
        })
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                info!(user = %ready.user.name, "Discord bot connected");
                ctx.set_presence(
                    Some(ActivityData::playing("Mr Roller 🎲")),
                    OnlineStatus::Online,
                );
                if let Err(error) = home_channel_id
                    .say(&ctx.http, "🎲 Mr Roller is online and ready.")
                    .await
                {
                    error!(?error, "failed to send Discord startup message");
                }
                if let Some(guild_id) = guild_id {
                    poise::builtins::register_in_guild(
                        ctx,
                        &framework.options().commands,
                        guild_id,
                    )
                    .await?;
                    info!(guild_id = guild_id.get(), "registered guild slash commands");
                } else {
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    info!("registered global slash commands");
                }
                spawn_event_scheduler(
                    scheduler_game,
                    ctx.http.clone(),
                    home_channel_id,
                    check_interval_seconds,
                );
                Ok(data)
            })
        })
        .build();

    let intents = GatewayIntents::non_privileged();
    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await?;

    let shard_manager = client.shard_manager.clone();

    tokio::select! {
        result = client.start() => {
            result?;
        }
        _ = shutdown_signal() => {
            info!("shutdown signal received; closing Discord gateway shards");
            shard_manager.shutdown_all().await;
            info!("Discord bot shutdown complete");
        }
    }

    Ok(())
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut terminate = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
}

async fn handle_event(
    ctx: &serenity::Context,
    event: &FullEvent,
    data: &Data,
) -> Result<(), Error> {
    let FullEvent::InteractionCreate { interaction } = event else {
        return Ok(());
    };
    let Interaction::Component(component) = interaction else {
        return Ok(());
    };
    let Some((action, event_id)) = parse_event_custom_id(&component.data.custom_id) else {
        return Ok(());
    };

    let player_id = PlayerId::new(component.user.id.get());
    let response = match action {
        EventButtonAction::Claim => {
            data.game
                .execute(ClaimEventCommand {
                    player_id,
                    event_id,
                })
                .await
        }
        EventButtonAction::Trash => {
            data.game
                .execute(TrashEventCommand {
                    player_id,
                    event_id,
                })
                .await
        }
    };

    if response.kind == mr_roller::response::ResponseKind::Error {
        if let Err(error) = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(response.message)
                        .ephemeral(true),
                ),
            )
            .await
        {
            error!(?error, "failed to send event button error response");
        }
        return Ok(());
    }

    update_event_interaction_message(&ctx.http, component, &response).await?;
    Ok(())
}
