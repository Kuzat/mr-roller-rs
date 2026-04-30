use std::sync::Arc;

use mr_roller::{event_scheduler::EventScheduler, game::Game};
use serenity::all::{ChannelId, Http};
use tracing::{error, info};

use super::publisher::publish_event_response;

pub fn spawn_event_scheduler(
    game: Arc<Game>,
    http: Arc<Http>,
    home_channel_id: ChannelId,
    check_interval_seconds: u64,
) {
    tokio::spawn(async move {
        let scheduler = EventScheduler::from_seconds(check_interval_seconds);
        info!(check_interval_seconds, "starting Discord event scheduler");
        scheduler
            .run(game, move |response| {
                let http = http.clone();
                async move {
                    if let Err(error) =
                        publish_event_response(&http, home_channel_id, &response).await
                    {
                        error!(?error, "failed to publish scheduled event");
                    }
                }
            })
            .await;
    });
}
