use std::{process::Command, time::Duration};
use zbus::{blocking::Connection, zvariant::OwnedValue};

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

pub fn main() {
    let connection = Connection::session().unwrap();

    let mut child = Command::new("spotify").spawn().unwrap();
    wait_for_spotify_ready(&connection);
    let _ = call_method(&connection, Method::Play);

    while let Ok(can_go) = get_boolean_property(&connection, "CanGoNext") {
        if can_go {
            std::thread::sleep(Duration::from_secs(1));
            continue;
        }

        let _ = child.kill();
        child = Command::new("spotify").spawn().unwrap();
        wait_for_spotify_ready(&connection);
        let _ = call_method(&connection, Method::Next);
        let _ = call_method(&connection, Method::Play);
    }
}

fn get_boolean_property(connection: &Connection, property: &str) -> Result<bool, zbus::Error> {
    return Ok(connection
        .call_method(
            Some(DESTINATION),
            PATH,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &("org.mpris.MediaPlayer2.Player", property),
        )?
        .body()
        .deserialize::<OwnedValue>()?
        .try_into()?);
}

fn call_method(connection: &Connection, method: Method) -> Result<zbus::Message, zbus::Error> {
    return connection.call_method(
        Some(DESTINATION),
        PATH,
        Some(method.interface()),
        method.name(),
        &(),
    );
}

fn wait_for_spotify_ready(connection: &Connection) {
    loop {
        if call_method(connection, Method::Ping).is_ok() {
            return;
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}
