use std::{
    fs::{self, OpenOptions},
    io::Write,
    sync::mpsc::{self, Receiver},
    thread::{self},
    time::{Duration, Instant},
};

use console::{Key, Term};
use spotify::Spotify;

mod spotify;

const FILE_NAME: &str = "autoskip.txt";

fn main() {
    let term = Term::stdout();
    let _ = term.hide_cursor();

    let mut spotify = Spotify::spawn_new().unwrap();
    let mut autoskip_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(FILE_NAME)
        .unwrap();
    let mut autoskip = fs::read_to_string(FILE_NAME)
        .unwrap()
        .lines()
        .map(|str| str.to_string())
        .collect::<Vec<String>>();

    let mut now = Instant::now();
    let rx = spawn_input();

    let mut window_name = spotify.window_name().unwrap();
    loop {
        {
            use ansi_control_codes::{
                c0::CR,
                control_sequences::{CUU, EL, EraseLine},
            };
            print!("{}{}{}", CUU(None), EL(Some(EraseLine::BeginToEnd)), CR);
        }

        println!("[{:.1}] {window_name}", now.elapsed().as_secs_f32());

        if let Ok(exit) = rx.try_recv() {
            if exit {
                let _ = term.show_cursor();
                return;
            }

            autoskip.push(window_name.clone());
            let _ = writeln!(autoskip_file, "{}", window_name);
        }

        if now.elapsed() > Duration::from_secs(1) {
            if spotify.exited() != Some(false) {
                let _ = term.show_cursor();
                return;
            }

            now = Instant::now();
            window_name = spotify.window_name().unwrap();

            if autoskip.contains(&window_name) {
                println!("Skipping {}", window_name);
                let _ = spotify.restart();
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}

fn spawn_input() -> Receiver<bool> {
    let (tx, rx) = mpsc::channel::<bool>();
    let term = Term::stdout();
    let mut choice = false;

    let prompt = |choice: bool| {
        use ansi_control_codes::control_sequences::{EL, EraseLine};

        print!(
            "{}{}",
            EL(Some(EraseLine::BeginToEnd)),
            if choice { "[Y/n]" } else { "[y/N]" }
        );
    };

    prompt(choice);

    thread::spawn(move || {
        loop {
            let Ok(key) = term.read_key_raw() else {
                continue;
            };

            match key {
                Key::Char('y') | Key::Char('a') | Key::Char('h') | Key::ArrowLeft => {
                    if !choice {
                        choice = true;
                        prompt(choice);
                    }
                }
                Key::Char('n') | Key::Char('d') | Key::Char('l') | Key::ArrowRight => {
                    if choice {
                        choice = false;
                        prompt(choice);
                    }
                }
                Key::Enter => {
                    if choice {
                        let _ = tx.send(false);
                    }
                }
                Key::CtrlC => {
                    let _ = tx.send(true);
                    return;
                }
                _ => {}
            }
        }
    });

    return rx;
}
