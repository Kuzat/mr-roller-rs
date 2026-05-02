use async_trait::async_trait;

use crate::{
    command::{common::leaderboard_score, Command, Context},
    errors::MrRollerError,
    game::{
        item::{
            dice::{basic_dice::BasicDice, cursed_dice::CursedDice, lucky_dice::LuckyDice},
            Item,
        },
        player::PlayerId,
    },
    response::Response,
};

// ── Shop helpers ───────────────────────────────────────────────────────────

/// Item types players can buy from the shop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopItemKind {
    StarterDice,
    RegularDice,
    LuckyDice,
    CursedDice,
}

pub struct ShopCatalogEntry {
    pub key: &'static str,
    pub item: ShopItemKind,
    pub price: u64,
}

const SHOP_CATALOG: &[ShopCatalogEntry] = &[
    ShopCatalogEntry {
        key: "starter_dice",
        item: ShopItemKind::StarterDice,
        price: 5,
    },
    ShopCatalogEntry {
        key: "regular_dice",
        item: ShopItemKind::RegularDice,
        price: 25,
    },
    ShopCatalogEntry {
        key: "lucky_dice",
        item: ShopItemKind::LuckyDice,
        price: 100,
    },
    ShopCatalogEntry {
        key: "cursed_dice",
        item: ShopItemKind::CursedDice,
        price: 50,
    },
];

impl ShopItemKind {
    pub fn keys() -> Vec<&'static str> {
        SHOP_CATALOG.iter().map(|entry| entry.key).collect()
    }

    pub fn catalog_entry(&self) -> &'static ShopCatalogEntry {
        SHOP_CATALOG
            .iter()
            .find(|entry| entry.item == *self)
            .expect("shop item must exist in catalog")
    }
}

impl std::str::FromStr for ShopItemKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "starter" | "starter_dice" => Ok(Self::StarterDice),
            "regular" | "regular_dice" | "basic" | "basic_dice" => Ok(Self::RegularDice),
            "lucky" | "lucky_dice" => Ok(Self::LuckyDice),
            "cursed" | "cursed_dice" => Ok(Self::CursedDice),
            other => Err(format!(
                "Unknown shop item '{}'. Available items: {}",
                other,
                Self::keys().join(", ")
            )),
        }
    }
}

impl From<ShopItemKind> for Item {
    fn from(kind: ShopItemKind) -> Self {
        match kind {
            ShopItemKind::StarterDice => Item::BasicDice(BasicDice::starter_dice()),
            ShopItemKind::RegularDice => Item::BasicDice(BasicDice::regular_dice()),
            ShopItemKind::LuckyDice => Item::LuckyDice(LuckyDice::new()),
            ShopItemKind::CursedDice => Item::CursedDice(CursedDice::new()),
        }
    }
}

// ── Shop ───────────────────────────────────────────────────────────────────

pub struct ShopCommand;

#[async_trait]
impl Command for ShopCommand {
    type Output = Response;

    async fn execute(self, _ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        let entries: Vec<serde_json::Value> = SHOP_CATALOG
            .iter()
            .map(|entry| {
                let item: Item = entry.item.into();
                serde_json::json!({
                    "key": entry.key,
                    "name": item.name(),
                    "description": item.description(),
                    "price": entry.price,
                })
            })
            .collect();

        Ok(Response::shop("Shop:", serde_json::json!(entries)))
    }
}

// ── BuyItem ────────────────────────────────────────────────────────────────

pub struct BuyItemCommand {
    pub player_id: PlayerId,
    pub item: ShopItemKind,
}

#[async_trait]
impl Command for BuyItemCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        let mut player = ctx.players.get(self.player_id).await?;
        let catalog_entry = self.item.catalog_entry();

        if player.coins < catalog_entry.price {
            return Ok(Response::error(format!(
                "You need {} coins to buy {} but only have {}.",
                catalog_entry.price, catalog_entry.key, player.coins
            )));
        }

        let item: Item = self.item.into();
        let item_name = item.name().to_string();
        player.coins -= catalog_entry.price;

        ctx.players.update(player.clone()).await?;
        ctx.inventory.add_item(self.player_id, item).await?;
        ctx.leaderboard
            .update_score(self.player_id, leaderboard_score(&player))
            .await?;

        Ok(Response::success(format!(
            "Bought {} for {} coins. You now have {} coins.",
            item_name, catalog_entry.price, player.coins
        )))
    }
}
