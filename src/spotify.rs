use std::{
    io,
    process::{Child, Command},
};

pub struct Spotify {
    process: Child,
    window_pid: String,
}

impl Spotify {
    pub fn spawn_new() -> io::Result<Self> {
        let spotify = Spotify {
            process: Command::new("spotify").spawn()?,
            window_pid: Spotify::window_pid(),
        };

        spotify.minimize();
        spotify.play_song();

        Ok(spotify)
    }

    pub fn restart(&mut self) -> io::Result<()> {
        let _ = self.process.kill();

        self.process = Command::new("spotify").spawn()?;
        self.window_pid = Spotify::window_pid();

        self.minimize();
        self.play_song();
        self.skip_song();

        Ok(())
    }

    pub fn window_name(&self) -> io::Result<String> {
        let window_name = Command::new("xdotool")
            .arg("getwindowname")
            .arg(&self.window_pid)
            .output()?;

        let name = window_name
            .stdout
            .into_iter()
            .map(|ascii| ascii as char)
            .collect::<String>()
            .trim()
            .to_owned();

        return Ok(name);
    }

    pub fn exited(&mut self) -> Option<bool> {
        match self.process.try_wait() {
            Ok(Some(_)) => Some(true),
            Ok(None) => Some(false),
            Err(_) => None,
        }
    }

    fn window_pid() -> String {
        let spotify_search = Command::new("xdotool")
            .arg("search")
            .arg("--sync")
            .arg("--name")
            .arg("spotify free")
            .output()
            .unwrap();

        let stdout = spotify_search
            .stdout
            .into_iter()
            .map(|ascii| ascii as char)
            .collect::<String>();

        return stdout.trim().to_owned();
    }

    fn minimize(&self) {
        let _ = Command::new("xdotool")
            .arg("windowminimize")
            .arg(&self.window_pid)
            .output();
    }

    fn play_song(&self) {
        let _ = Command::new("xdotool")
            .arg("key")
            .arg("XF86AudioPlay")
            .output();
    }

    fn skip_song(&self) {
        let _ = Command::new("xdotool")
            .arg("key")
            .arg("XF86AudioNext")
            .output();
    }
}
