use std::time::Duration;

use tokio::process::{Child, Command};
use zbus::{
    Connection, Proxy,
    proxy::{OwnerChangedStream, SignalStream},
};

// busctl --user introspect org.mpris.MediaPlayer2.spotify /org/mpris/MediaPlayer2
pub const DESTINATION: &str = "org.mpris.MediaPlayer2.spotify";
pub const PATH: &str = "/org/mpris/MediaPlayer2";

pub struct SpotifySignals {
    pub connection: Connection,
    pub properties_changed: SignalStream<'static>,
    pub owner_changed: OwnerChangedStream<'static>,
}

pub async fn get_signals() -> Result<SpotifySignals, zbus::Error> {
    let connection = Connection::session().await?;

    let proxy = Proxy::new(
        &connection,
        DESTINATION,
        PATH,
        "org.freedesktop.DBus.Properties",
    )
    .await?;

    let properties_changed = proxy.receive_signal("PropertiesChanged").await?;
    let owner_changed = proxy.receive_owner_changed().await?;

    Ok(SpotifySignals {
        connection,
        properties_changed,
        owner_changed,
    })
}

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

#[derive(Debug, thiserror::Error)]
pub enum SpotifyError {
    #[error("Timed out waiting for Spotify to become ready after {0:?}")]
    ReadyTimeout(Duration),

    #[error("io error")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Zbus(#[from] zbus::Error),
}

pub struct Spotify {
    process: Child,
}

impl Spotify {
    pub async fn new(connection: &Connection) -> Result<Self, SpotifyError> {
        let process = Spotify::spawn_process()?;

        Spotify::wait_for_spotify_ready(connection, Duration::from_secs(5)).await?;
        call_method(connection, Method::Play).await?;

        Ok(Spotify { process })
    }

    pub async fn wait_for_spotify_ready(
        connection: &Connection,
        timeout_dur: Duration,
    ) -> Result<(), SpotifyError> {
        let ready = async {
            loop {
                match call_method(connection, Method::Ping).await {
                    Ok(_) => return Ok(()),
                    Err(_) => tokio::time::sleep(Duration::from_millis(200)).await,
                }
            }
        };

        match tokio::time::timeout(timeout_dur, ready).await {
            Ok(ready) => ready,
            Err(_) => Err(SpotifyError::ReadyTimeout(timeout_dur)),
        }
    }

    fn spawn_process() -> Result<Child, std::io::Error> {
        Command::new("spotify").arg("--minimized").spawn()
    }

    async fn restart(&mut self) -> Result<(), SpotifyError> {
        self.process.kill().await?;
        self.process = Spotify::spawn_process()?;
        Ok(())
    }

    pub async fn restart_and_skip(&mut self, connection: &Connection) -> Result<(), SpotifyError> {
        self.restart().await?;
        Spotify::wait_for_spotify_ready(connection, Duration::from_secs(2)).await?;
        call_method(connection, Method::Play).await?;
        call_method(connection, Method::Next).await?;
        Ok(())
    }
}
