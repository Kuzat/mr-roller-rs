use std::fmt::Debug;

use crate::response::Response;

pub mod dice;
pub mod tokens;

// ── GameItem trait — contract every item struct must fulfil ────────────────

pub trait GameItem: Debug + Clone + Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn handle(&self) -> Response;

    /// Whether using this item consumes the player's daily roll allowance.
    /// Dice consume the daily roll by default. Utility items like reroll tokens
    /// can override this to bypass/reset cooldown behaviour.
    fn consumes_daily_roll(&self) -> bool {
        true
    }
}

// ── define_items! macro — one line per item type ───────────────────────────

/// Define the `Item` enum, `From` conversions, accessor helpers, and
/// delegate methods (`name`, `description`, `handle`) — all generated from
/// a single declarative block.
///
/// Usage:
/// ```ignore
/// define_items! {
///     BasicDice(basic_dice::BasicDice) as basic_dice,
///     RerollToken(tokens::reroll_token::RerollToken) as reroll_token,
/// }
/// ```
#[macro_export]
macro_rules! define_items {
    (
        $(
            $variant:ident($ty:ty) as $snake:ident
        ),+ $(,)?
    ) => {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub enum Item {
            $($variant($ty)),+
        }

        $(
            impl From<$ty> for Item {
                fn from(v: $ty) -> Self {
                    Item::$variant(v)
                }
            }
        )+

        impl Item {
            $(
                /// Returns `Some` if this item is a `$variant`, `None` otherwise.
                pub fn $snake(self) -> Option<$ty> {
                    match self {
                        Item::$variant(v) => Some(v),
                        _ => None,
                    }
                }
            )+

            /// Returns the human-readable name of the item.
            pub fn name(&self) -> &str {
                match self {
                    $(Item::$variant(item) => $crate::game::item::GameItem::name(item)),+
                }
            }

            /// Returns a short description of the item.
            pub fn description(&self) -> &str {
                match self {
                    $(Item::$variant(item) => $crate::game::item::GameItem::description(item)),+
                }
            }

            /// Activates the item and returns the result.
            pub fn handle(&self) -> Response {
                match self {
                    $(Item::$variant(item) => $crate::game::item::GameItem::handle(item)),+
                }
            }

            /// Returns whether this item consumes the daily roll allowance.
            pub fn consumes_daily_roll(&self) -> bool {
                match self {
                    $(Item::$variant(item) => $crate::game::item::GameItem::consumes_daily_roll(item)),+
                }
            }
        }
    };
}

// ── Item registry — add new items here ─────────────────────────────────────

define_items! {
    BasicDice(dice::basic_dice::BasicDice) as basic_dice,
    LuckyDice(dice::lucky_dice::LuckyDice) as lucky_dice,
    CursedDice(dice::cursed_dice::CursedDice) as cursed_dice,
    RerollToken(tokens::reroll_token::RerollToken) as reroll_token,
}
