pub mod admin;
pub mod player;

use crate::{Data, Error};

pub fn commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        player::ping(),
        player::start(),
        player::inventory(),
        player::shop(),
        player::buy(),
        player::leaderboard(),
        player::use_item(),
        player::events(),
        player::event(),
        admin::admin(),
    ]
}
