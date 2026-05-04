use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::{
    errors::MrRollerError,
    game::{inventory::ItemId, player::PlayerId},
};

#[derive(Debug, Clone, PartialEq)]
pub struct ItemUseRecord {
    pub id: Uuid,
    pub player_id: PlayerId,
    pub item_id: ItemId,
    pub item_name: String,
    pub item_kind: String,
    pub item_json: serde_json::Value,
    pub response_kind: String,
    pub roll: Option<u64>,
    pub used_at: DateTime<Utc>,
}

impl ItemUseRecord {
    pub fn new(
        player_id: PlayerId,
        item_id: ItemId,
        item_name: String,
        item_kind: String,
        item_json: serde_json::Value,
        response_kind: String,
        roll: Option<u64>,
        used_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            player_id,
            item_id,
            item_name,
            item_kind,
            item_json,
            response_kind,
            roll,
            used_at,
        }
    }
}

#[async_trait]
pub trait ItemUseHistoryStore: Send + Sync {
    async fn record_item_use(&self, record: ItemUseRecord) -> Result<(), MrRollerError>;
    async fn list_item_uses(
        &self,
        player_id: PlayerId,
        limit: usize,
    ) -> Result<Vec<ItemUseRecord>, MrRollerError>;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryItemUseHistoryStore {
    records: Arc<Mutex<Vec<ItemUseRecord>>>,
}

impl InMemoryItemUseHistoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ItemUseHistoryStore for InMemoryItemUseHistoryStore {
    async fn record_item_use(&self, record: ItemUseRecord) -> Result<(), MrRollerError> {
        self.records.lock().unwrap().push(record);
        Ok(())
    }

    async fn list_item_uses(
        &self,
        player_id: PlayerId,
        limit: usize,
    ) -> Result<Vec<ItemUseRecord>, MrRollerError> {
        let mut records: Vec<_> = self
            .records
            .lock()
            .unwrap()
            .iter()
            .filter(|record| record.player_id == player_id)
            .cloned()
            .collect();
        records.sort_by(|a, b| b.used_at.cmp(&a.used_at).then_with(|| b.id.cmp(&a.id)));
        records.truncate(limit);
        Ok(records)
    }
}
