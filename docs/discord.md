# Running the Discord multi-tenant bot

The Discord runtime is a public/installable, multi-tenant bot. Each Discord guild + channel runs an isolated game after a server manager runs `/setup channel:#dice-game`.

## Prerequisites

- Rust toolchain
- Docker / Docker Compose
- A Discord application with a bot token

## 1. Start PostgreSQL

The Discord runtime requires PostgreSQL. The local database settings are defined in `compose.yaml`:

```bash
docker compose up -d postgres
```

This exposes PostgreSQL at:

```text
postgres://mr_roller:secret@localhost:5432/mr_roller
```

## 2. Configure the bot

Use the Discord-specific config file:

```text
mr-roller-discord.toml
```

It already contains the PostgreSQL URL from `compose.yaml`.

Set your Discord bot token with an environment variable:

```bash
export MR_ROLLER__DISCORD__TOKEN='your-bot-token'
```

The multi-tenant Discord bot always registers slash commands globally so the same bot install works across many servers.

To bootstrap in-game admins, add Discord user IDs to:

```toml
[admin]
bootstrap_admin_ids = [123456789012345678]
```

## 3. Run the Discord bot

```bash
MR_ROLLER_CONFIG='./mr-roller-discord.toml' \
MR_ROLLER__DISCORD__TOKEN='your-bot-token' \
cargo run -p mr-roller-discord
```

On startup, database migrations are applied automatically.

## 4. Invite and set up a game channel

Create an OAuth2 install URL in the Discord Developer Portal with these scopes:

- `bot`
- `applications.commands`

Recommended bot permissions:

- View Channels
- Send Messages
- Embed Links
- Read Message History
- Use Slash Commands

After inviting the bot, a server manager should run:

```text
/setup channel:#dice-game
```

Players can then use game commands in that channel. Running `/setup` in another channel creates a separate isolated game for the same server.

## Useful commands

```text
/ping
/setup channel:<text-channel>
/status
/start
/inventory
/shop
/buy item:<shop key>
/leaderboard
/use item:<inventory item>
/events
/event claim event:<event id>
/event trash event:<event id>
/admin give user:<Discord user> item:<item>
/admin coins user:<Discord user> amount:<delta>
/admin set-admin user:<Discord user> is-admin:<bool>
/admin event spawn-random-item
```
