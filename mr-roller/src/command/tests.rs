use super::*;
use crate::{
    cooldown::CooldownConfig,
    game::{
        event::EventStatus,
        item::{dice::basic_dice::BasicDice, tokens::reroll_token::RerollToken, Item},
        player::PlayerId,
    },
    response::ResponseKind,
    store::{
        EventStore, InMemoryEventStore, InMemoryInventoryStore, InMemoryLeaderboardStore,
        InMemoryPlayerStore, InventoryStore, LeaderboardStore, PlayerStore,
    },
};

fn make_context<'a>(
    players: &'a InMemoryPlayerStore,
    inventory: &'a InMemoryInventoryStore,
    leaderboard: &'a InMemoryLeaderboardStore,
    cooldown: &'a CooldownConfig,
) -> Context<'a> {
    Context {
        players,
        inventory,
        leaderboard,
        events: Box::leak(Box::new(InMemoryEventStore::new())),
        cooldown,
        bootstrap_admin_ids: &[],
        event_config: Box::leak(Box::new(crate::config::EventsConfig::default())),
    }
}

// ── StartCommand tests ──────────────────────────────────────────────

#[tokio::test]
async fn test_start_new_player() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    let cmd = StartCommand {
        player_id: PlayerId::new(1),
    };
    let resp = cmd.execute(&ctx).await.unwrap();
    assert_eq!(resp.kind, ResponseKind::Success);
    assert!(resp.message.contains("starter dice"));

    // Player exists
    assert!(players.contains(PlayerId::new(1)).await.unwrap());

    // Has one item (starter dice)
    let items = inventory.list_items(PlayerId::new(1)).await.unwrap();
    assert_eq!(items.len(), 1);
}

#[tokio::test]
async fn test_start_bootstrap_admin() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let bootstrap_admin_ids = [PlayerId::new(7)];
    let ctx = Context {
        players: &players,
        inventory: &inventory,
        leaderboard: &leaderboard,
        events: Box::leak(Box::new(InMemoryEventStore::new())),
        cooldown: &cooldown,
        bootstrap_admin_ids: &bootstrap_admin_ids,
        event_config: Box::leak(Box::new(crate::config::EventsConfig::default())),
    };

    StartCommand {
        player_id: PlayerId::new(7),
    }
    .execute(&ctx)
    .await
    .unwrap();

    assert!(players.get(PlayerId::new(7)).await.unwrap().is_admin);
}

#[tokio::test]
async fn test_start_promotes_existing_bootstrap_admin() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let bootstrap_admin_ids = [PlayerId::new(7)];
    let ctx = Context {
        players: &players,
        inventory: &inventory,
        leaderboard: &leaderboard,
        events: Box::leak(Box::new(InMemoryEventStore::new())),
        cooldown: &cooldown,
        bootstrap_admin_ids: &bootstrap_admin_ids,
        event_config: Box::leak(Box::new(crate::config::EventsConfig::default())),
    };

    players
        .insert(crate::game::player::Player::new(PlayerId::new(7)))
        .await
        .unwrap();

    let resp = StartCommand {
        player_id: PlayerId::new(7),
    }
    .execute(&ctx)
    .await
    .unwrap();

    assert_eq!(resp.kind, ResponseKind::Success);
    assert!(players.get(PlayerId::new(7)).await.unwrap().is_admin);
}

#[tokio::test]
async fn test_start_existing_admin_without_starter_dice_grants_starter_dice() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);
    let player_id = PlayerId::new(7);
    let mut setup_admin = crate::game::player::Player::new(player_id);
    setup_admin.is_admin = true;
    players.insert(setup_admin).await.unwrap();

    let resp = StartCommand { player_id }.execute(&ctx).await.unwrap();

    assert_eq!(resp.kind, ResponseKind::Success);
    assert!(resp.message.contains("starter dice"));
    let items = inventory.list_items(player_id).await.unwrap();
    assert_eq!(items.len(), 1);
    assert!(matches!(&items[0].1, Item::BasicDice(dice) if dice.name == "Starter Dice"));
    assert!(players.get(player_id).await.unwrap().is_admin);
}

#[tokio::test]
async fn test_first_dice_use_completes_tutorial() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);
    let player_id = PlayerId::new(1);

    StartCommand { player_id }.execute(&ctx).await.unwrap();
    let item_id = inventory.list_items(player_id).await.unwrap()[0].0;

    let resp = UseItemCommand { player_id, item_id }
        .execute(&ctx)
        .await
        .unwrap();

    assert_eq!(resp.kind, ResponseKind::DiceRoll);
    assert!(resp.message.contains("You earned"));
    assert!(resp.message.contains("/shop"));
    assert!(resp.message.contains("/leaderboard"));
    assert!(players.get(player_id).await.unwrap().tutorial_completed);
}

#[tokio::test]
async fn test_bootstrap_admin_can_use_admin_command_even_before_persisted_flag() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let bootstrap_admin_ids = [PlayerId::new(7)];
    let ctx = Context {
        players: &players,
        inventory: &inventory,
        leaderboard: &leaderboard,
        events: Box::leak(Box::new(InMemoryEventStore::new())),
        cooldown: &cooldown,
        bootstrap_admin_ids: &bootstrap_admin_ids,
        event_config: Box::leak(Box::new(crate::config::EventsConfig::default())),
    };

    players
        .insert(crate::game::player::Player::new(PlayerId::new(7)))
        .await
        .unwrap();

    let resp = AdminHelpCommand {
        player_id: PlayerId::new(7),
    }
    .execute(&ctx)
    .await
    .unwrap();

    assert_eq!(resp.kind, ResponseKind::Success);
}

#[tokio::test]
async fn test_start_duplicate_player() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    // First start succeeds
    StartCommand {
        player_id: PlayerId::new(1),
    }
    .execute(&ctx)
    .await
    .unwrap();

    // Second start returns error
    let resp = StartCommand {
        player_id: PlayerId::new(1),
    }
    .execute(&ctx)
    .await
    .unwrap();
    assert_eq!(resp.kind, ResponseKind::Error);
}

// ── UseItemCommand tests ────────────────────────────────────────────

#[tokio::test]
async fn test_use_item_dice_roll() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    // Start a player
    StartCommand {
        player_id: PlayerId::new(1),
    }
    .execute(&ctx)
    .await
    .unwrap();

    // Give them a regular dice
    let item_id = inventory
        .add_item(PlayerId::new(1), Item::BasicDice(BasicDice::regular_dice()))
        .await
        .unwrap();

    let resp = UseItemCommand {
        player_id: PlayerId::new(1),
        item_id,
    }
    .execute(&ctx)
    .await
    .unwrap();

    assert_eq!(resp.kind, ResponseKind::DiceRoll);
    let data = resp.data.as_ref().unwrap();

    let player = players.get(PlayerId::new(1)).await.unwrap();
    let roll = data["roll"].as_u64().unwrap();
    assert_eq!(player.coins, roll);
    assert_eq!(player.xp, roll);

    // Leaderboard should have an entry
    let scores = leaderboard.get_scores(10).await.unwrap();
    assert_eq!(scores.len(), 1);
}

#[tokio::test]
async fn test_second_dice_roll_same_day_is_blocked() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    StartCommand {
        player_id: PlayerId::new(1),
    }
    .execute(&ctx)
    .await
    .unwrap();

    let item_id = inventory
        .add_item(PlayerId::new(1), Item::BasicDice(BasicDice::regular_dice()))
        .await
        .unwrap();

    let first = UseItemCommand {
        player_id: PlayerId::new(1),
        item_id,
    }
    .execute(&ctx)
    .await
    .unwrap();
    assert_eq!(first.kind, ResponseKind::DiceRoll);

    let second = UseItemCommand {
        player_id: PlayerId::new(1),
        item_id,
    }
    .execute(&ctx)
    .await
    .unwrap();
    assert_eq!(second.kind, ResponseKind::Error);
    assert!(second.message.contains("cooldown"));
}

#[tokio::test]
async fn test_reroll_token_clears_roll_cooldown() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    StartCommand {
        player_id: PlayerId::new(1),
    }
    .execute(&ctx)
    .await
    .unwrap();

    let dice_id = inventory
        .add_item(PlayerId::new(1), Item::BasicDice(BasicDice::regular_dice()))
        .await
        .unwrap();
    let token_id = inventory
        .add_item(PlayerId::new(1), Item::RerollToken(RerollToken::new()))
        .await
        .unwrap();

    let first = UseItemCommand {
        player_id: PlayerId::new(1),
        item_id: dice_id,
    }
    .execute(&ctx)
    .await
    .unwrap();
    assert_eq!(first.kind, ResponseKind::DiceRoll);

    let token = UseItemCommand {
        player_id: PlayerId::new(1),
        item_id: token_id,
    }
    .execute(&ctx)
    .await
    .unwrap();
    assert_eq!(token.kind, ResponseKind::Success);

    let second_roll = UseItemCommand {
        player_id: PlayerId::new(1),
        item_id: dice_id,
    }
    .execute(&ctx)
    .await
    .unwrap();
    assert_eq!(second_roll.kind, ResponseKind::DiceRoll);
}

#[tokio::test]
async fn test_use_item_player_not_found() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    let err = UseItemCommand {
        player_id: PlayerId::new(999),
        item_id: uuid::Uuid::new_v4(),
    }
    .execute(&ctx)
    .await
    .unwrap_err();

    assert!(matches!(err, MrRollerError::PlayerNotFound));
}

// ── InventoryCommand tests ──────────────────────────────────────────

#[tokio::test]
async fn test_inventory_empty() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    StartCommand {
        player_id: PlayerId::new(1),
    }
    .execute(&ctx)
    .await
    .unwrap();

    let resp = InventoryCommand {
        player_id: PlayerId::new(1),
    }
    .execute(&ctx)
    .await
    .unwrap();
    assert_eq!(resp.kind, ResponseKind::Inventory);
}

// ── LeaderboardCommand tests ────────────────────────────────────────

#[tokio::test]
async fn test_leaderboard_empty() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    let resp = LeaderboardCommand { limit: 10 }
        .execute(&ctx)
        .await
        .unwrap();
    assert_eq!(resp.kind, ResponseKind::Leaderboard);
}

// ── ShopCommand tests ───────────────────────────────────────────────

#[tokio::test]
async fn test_shop_lists_dice_without_reroll_token() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    let resp = ShopCommand.execute(&ctx).await.unwrap();
    assert_eq!(resp.kind, ResponseKind::Shop);
    let items = resp.data.unwrap().as_array().cloned().unwrap();
    assert!(items.iter().any(|item| item["key"] == "regular_dice"));
    assert!(items.iter().any(|item| item["key"] == "lucky_dice"));
    assert!(items.iter().any(|item| item["key"] == "cursed_dice"));
    assert!(!items.iter().any(|item| item["key"] == "reroll_token"));
}

#[tokio::test]
async fn test_buy_item_spends_coins_and_adds_item() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    let mut player = crate::game::player::Player::new(PlayerId::new(1));
    player.coins = 100;
    players.insert(player).await.unwrap();

    let resp = BuyItemCommand {
        player_id: PlayerId::new(1),
        item: ShopItemKind::LuckyDice,
    }
    .execute(&ctx)
    .await
    .unwrap();

    assert_eq!(resp.kind, ResponseKind::Success);
    assert_eq!(players.get(PlayerId::new(1)).await.unwrap().coins, 0);
    let items = inventory.list_items(PlayerId::new(1)).await.unwrap();
    assert_eq!(items.len(), 1);
    assert!(matches!(items[0].1, Item::LuckyDice(_)));
}

#[tokio::test]
async fn test_buy_item_requires_enough_coins() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    players
        .insert(crate::game::player::Player::new(PlayerId::new(1)))
        .await
        .unwrap();

    let resp = BuyItemCommand {
        player_id: PlayerId::new(1),
        item: ShopItemKind::LuckyDice,
    }
    .execute(&ctx)
    .await
    .unwrap();

    assert_eq!(resp.kind, ResponseKind::Error);
    assert!(inventory
        .list_items(PlayerId::new(1))
        .await
        .unwrap()
        .is_empty());
}

// ── EventCommand tests ──────────────────────────────────────────────

#[tokio::test]
async fn test_admin_spawn_random_item_event_and_claim() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let events = InMemoryEventStore::new();
    let cooldown = CooldownConfig::default();
    let event_config = crate::config::EventsConfig::default();
    let ctx = Context {
        players: &players,
        inventory: &inventory,
        leaderboard: &leaderboard,
        events: &events,
        cooldown: &cooldown,
        bootstrap_admin_ids: &[],
        event_config: &event_config,
    };

    let mut admin = crate::game::player::Player::new(PlayerId::new(1));
    admin.is_admin = true;
    players.insert(admin).await.unwrap();
    players
        .insert(crate::game::player::Player::new(PlayerId::new(2)))
        .await
        .unwrap();

    let spawned = SpawnRandomItemEventCommand {
        admin_id: PlayerId::new(1),
    }
    .execute(&ctx)
    .await
    .unwrap();
    assert_eq!(spawned.kind, ResponseKind::Event);
    let event_id = spawned.data.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    let claimed = ClaimEventCommand {
        player_id: PlayerId::new(2),
        event_id,
    }
    .execute(&ctx)
    .await
    .unwrap();

    assert_eq!(claimed.kind, ResponseKind::Event);
    assert_eq!(
        inventory.list_items(PlayerId::new(2)).await.unwrap().len(),
        1
    );
    assert!(matches!(
        events.get_event(event_id).await.unwrap().status,
        EventStatus::Claimed { .. }
    ));
}

#[tokio::test]
async fn test_trash_event_prevents_claim() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let events = InMemoryEventStore::new();
    let cooldown = CooldownConfig::default();
    let event_config = crate::config::EventsConfig::default();
    let ctx = Context {
        players: &players,
        inventory: &inventory,
        leaderboard: &leaderboard,
        events: &events,
        cooldown: &cooldown,
        bootstrap_admin_ids: &[],
        event_config: &event_config,
    };

    let mut admin = crate::game::player::Player::new(PlayerId::new(1));
    admin.is_admin = true;
    players.insert(admin).await.unwrap();
    players
        .insert(crate::game::player::Player::new(PlayerId::new(2)))
        .await
        .unwrap();

    let spawned = SpawnRandomItemEventCommand {
        admin_id: PlayerId::new(1),
    }
    .execute(&ctx)
    .await
    .unwrap();
    let event_id = spawned.data.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    let trashed = TrashEventCommand {
        player_id: PlayerId::new(2),
        event_id,
    }
    .execute(&ctx)
    .await
    .unwrap();
    assert_eq!(trashed.kind, ResponseKind::Event);

    let claim = ClaimEventCommand {
        player_id: PlayerId::new(2),
        event_id,
    }
    .execute(&ctx)
    .await
    .unwrap();
    assert_eq!(claim.kind, ResponseKind::Error);
    assert!(inventory
        .list_items(PlayerId::new(2))
        .await
        .unwrap()
        .is_empty());
}

// ── AdminCommand tests ──────────────────────────────────────────────

#[tokio::test]
async fn test_admin_give_item_requires_admin() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    players
        .insert(crate::game::player::Player::new(PlayerId::new(1)))
        .await
        .unwrap();
    players
        .insert(crate::game::player::Player::new(PlayerId::new(2)))
        .await
        .unwrap();

    let resp = AdminGiveItemCommand {
        admin_id: PlayerId::new(1),
        target_player_id: PlayerId::new(2),
        item: AdminItemKind::RerollToken,
    }
    .execute(&ctx)
    .await
    .unwrap();

    assert_eq!(resp.kind, ResponseKind::Error);
    assert!(inventory
        .list_items(PlayerId::new(2))
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn test_admin_give_item_adds_item_to_target() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    let mut admin = crate::game::player::Player::new(PlayerId::new(1));
    admin.is_admin = true;
    players.insert(admin).await.unwrap();
    players
        .insert(crate::game::player::Player::new(PlayerId::new(2)))
        .await
        .unwrap();

    let resp = AdminGiveItemCommand {
        admin_id: PlayerId::new(1),
        target_player_id: PlayerId::new(2),
        item: AdminItemKind::LuckyDice,
    }
    .execute(&ctx)
    .await
    .unwrap();

    assert_eq!(resp.kind, ResponseKind::Success);
    let items = inventory.list_items(PlayerId::new(2)).await.unwrap();
    assert_eq!(items.len(), 1);
    assert!(matches!(items[0].1, Item::LuckyDice(_)));
}

#[tokio::test]
async fn test_admin_give_item_creates_missing_target() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    let mut admin = crate::game::player::Player::new(PlayerId::new(1));
    admin.is_admin = true;
    players.insert(admin).await.unwrap();

    let resp = AdminGiveItemCommand {
        admin_id: PlayerId::new(1),
        target_player_id: PlayerId::new(99),
        item: AdminItemKind::RerollToken,
    }
    .execute(&ctx)
    .await
    .unwrap();

    assert_eq!(resp.kind, ResponseKind::Success);
    assert!(players.contains(PlayerId::new(99)).await.unwrap());
    assert_eq!(
        inventory.list_items(PlayerId::new(99)).await.unwrap().len(),
        1
    );
}

#[tokio::test]
async fn test_admin_adjust_coins_adds_and_removes_coins() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    let mut admin = crate::game::player::Player::new(PlayerId::new(1));
    admin.is_admin = true;
    players.insert(admin).await.unwrap();
    players
        .insert(crate::game::player::Player::new(PlayerId::new(2)))
        .await
        .unwrap();

    AdminAdjustCoinsCommand {
        admin_id: PlayerId::new(1),
        target_player_id: PlayerId::new(2),
        amount: 25,
    }
    .execute(&ctx)
    .await
    .unwrap();

    let resp = AdminAdjustCoinsCommand {
        admin_id: PlayerId::new(1),
        target_player_id: PlayerId::new(2),
        amount: -10,
    }
    .execute(&ctx)
    .await
    .unwrap();

    assert_eq!(resp.kind, ResponseKind::Success);
    assert_eq!(players.get(PlayerId::new(2)).await.unwrap().coins, 15);
}

#[tokio::test]
async fn test_admin_adjust_coins_cannot_remove_more_than_player_has() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    let mut admin = crate::game::player::Player::new(PlayerId::new(1));
    admin.is_admin = true;
    players.insert(admin).await.unwrap();
    players
        .insert(crate::game::player::Player::new(PlayerId::new(2)))
        .await
        .unwrap();

    let resp = AdminAdjustCoinsCommand {
        admin_id: PlayerId::new(1),
        target_player_id: PlayerId::new(2),
        amount: -1,
    }
    .execute(&ctx)
    .await
    .unwrap();

    assert_eq!(resp.kind, ResponseKind::Error);
    assert_eq!(players.get(PlayerId::new(2)).await.unwrap().coins, 0);
}

#[tokio::test]
async fn test_admin_set_admin_updates_target() {
    let players = InMemoryPlayerStore::new();
    let inventory = InMemoryInventoryStore::new();
    let leaderboard = InMemoryLeaderboardStore::new();
    let cooldown = CooldownConfig::default();
    let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

    let mut admin = crate::game::player::Player::new(PlayerId::new(1));
    admin.is_admin = true;
    players.insert(admin).await.unwrap();
    players
        .insert(crate::game::player::Player::new(PlayerId::new(2)))
        .await
        .unwrap();

    let resp = AdminSetAdminCommand {
        admin_id: PlayerId::new(1),
        target_player_id: PlayerId::new(2),
        is_admin: true,
    }
    .execute(&ctx)
    .await
    .unwrap();

    assert_eq!(resp.kind, ResponseKind::Success);
    assert!(players.get(PlayerId::new(2)).await.unwrap().is_admin);
}
