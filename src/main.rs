use std::{
    collections::HashMap,
    process::{Child, Command},
    time::Duration,
};

use futures_util::StreamExt;
use zbus::{Connection, Message, Proxy, zvariant::OwnedValue};

// busctl --user introspect org.mpris.MediaPlayer2.spotify /org/mpris/MediaPlayer2
const DESTINATION: &str = "org.mpris.MediaPlayer2.spotify";
const PATH: &str = "/org/mpris/MediaPlayer2";

enum Method {
    Ping,
    Play,
    Next,
}

impl Method {
    pub fn interface(&self) -> &str {
        match self {
            Method::Ping => "org.freedesktop.DBus.Peer",
            Method::Play => "org.mpris.MediaPlayer2.Player",
            Method::Next => "org.mpris.MediaPlayer2.Player",
        }
    }
    pub fn name(&self) -> &str {
        match self {
            Method::Ping => "Ping",
            Method::Play => "Play",
            Method::Next => "Next",
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection = Connection::session().await?;

    let proxy = Proxy::new(
        &connection,
        DESTINATION,
        PATH,
        "org.freedesktop.DBus.Properties",
    )
    .await?;

    let mut signal = proxy.receive_signal("PropertiesChanged").await?;
    let mut owner = proxy.receive_owner_changed().await?;

    let mut restarted = false;

    let mut child = spawn_spotify()?;
    wait_for_spotify_ready(&connection).await;
    call_method(&connection, Method::Play).await?;

    loop {
        tokio::select! {
            message = signal.next() => {
                let Some(message) = message else {
                    // stream dropped
                    return Ok(());
                };

                let Some(track_id) = track_id(&message) else {
                    continue;
                };

                if is_ad(&track_id) == Some(true) {
                    restart_spotify(&mut child)?;
                    wait_for_spotify_ready(&connection).await;
                    call_method(&connection, Method::Play).await?;
                    call_method(&connection, Method::Next).await?;

                    restarted = true;
                    continue;
                }
            }
            message = owner.next() => {
                let Some(owner) = message else {
                    // stream dropped
                    return Ok(())
                };

                if owner.is_some() {
                    // gained a new owner
                    continue;
                };

                // lost an owner
                if restarted {
                    // owner loss caused by self
                    restarted = false;
                    continue;
                }

                // owner loss not caused by self
                return Ok(());
            }
        }
    }
}

fn track_id(message: &Message) -> Option<String> {
    let (_interface, changed, _invalidated): (String, HashMap<String, OwnedValue>, Vec<String>) =
        message
            .body()
            .deserialize()
            .expect("Failed to deserialize message");

    let metadata_value = changed.get("Metadata")?;
    let metadata: HashMap<String, OwnedValue> = metadata_value
        .clone()
        .try_into()
        .expect("Failed to deserialize metadata");

    let track_id_value = metadata.get("mpris:trackid")?;
    let track_id = track_id_value
        .clone()
        .try_into()
        .expect("Failed to deserialize track ID");

    return Some(track_id);
}

fn is_ad(track_id: &str) -> Option<bool> {
    let is_ad = track_id.split('/').nth(3)? == "ad";
    return Some(is_ad);
}

async fn call_method(
    connection: &Connection,
    method: Method,
) -> Result<zbus::Message, zbus::Error> {
    return connection
        .call_method(
            Some(DESTINATION),
            PATH,
            Some(method.interface()),
            method.name(),
            &(),
        )
        .await;
}

async fn wait_for_spotify_ready(connection: &Connection) {
    loop {
        if call_method(connection, Method::Ping).await.is_ok() {
            return;
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn spawn_spotify() -> Result<std::process::Child, std::io::Error> {
    Command::new("spotify").arg("--minimized").spawn()
}

fn restart_spotify(child: &mut Child) -> Result<(), std::io::Error> {
    child.kill()?;
    child.wait()?;

    *child = spawn_spotify()?;
    Ok(())
}
