use mr_roller::response::Response;
use serenity::{
    all::{ChannelId, CreateMessage, Http},
    builder::EditMessage,
};

use crate::render::{components, embeds};
use crate::Error;

pub async fn publish_event_response(
    http: &Http,
    channel_id: ChannelId,
    response: &Response,
) -> Result<(), Error> {
    let mut message = CreateMessage::new().embed(embeds::event_embed(response));
    if let Some(event_id) = embeds::event_id(response) {
        message = message.components(components::event_buttons(event_id));
    }
    channel_id.send_message(http, message).await?;
    Ok(())
}

pub async fn edit_event_message_final(
    http: &Http,
    component: &serenity::all::ComponentInteraction,
    response: &Response,
) -> Result<(), Error> {
    let message = EditMessage::new()
        .embed(embeds::event_embed(response))
        .components(Vec::new());
    component.message.clone().edit(http, message).await?;
    Ok(())
}
