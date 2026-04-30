pub mod admin;
pub mod player;
pub mod setup;

use crate::{Data, Error};

pub fn commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        player::ping(),
        setup::setup(),
        setup::status(),
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
