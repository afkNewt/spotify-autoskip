use std::collections::HashMap;

use futures_util::StreamExt;
use zbus::{Message, zvariant::OwnedValue};

use crate::spotify::{Spotify, SpotifySignals, get_signals};

mod spotify;

enum Event {
    Property(Message),
    Owner(OwnerChange),
    StreamDropped,
}

enum OwnerChange {
    Gained,
    Lost,
}

#[derive(Clone, Copy)]
enum OwnerLossPolicy {
    ExitOnLoss,
    IgnoreNextLoss,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let SpotifySignals {
        connection,
        mut properties_changed,
        mut owner_changed,
    } = get_signals().await?;

    let mut owner_loss = OwnerLossPolicy::ExitOnLoss;

    let mut spotify = Spotify::new(&connection).await?;
    loop {
        let event = tokio::select! {
            message = properties_changed.next() => {
                match message {
                    Some(property) => Event::Property(property),
                    None => Event::StreamDropped,
                }
            }
            message = owner_changed.next() => {
                match message {
                    Some(owner) => Event::Owner(if owner.is_some() { OwnerChange::Gained } else { OwnerChange::Lost }),
                    None => Event::StreamDropped,
                }
            }
        };

        match event {
            Event::Property(message) => {
                let Some(track_id) = get_track_id(&message)? else {
                    continue;
                };

                if is_ad(&track_id) {
                    spotify.restart_and_skip(&connection).await?;
                    owner_loss = OwnerLossPolicy::IgnoreNextLoss;
                }
            }
            Event::Owner(owner) => match (owner, owner_loss) {
                (OwnerChange::Gained, _) => {
                    // gained a new owner
                }
                (OwnerChange::Lost, OwnerLossPolicy::IgnoreNextLoss) => {
                    owner_loss = OwnerLossPolicy::ExitOnLoss;
                }
                (OwnerChange::Lost, OwnerLossPolicy::ExitOnLoss) => return Ok(()),
            },
            Event::StreamDropped => return Ok(()),
        }
    }
}

fn get_track_id(message: &Message) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let (_interface, changed, _invalidated): (String, HashMap<String, OwnedValue>, Vec<String>) =
        message.body().deserialize()?;

    let Some(metadata_value) = changed.get("Metadata") else {
        return Ok(None);
    };

    let metadata: HashMap<String, OwnedValue> = metadata_value.clone().try_into()?;
    let Some(track_id_value) = metadata.get("mpris:trackid") else {
        return Ok(None);
    };

    let track_id: String = track_id_value.clone().try_into()?;
    Ok(Some(track_id))
}

fn is_ad(track_id: &str) -> bool {
    track_id.split('/').any(|s| s == "ad")
}
