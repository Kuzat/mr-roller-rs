use serenity::all::{ButtonStyle, CreateActionRow, CreateButton};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventButtonAction {
    Claim,
    Trash,
}

pub fn event_custom_id(action: EventButtonAction, event_id: &str) -> String {
    let action = match action {
        EventButtonAction::Claim => "claim",
        EventButtonAction::Trash => "trash",
    };
    format!("event:{action}:{event_id}")
}

pub fn parse_event_custom_id(custom_id: &str) -> Option<(EventButtonAction, Uuid)> {
    let mut parts = custom_id.splitn(3, ':');
    if parts.next()? != "event" {
        return None;
    }
    let action = match parts.next()? {
        "claim" => EventButtonAction::Claim,
        "trash" => EventButtonAction::Trash,
        _ => return None,
    };
    let event_id = parts.next()?.parse().ok()?;
    Some((action, event_id))
}

pub fn event_buttons(event_id: &str) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(event_custom_id(EventButtonAction::Claim, event_id))
            .label("Claim")
            .style(ButtonStyle::Success),
        CreateButton::new(event_custom_id(EventButtonAction::Trash, event_id))
            .label("Trash")
            .style(ButtonStyle::Danger),
    ])]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_event_custom_ids() {
        let id = Uuid::new_v4();
        assert_eq!(
            parse_event_custom_id(&format!("event:claim:{id}")),
            Some((EventButtonAction::Claim, id))
        );
        assert_eq!(parse_event_custom_id("event:boop:nope"), None);
    }
}
