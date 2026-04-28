use std::{io, sync::Arc};

use mr_roller::{
    command::{InventoryCommand, LeaderboardCommand, StartCommand, UseItemCommand},
    game::{player::PlayerId, Game},
    response::ResponseKind,
    store::{
        InMemoryInventoryStore, InMemoryLeaderboardStore, InMemoryPlayerStore,
    },
};

#[tokio::main]
async fn main() {
    let player_id = parse_player_id();
    let game = build_game();

    println!("🎲 Mr Roller CLI");
    println!("  /start       — join the game");
    println!("  /use <id>    — use an item from inventory");
    println!("  /inventory   — list your items");
    println!("  /leaderboard — show top scores");
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

fn build_game() -> Game {
    Game::new(
        Arc::new(InMemoryPlayerStore::new()),
        Arc::new(InMemoryInventoryStore::new()),
        Arc::new(InMemoryLeaderboardStore::new()),
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
    Leaderboard,
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
        Some("/leaderboard") | Some("/lb") => Ok(ParsedCommand::Leaderboard),
        Some(cmd) => Err(format!("Unknown command: {}. Try /start, /use, /inventory, /leaderboard.", cmd)),
        None => Err("Empty command.".into()),
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
            ParsedCommand::Leaderboard => game.execute(LeaderboardCommand { limit: 10 }).await,
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
            ResponseKind::Leaderboard => {
                if let Some(entries) = data.as_array() {
                    for (i, entry) in entries.iter().enumerate() {
                        let pid = entry["player_id"].as_u64().unwrap_or(0);
                        let xp = entry["xp"].as_u64().unwrap_or(0);
                        let coins = entry["coins"].as_u64().unwrap_or(0);
                        println!(
                            "    {}. Player {} — {} XP, {} coins",
                            i + 1,
                            pid,
                            xp,
                            coins
                        );
                    }
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
