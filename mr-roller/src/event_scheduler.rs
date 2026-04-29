use std::{future::Future, sync::Arc, time::Duration};

use crate::{
    command::MaybeSpawnRandomItemEventCommand,
    game::Game,
    response::{Response, ResponseKind},
};

/// Frontend-agnostic random event scheduler.
///
/// The scheduler only decides when to ask the core game to maybe spawn an event.
/// It does not know how events are published. Discord, CLI, or a future web
/// server can provide an `on_spawned` handler to render/send the announcement.
pub struct EventScheduler {
    check_interval: Duration,
}

impl EventScheduler {
    pub fn new(check_interval: Duration) -> Self {
        Self { check_interval }
    }

    pub fn from_seconds(check_interval_seconds: u64) -> Self {
        Self::new(Duration::from_secs(check_interval_seconds.max(1)))
    }

    pub async fn tick(&self, game: &Game) -> Option<Response> {
        let response = game.execute(MaybeSpawnRandomItemEventCommand).await;
        is_spawned_event_response(&response).then_some(response)
    }

    pub async fn run<F, Fut>(&self, game: Arc<Game>, mut on_spawned: F)
    where
        F: FnMut(Response) -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut interval = tokio::time::interval(self.check_interval);
        // `tokio::time::interval` ticks immediately the first time. Consume that
        // initial tick so event checks happen after the configured interval.
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Some(response) = self.tick(&game).await {
                on_spawned(response).await;
            }
        }
    }
}

fn is_spawned_event_response(response: &Response) -> bool {
    response.kind == ResponseKind::Event
        && response.message == "Random item event spawned."
        && response.data.as_ref().is_some_and(|data| data.is_object())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::EventsConfig,
        game::Game,
        store::{
            InMemoryEventStore, InMemoryInventoryStore, InMemoryLeaderboardStore,
            InMemoryPlayerStore,
        },
    };

    #[tokio::test]
    async fn tick_returns_spawned_event_when_chance_is_one() {
        let mut events_config = EventsConfig::default();
        events_config.spawn_chance_per_check = 1.0;

        let game = Game::with_event_store(
            Arc::new(InMemoryPlayerStore::new()),
            Arc::new(InMemoryInventoryStore::new()),
            Arc::new(InMemoryLeaderboardStore::new()),
            Arc::new(InMemoryEventStore::new()),
            Vec::new(),
            events_config,
        );
        let scheduler = EventScheduler::from_seconds(60);

        let response = scheduler.tick(&game).await;
        assert!(response.is_some());
    }
}
