use mr_roller::event_scheduler::EventScheduler;
use serenity::all::Http;
use std::{sync::Arc, time::Duration};
use tracing::{error, info};

use crate::storage::DiscordGameRegistry;

use super::publisher::publish_event_response;

pub fn spawn_event_scheduler(
    registry: DiscordGameRegistry,
    http: Arc<Http>,
    check_interval_seconds: u64,
) {
    tokio::spawn(async move {
        let scheduler = EventScheduler::from_seconds(check_interval_seconds);
        let mut interval =
            tokio::time::interval(Duration::from_secs(check_interval_seconds.max(1)));
        interval.tick().await;
        info!(
            check_interval_seconds,
            "starting multi-game Discord event scheduler"
        );

        loop {
            interval.tick().await;
            let discord_games = match registry.list_games_with_events_enabled().await {
                Ok(games) => games,
                Err(error) => {
                    error!(?error, "failed to list Discord games for scheduler");
                    continue;
                }
            };

            for discord_game in discord_games {
                let resolved = match registry.game_for_id(discord_game.game_id).await {
                    Ok(Some(resolved)) => resolved,
                    Ok(None) => continue,
                    Err(error) => {
                        error!(?error, game_id = %discord_game.game_id, "failed to resolve scheduled game");
                        continue;
                    }
                };

                if let Some(response) = scheduler.tick(&resolved.game).await {
                    if let Err(error) =
                        publish_event_response(&http, discord_game.channel_id, &response).await
                    {
                        error!(
                            ?error,
                            game_id = %discord_game.game_id,
                            channel_id = discord_game.channel_id.get(),
                            "failed to publish scheduled event"
                        );
                    }
                }
            }
        }
    });
}
