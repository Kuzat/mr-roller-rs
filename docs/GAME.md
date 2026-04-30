# Game Document

This is a document where I try to explain how I envison the first playable iteration of the game and all the features and how I expect it to be interacted with.
I will also try to explain what assets we might want to have in this game. 

## Short Introduction

This game Mr Roller, is a async dice game, but working more like a RPG than jsut a normal dice game. The core mecahnic of the game
is to roll dice. However, it is a game you play against other players, and also you will want to progress like a incremental game or RPG where you have
some stats that you want to improve. The other part of the core mechanic is the limited amount of rolls you can do daily and the cooldown between these. 
This is to give some value to limiting how many actions can be done and also to make it more forgiving in the async nature of this game living in a chat channel
and players might play at different times. 

The core items are dice, but there are many types of dice. Some are simple dice, some give better rolls, some let you sabotage other player, some let you leech from other, 
or let you gamble to get a better reward. There is two main currencies in this game. The first being XP this will be used to lvl up and progress your stats in a RPG sense. 
You will have to make choice when leveling up which skills you want to improve and in this way it might affect which dice and which play style suite you. The other currency is 
the coin, this primarily used to buy and sell items in the shop. But, there will be other use for it as well, and players can use it if they want to do trading of items. 

The game is meant to be played async in a chat, which for now only is created for discord. Where you use slash command and other embeds or features in discord to display things happening. 

## Skills

The players will have a few skills that affect rolls and other features. These skills can be improved by leveling up or by equipping some items.

Skills should use a 0 to 100 scale. Each point in a skill should give a small passive bonus, while larger threshold bonuses should unlock at regular intervals, such as every 10 points. This should make every point feel useful while still giving players clear milestone goals to work toward.

The goal is that each skill supports a different playstyle, so the player has meaningful choices when leveling up instead of every skill simply increasing rewards in the same way.

### Luck

Passively affects how you roll all dice. Having higher luck will mean you are more likely to roll a favorable value for a dice. What counts as favorable might be different based on the dice and its effect.

For example, luck might make a normal reward dice more likely to roll high, make a gamble dice less risky, or improve the chance of getting a better result from a special dice.

### Endurance

Endurance affects how often a player can roll.

Each point in Endurance should slightly reduce the cooldown between rolls. At larger thresholds, such as every 10 points, Endurance should increase how many rolls the player can do each day.

This skill supports players who want to be more active and progress through consistent daily play. The bonuses should be kept modest so that Endurance does not become mandatory compared to the other skills.

### Greed

Greed affects coin gain and the player's interaction with the economy.

Each point in Greed should slightly improve coin-related rewards. This could include earning more coins from dice, getting better value from selling items, receiving small shop discounts, or improving the reward side of gambling-style dice.

This skill supports players who want to focus on buying, selling, trading, and building wealth. Greed should mostly affect coins rather than XP, so it creates a real dilemma against Wisdom instead of becoming a direct leveling skill.

### Wisdom

Wisdom affects XP gain and long-term progression.

Each point in Wisdom should slightly improve XP-related rewards. This could include earning more XP from rolls, getting better XP from streaks or achievements, or improving progression-focused rewards.

This skill supports players who want to level faster and unlock long-term power. Wisdom should be balanced carefully because XP is tied directly to character progression. It should be useful for growth without making it the only correct skill choice.

## Leveling

Players gain XP through rolling dice, achievements, and other progression rewards. When a player gains enough XP, they level up and can improve their skills.

There should be no maximum player level. Instead, the XP required for each level should scale exponentially and become significantly harder over time. This allows the game to support long-term progression, prevents players from maxing out too early, and gives room to add higher-level features later.

The progression curve should be designed intentionally around the expected pace of daily rolls, cooldowns, achievements, and future systems. Early levels should come quickly enough that players understand the leveling loop, while later levels should require longer-term commitment.

### Initial XP Curve

The initial XP curve should use this formula for the XP required to reach the next level:

```text
XP to next level = floor(100 * current_level^1.6)
```

For example, a level 1 player needs `100` XP to reach level 2.

This gives a curve that starts approachable, slows down meaningfully in the mid-game, and keeps scaling upward without requiring a maximum level. The exponent can be tuned later based on the real XP income players get from daily rolls and achievements.

The main tuning knobs are:

- `1.4`: faster and more casual progression.
- `1.6`: balanced long-term progression.
- `1.8`: slower and grindier progression.
- `2.0`: very hard scaling.

The starting recommendation is `1.6`.

### Initial Level Table

| Current Level | XP to Next Level | Total XP Required |
|---:|---:|---:|
| 1 | 100 | 0 |
| 2 | 303 | 100 |
| 3 | 579 | 403 |
| 4 | 918 | 982 |
| 5 | 1,313 | 1,900 |
| 6 | 1,756 | 3,213 |
| 7 | 2,247 | 4,969 |
| 8 | 2,786 | 7,216 |
| 9 | 3,367 | 10,002 |
| 10 | 3,981 | 13,369 |
| 11 | 4,641 | 17,350 |
| 12 | 5,333 | 21,991 |
| 13 | 6,067 | 27,324 |
| 14 | 6,835 | 33,391 |
| 15 | 7,637 | 40,226 |
| 16 | 8,445 | 47,863 |
| 17 | 9,305 | 56,308 |
| 18 | 10,193 | 65,613 |
| 19 | 11,110 | 75,806 |
| 20 | 12,065 | 86,916 |
| 21 | 13,044 | 98,981 |
| 22 | 14,064 | 112,025 |
| 23 | 15,113 | 126,089 |
| 24 | 16,185 | 141,202 |
| 25 | 17,302 | 157,387 |
| 26 | 18,424 | 174,689 |
| 27 | 19,585 | 193,113 |
| 28 | 20,773 | 212,698 |
| 29 | 21,994 | 233,471 |
| 30 | 23,238 | 255,465 |

`Total XP Required` means the amount of lifetime XP needed to already be at that level.

### Skill Points

Players should gain `1` skill point each time they level up. These points can be spent on improving skills.

Because skills have a 0 to 100 scale and there are multiple skills, this creates long-term specialization. A player can focus heavily on one skill to reach threshold bonuses faster, or spread points across multiple skills for a more balanced build.

With 4 initial skills, fully maxing every skill would require a very high player level, especially because player levels have no maximum and XP scales upward. Items and future systems can also provide temporary or permanent skill bonuses.

## Items

### Dices

### Tokens

### Equipable Items


## Events

## Magic (Secret machanic)

Magic is a secret mechanic that is hidden to the palyer and can be unlocked by certain conditions. It will allow players to combine, destroy or sacrifice items, coins or XP for 
new and interesting effects. These effects is invisible to other players and only if the player them selves decides to reveal it will other players know. 

## Achivements

These are goals that you can reach to get small extra reward. There will be a few public achivements that is visible to all, like roll 7 days in a row, roll your first 6 with a regular dice. But, then there is also hidden achivements that is not visible to the player, like own two starter dice (how did you manage to get this?), Own 1 of every regular dice.
These achivements are shared with everyone once someone achives them, even the hidden ones. So in this way it is separate from the magic feature.

## Leaderboard

The game will have a leaderboard based on different stats. One will be LVL who has the highest XP, and one will be the highest amount of total dice rolls. 
This is to drive competition between players and want thme to do better than other players. 

