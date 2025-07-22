use std::{fs, thread::sleep, time::Duration};

use spotify::Spotify;

mod spotify;

fn main() {
    let mut spotify = Spotify::spawn_new().unwrap();
    let file_contents = fs::read_to_string("autoskip.txt").unwrap();
    let autoskip = file_contents.lines().collect::<Vec<&str>>();

    while spotify.exited() == Some(false) {
        // fairly confident this will never panic
        let window_name = spotify.window_name().unwrap();

        if !window_name.contains("Spotify Free\n")
            && (!window_name.contains('-') || autoskip.contains(&window_name.trim()))
        {
            println!("{window_name}");
            let _ = spotify.restart();
        }

        sleep(Duration::from_secs(1));
    }
}
