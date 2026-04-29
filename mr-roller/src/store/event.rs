use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    errors::MrRollerError,
    game::event::{ActiveEvent, EventId},
};

#[async_trait]
pub trait EventStore: Send + Sync {
    async fn insert_event(&self, event: ActiveEvent) -> Result<(), MrRollerError>;
    async fn get_event(&self, id: EventId) -> Result<ActiveEvent, MrRollerError>;
    async fn update_event(&self, event: ActiveEvent) -> Result<(), MrRollerError>;
    async fn list_events(&self) -> Result<Vec<ActiveEvent>, MrRollerError>;
}

#[derive(Clone, Default)]
pub struct InMemoryEventStore {
    events: Arc<RwLock<HashMap<EventId, ActiveEvent>>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    async fn insert_event(&self, event: ActiveEvent) -> Result<(), MrRollerError> {
        self.events.write().await.insert(event.id, event);
        Ok(())
    }

    async fn get_event(&self, id: EventId) -> Result<ActiveEvent, MrRollerError> {
        self.events
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or_else(|| MrRollerError::Storage("Event not found".to_string()))
    }

    async fn update_event(&self, event: ActiveEvent) -> Result<(), MrRollerError> {
        let mut events = self.events.write().await;
        if !events.contains_key(&event.id) {
            return Err(MrRollerError::Storage("Event not found".to_string()));
        }
        events.insert(event.id, event);
        Ok(())
    }

    async fn list_events(&self) -> Result<Vec<ActiveEvent>, MrRollerError> {
        Ok(self.events.read().await.values().cloned().collect())
    }
}
