use std::{io, sync::Arc};

use mr_roller::{
    command::{
        AdminAdjustCoinsCommand, AdminGiveItemCommand, AdminHelpCommand, AdminItemKind,
        AdminSetAdminCommand, BuyItemCommand, ClaimEventCommand, InventoryCommand,
        LeaderboardCommand, ListActiveEventsCommand, ShopCommand, ShopItemKind,
        SpawnRandomItemEventCommand, StartCommand, TrashEventCommand, UseItemCommand,
    },
    config::{EventsConfig, Settings},
    event_scheduler::EventScheduler,
    game::{player::PlayerId, Game},
    response::ResponseKind,
    store::{InMemoryInventoryStore, InMemoryLeaderboardStore, InMemoryPlayerStore, SqliteStore},
};

#[tokio::main]
async fn main() {
    let player_id = parse_player_id();
    let settings = Settings::load().expect("failed to load Mr Roller configuration");
    let game = Arc::new(build_game(&settings).await);
    start_event_scheduler(game.clone(), settings.events.clone());

    println!("🎲 Mr Roller CLI");
    println!("  /start       — join the game");
    println!("  /use <id>    — use an item from inventory");
    println!("  /inventory   — list your items");
    println!("  /shop        — list buyable items");
    println!("  /buy <item>  — buy an item from the shop");
    println!("  /leaderboard — show top scores");
    println!("  /events      — list active events");
    println!("  /event claim <id> | /event trash <id>");
    println!("  /admin       — show admin commands if you are an admin");
    println!("  /quit        — exit");
    println!();

    loop {
        let input = read_line("> ");
        let trimmed = input.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed == "/quit" || trimmed == "/q" {
            println!("Goodbye!");
            break;
        }

        let response = match parse_command(trimmed, player_id) {
            Ok(cmd) => cmd.execute(&game).await,
            Err(msg) => {
                println!("  ❌ {}", msg);
                continue;
            }
        };

        print_response(&response);
    }
}

// ── Setup ──────────────────────────────────────────────────────────────────

fn parse_player_id() -> PlayerId {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 {
        if let Ok(id) = args[1].parse::<u64>() {
            return PlayerId::new(id);
        }
    }
    // Default player ID
    PlayerId::new(1)
}

fn start_event_scheduler(game: Arc<Game>, events_config: EventsConfig) {
    if !events_config.enabled {
        return;
    }

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("failed to start event scheduler runtime");
        let scheduler = EventScheduler::from_seconds(events_config.check_interval_seconds);

        runtime.block_on(async move {
            scheduler
                .run(game, |response| async move {
                    println!();
                    print_response(&response);
                    use std::io::Write;
                    print!("> ");
                    io::stdout().flush().ok();
                })
                .await;
        });
    });
}

async fn build_game(settings: &Settings) -> Game {
    let bootstrap_admin_ids = settings.bootstrap_admin_player_ids();

    if let Some(database_url) = settings
        .database
        .url
        .as_deref()
        .filter(|url| !url.is_empty())
    {
        match SqliteStore::connect(database_url).await {
            Ok(store) => {
                println!("Using SQLite store: {}", database_url);
                let store = Arc::new(store);
                return Game::with_event_store(
                    store.clone(),
                    store.clone(),
                    store.clone(),
                    store,
                    bootstrap_admin_ids,
                    settings.events.clone(),
                );
            }
            Err(err) => {
                eprintln!("Failed to open SQLite store: {}", err);
                eprintln!("Falling back to in-memory store.");
            }
        }
    }

    Game::with_event_store(
        Arc::new(InMemoryPlayerStore::new()),
        Arc::new(InMemoryInventoryStore::new()),
        Arc::new(InMemoryLeaderboardStore::new()),
        Arc::new(mr_roller::store::InMemoryEventStore::new()),
        bootstrap_admin_ids,
        settings.events.clone(),
    )
}

// ── Input ──────────────────────────────────────────────────────────────────

fn read_line(prompt: &str) -> String {
    use std::io::Write;
    print!("{}", prompt);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap_or_default();
    input
}

// ── Command parsing ────────────────────────────────────────────────────────

enum ParsedCommand {
    Start(PlayerId),
    UseItem(PlayerId, uuid::Uuid),
    Inventory(PlayerId),
    Shop,
    BuyItem(PlayerId, ShopItemKind),
    Leaderboard,
    Events,
    ClaimEvent(PlayerId, uuid::Uuid),
    TrashEvent(PlayerId, uuid::Uuid),
    AdminHelp(PlayerId),
    AdminGiveItem(PlayerId, PlayerId, AdminItemKind),
    AdminAdjustCoins(PlayerId, PlayerId, i64),
    AdminSpawnRandomItemEvent(PlayerId),
    AdminSetAdmin(PlayerId, PlayerId, bool),
}

fn parse_command(input: &str, pid: PlayerId) -> Result<ParsedCommand, String> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    match parts.get(0).copied() {
        Some("/start") => Ok(ParsedCommand::Start(pid)),
        Some("/use") => {
            let raw_id = parts.get(1).ok_or("Usage: /use <item-id>")?;
            let item_id = uuid::Uuid::parse_str(raw_id).map_err(|_| "Invalid item ID format.")?;
            Ok(ParsedCommand::UseItem(pid, item_id))
        }
        Some("/inventory") | Some("/inv") => Ok(ParsedCommand::Inventory(pid)),
        Some("/shop") => Ok(ParsedCommand::Shop),
        Some("/buy") => {
            let item = parts
                .get(1)
                .ok_or("Usage: /buy <item>")?
                .parse::<ShopItemKind>()?;
            Ok(ParsedCommand::BuyItem(pid, item))
        }
        Some("/leaderboard") | Some("/lb") => Ok(ParsedCommand::Leaderboard),
        Some("/events") => Ok(ParsedCommand::Events),
        Some("/event") => parse_event_command(&parts, pid),
        Some("/admin") => parse_admin_command(&parts, pid),
        Some(cmd) => Err(format!(
            "Unknown command: {}. Try /start, /use, /inventory, /shop, /buy, /leaderboard, /events, /admin.",
            cmd
        )),
        None => Err("Empty command.".into()),
    }
}

fn parse_event_command(parts: &[&str], pid: PlayerId) -> Result<ParsedCommand, String> {
    let raw_id = parts
        .get(2)
        .ok_or("Usage: /event claim <id> or /event trash <id>")?;
    let event_id = uuid::Uuid::parse_str(raw_id).map_err(|_| "Invalid event ID format.")?;
    match parts.get(1).copied() {
        Some("claim") => Ok(ParsedCommand::ClaimEvent(pid, event_id)),
        Some("trash") => Ok(ParsedCommand::TrashEvent(pid, event_id)),
        _ => Err("Usage: /event claim <id> or /event trash <id>".to_string()),
    }
}

fn parse_admin_command(parts: &[&str], pid: PlayerId) -> Result<ParsedCommand, String> {
    match parts.get(1).copied() {
        None | Some("help") => Ok(ParsedCommand::AdminHelp(pid)),
        Some("give") => {
            let target =
                parse_player_id_arg(parts.get(2), "Usage: /admin give <player-id> <item>")?;
            let item = parts
                .get(3)
                .ok_or("Usage: /admin give <player-id> <item>")?
                .parse::<AdminItemKind>()?;
            Ok(ParsedCommand::AdminGiveItem(pid, target, item))
        }
        Some("coins") => {
            let target = parse_player_id_arg(
                parts.get(2),
                "Usage: /admin coins <player-id> <amount>",
            )?;
            let amount = parts
                .get(3)
                .ok_or("Usage: /admin coins <player-id> <amount>")?
                .parse::<i64>()
                .map_err(|_| "Invalid coin amount.".to_string())?;
            Ok(ParsedCommand::AdminAdjustCoins(pid, target, amount))
        }
        Some("event") => match parts.get(2).copied() {
            Some("spawn-random-item") | Some("spawn") => {
                Ok(ParsedCommand::AdminSpawnRandomItemEvent(pid))
            }
            _ => Err("Usage: /admin event spawn-random-item".to_string()),
        },
        Some("set-admin") | Some("admin") => {
            let target = parse_player_id_arg(
                parts.get(2),
                "Usage: /admin set-admin <player-id> <true|false>",
            )?;
            let is_admin = parse_bool_arg(
                parts.get(3),
                "Usage: /admin set-admin <player-id> <true|false>",
            )?;
            Ok(ParsedCommand::AdminSetAdmin(pid, target, is_admin))
        }
        Some(cmd) => Err(format!(
            "Unknown admin command: {}. Try /admin, /admin give, /admin coins, /admin event, or /admin set-admin.",
            cmd
        )),
    }
}

fn parse_player_id_arg(value: Option<&&str>, usage: &str) -> Result<PlayerId, String> {
    value
        .ok_or(usage.to_string())?
        .parse::<u64>()
        .map(PlayerId::new)
        .map_err(|_| "Invalid player ID.".to_string())
}

fn parse_bool_arg(value: Option<&&str>, usage: &str) -> Result<bool, String> {
    match value
        .ok_or(usage.to_string())?
        .to_ascii_lowercase()
        .as_str()
    {
        "true" | "yes" | "1" | "on" => Ok(true),
        "false" | "no" | "0" | "off" => Ok(false),
        _ => Err("Expected true or false.".to_string()),
    }
}

// ── Dispatch ───────────────────────────────────────────────────────────────

// We erase the Command type by boxing. A real app would use an enum, but this
// is simpler for a small CLI. The pattern preserves the Command trait design.
impl ParsedCommand {
    async fn execute(self, game: &Game) -> mr_roller::response::Response {
        match self {
            ParsedCommand::Start(pid) => game.execute(StartCommand { player_id: pid }).await,
            ParsedCommand::UseItem(pid, item_id) => {
                game.execute(UseItemCommand {
                    player_id: pid,
                    item_id,
                })
                .await
            }
            ParsedCommand::Inventory(pid) => {
                game.execute(InventoryCommand { player_id: pid }).await
            }
            ParsedCommand::Shop => game.execute(ShopCommand).await,
            ParsedCommand::BuyItem(pid, item) => {
                game.execute(BuyItemCommand {
                    player_id: pid,
                    item,
                })
                .await
            }
            ParsedCommand::Leaderboard => game.execute(LeaderboardCommand { limit: 10 }).await,
            ParsedCommand::Events => game.execute(ListActiveEventsCommand).await,
            ParsedCommand::ClaimEvent(pid, event_id) => {
                game.execute(ClaimEventCommand {
                    player_id: pid,
                    event_id,
                })
                .await
            }
            ParsedCommand::TrashEvent(pid, event_id) => {
                game.execute(TrashEventCommand {
                    player_id: pid,
                    event_id,
                })
                .await
            }
            ParsedCommand::AdminHelp(pid) => {
                game.execute(AdminHelpCommand { player_id: pid }).await
            }
            ParsedCommand::AdminGiveItem(admin_id, target_player_id, item) => {
                game.execute(AdminGiveItemCommand {
                    admin_id,
                    target_player_id,
                    item,
                })
                .await
            }
            ParsedCommand::AdminAdjustCoins(admin_id, target_player_id, amount) => {
                game.execute(AdminAdjustCoinsCommand {
                    admin_id,
                    target_player_id,
                    amount,
                })
                .await
            }
            ParsedCommand::AdminSpawnRandomItemEvent(admin_id) => {
                game.execute(SpawnRandomItemEventCommand { admin_id }).await
            }
            ParsedCommand::AdminSetAdmin(admin_id, target_player_id, is_admin) => {
                game.execute(AdminSetAdminCommand {
                    admin_id,
                    target_player_id,
                    is_admin,
                })
                .await
            }
        }
    }
}

// ── Output rendering ───────────────────────────────────────────────────────

fn print_response(response: &mr_roller::response::Response) {
    let icon = match response.kind {
        ResponseKind::Success => "✅",
        ResponseKind::Error => "❌",
        ResponseKind::DiceRoll => "🎲",
        ResponseKind::Inventory => "🎒",
        ResponseKind::Leaderboard => "🏆",
        ResponseKind::Shop => "🛒",
        ResponseKind::Event => "🎉",
    };

    println!("  {} {}", icon, response.message);

    // Print structured data if present
    if let Some(data) = &response.data {
        match response.kind {
            ResponseKind::Inventory => {
                if let Some(items) = data.as_array() {
                    for item in items {
                        let id = item["id"].as_str().unwrap_or("?");
                        let name = item["name"].as_str().unwrap_or("?");
                        let desc = item["description"].as_str().unwrap_or("");
                        println!("    [{}] {} — {}", id, name, desc);
                    }
                }
            }
            ResponseKind::Shop => {
                if let Some(items) = data.as_array() {
                    for item in items {
                        let key = item["key"].as_str().unwrap_or("?");
                        let name = item["name"].as_str().unwrap_or("?");
                        let desc = item["description"].as_str().unwrap_or("");
                        let price = item["price"].as_u64().unwrap_or(0);
                        println!("    {} — {} coins — {} — {}", key, price, name, desc);
                    }
                }
            }
            ResponseKind::Leaderboard => {
                if let Some(entries) = data.as_array() {
                    for (i, entry) in entries.iter().enumerate() {
                        let pid = entry["player_id"].as_u64().unwrap_or(0);
                        let xp = entry["xp"].as_u64().unwrap_or(0);
                        let coins = entry["coins"].as_u64().unwrap_or(0);
                        println!("    {}. Player {} — {} XP, {} coins", i + 1, pid, xp, coins);
                    }
                }
            }
            ResponseKind::Event => {
                if let Some(events) = data.as_array() {
                    for event in events {
                        print_event(event);
                    }
                } else if data.is_object() {
                    print_event(data);
                }
            }
            ResponseKind::DiceRoll => {
                if let Some(roll) = data.get("roll").and_then(|v| v.as_u64()) {
                    println!("    Roll result: {}", roll);
                }
            }
            _ => {}
        }
    }
}

fn print_event(event: &serde_json::Value) {
    let id = event["id"].as_str().unwrap_or("?");
    let title = event["title"].as_str().unwrap_or("?");
    let desc = event["description"].as_str().unwrap_or("");
    let status = event["status"].as_str().unwrap_or("?");
    let expires_at = event["expires_at"].as_str().unwrap_or("?");
    println!("    [{}] {} — {}", id, title, status);
    println!("        {}", desc);
    println!("        expires at {}", expires_at);
}
