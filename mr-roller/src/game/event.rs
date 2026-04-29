use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::game::{item::Item, player::PlayerId};

pub type EventId = Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActiveEvent {
    pub id: EventId,
    pub kind: EventKind,
    pub status: EventStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EventKind {
    RandomItemSpawn { item: Item },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum EventStatus {
    Active,
    Claimed { player_id: PlayerId },
    Trashed { player_id: PlayerId },
    Expired,
}

impl ActiveEvent {
    pub fn random_item_spawn(item: Item, timeout_seconds: u64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            kind: EventKind::RandomItemSpawn { item },
            status: EventStatus::Active,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(timeout_seconds as i64),
        }
    }

    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        self.status == EventStatus::Active && self.expires_at > now
    }

    pub fn title(&self) -> &'static str {
        match self.kind {
            EventKind::RandomItemSpawn { .. } => "Random Item Spawn",
        }
    }

    pub fn description(&self) -> String {
        match &self.kind {
            EventKind::RandomItemSpawn { item } => format!(
                "A {} has spawned! Claim it before someone else does.",
                item.name()
            ),
        }
    }
}
