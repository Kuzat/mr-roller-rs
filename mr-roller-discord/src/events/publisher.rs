use mr_roller::response::Response;
use serenity::all::{
    ChannelId, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, Http,
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

pub async fn update_event_interaction_message(
    http: &Http,
    component: &serenity::all::ComponentInteraction,
    response: &Response,
) -> Result<(), Error> {
    component
        .create_response(
            http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embeds::event_embed(response))
                    .components(Vec::new()),
            ),
        )
        .await?;
    Ok(())
}
