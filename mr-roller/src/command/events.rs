use async_trait::async_trait;
use chrono::Utc;
use rand::{thread_rng, Rng};

use crate::{
    command::{
        admin::{require_admin, AdminItemKind},
        Command, Context,
    },
    errors::MrRollerError,
    game::{
        event::{ActiveEvent, EventId, EventKind, EventStatus},
        item::Item,
        player::PlayerId,
    },
    response::Response,
};

fn event_to_json(event: &ActiveEvent) -> serde_json::Value {
    let item_name = match &event.kind {
        EventKind::RandomItemSpawn { item } => Some(item.name().to_string()),
    };

    let (status, actor_id) = match &event.status {
        EventStatus::Active => ("active", None),
        EventStatus::Claimed { player_id } => ("claimed", Some(player_id.0)),
        EventStatus::Trashed { player_id } => ("trashed", Some(player_id.0)),
        EventStatus::Expired => ("expired", None),
    };

    serde_json::json!({
        "id": event.id.to_string(),
        "title": event.title(),
        "description": event.description(),
        "status": status,
        "actor_id": actor_id,
        "item_name": item_name,
        "expires_at": event.expires_at.to_rfc3339(),
    })
}

fn configured_random_item(ctx: &Context<'_>) -> Result<Item, MrRollerError> {
    let candidates: Vec<_> = ctx
        .event_config
        .random_item_spawn
        .items
        .iter()
        .filter(|entry| entry.weight > 0)
        .collect();

    if candidates.is_empty() {
        return Err(MrRollerError::Storage(
            "Random item spawn has no configured items".to_string(),
        ));
    }

    let total_weight: u32 = candidates.iter().map(|entry| entry.weight).sum();
    let mut roll = thread_rng().gen_range(0..total_weight);
    for entry in candidates {
        if roll < entry.weight {
            let kind = entry
                .kind
                .parse::<AdminItemKind>()
                .map_err(MrRollerError::Storage)?;
            return Ok(kind.into());
        }
        roll -= entry.weight;
    }

    Err(MrRollerError::Storage(
        "Failed to choose random event item".to_string(),
    ))
}

// ── Events ─────────────────────────────────────────────────────────────────

pub struct ListActiveEventsCommand;

#[async_trait]
impl Command for ListActiveEventsCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        let now = Utc::now();
        let events: Vec<_> = ctx
            .events
            .list_events()
            .await?
            .into_iter()
            .filter(|event| event.is_active(now))
            .map(|event| event_to_json(&event))
            .collect();

        if events.is_empty() {
            Ok(Response::event("No active events.", serde_json::json!([])))
        } else {
            Ok(Response::event("Active events:", serde_json::json!(events)))
        }
    }
}

pub struct MaybeSpawnRandomItemEventCommand;

#[async_trait]
impl Command for MaybeSpawnRandomItemEventCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        if !ctx.event_config.enabled || !ctx.event_config.random_item_spawn.enabled {
            return Ok(Response::event(
                "Events are disabled.",
                serde_json::json!(null),
            ));
        }

        let active_count = ctx
            .events
            .list_events()
            .await?
            .into_iter()
            .filter(|event| event.is_active(Utc::now()))
            .count();
        if active_count >= ctx.event_config.max_active_events {
            return Ok(Response::event(
                "No event spawned because the active event limit has been reached.",
                serde_json::json!(null),
            ));
        }

        if !thread_rng().gen_bool(ctx.event_config.spawn_chance_per_check) {
            return Ok(Response::event(
                "No event spawned.",
                serde_json::json!(null),
            ));
        }

        spawn_random_item_event(ctx).await
    }
}

pub struct SpawnRandomItemEventCommand {
    pub admin_id: PlayerId,
}

#[async_trait]
impl Command for SpawnRandomItemEventCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        if let Some(response) = require_admin(ctx, self.admin_id).await? {
            return Ok(response);
        }
        spawn_random_item_event(ctx).await
    }
}

async fn spawn_random_item_event(ctx: &Context<'_>) -> Result<Response, MrRollerError> {
    let item = configured_random_item(ctx)?;
    let event =
        ActiveEvent::random_item_spawn(item, ctx.event_config.random_item_spawn.timeout_seconds);
    let data = event_to_json(&event);
    ctx.events.insert_event(event).await?;
    Ok(Response::event("Random item event spawned.", data))
}

pub struct ClaimEventCommand {
    pub player_id: PlayerId,
    pub event_id: EventId,
}

#[async_trait]
impl Command for ClaimEventCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        ctx.players.get(self.player_id).await?;
        let mut event = ctx.events.get_event(self.event_id).await?;
        if !event.is_active(Utc::now()) {
            return Ok(Response::error("This event is no longer active."));
        }

        match &event.kind {
            EventKind::RandomItemSpawn { item } => {
                ctx.inventory.add_item(self.player_id, item.clone()).await?;
            }
        }

        event.status = EventStatus::Claimed {
            player_id: self.player_id,
        };
        ctx.events.update_event(event.clone()).await?;
        Ok(Response::event("Event claimed.", event_to_json(&event)))
    }
}

pub struct TrashEventCommand {
    pub player_id: PlayerId,
    pub event_id: EventId,
}

#[async_trait]
impl Command for TrashEventCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        ctx.players.get(self.player_id).await?;
        let mut event = ctx.events.get_event(self.event_id).await?;
        if !event.is_active(Utc::now()) {
            return Ok(Response::error("This event is no longer active."));
        }

        event.status = EventStatus::Trashed {
            player_id: self.player_id,
        };
        ctx.events.update_event(event.clone()).await?;
        Ok(Response::event("Event trashed.", event_to_json(&event)))
    }
}
