use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use std::{error::Error, io};
use crossterm::event::{Event, KeyCode, self};
use rusty_audio::Audio;
use space_invaders::frame::{new_frame, Drawable};
use space_invaders::invaders::Invaders;
use space_invaders::player::Player;
use space_invaders::{render, frame};
use std::path::Path;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, cursor};

fn main() -> Result<(), Box<dyn Error>> {
    let mut audio = Audio::new();
    audio.add("explode", Path::new("./assets/sounds/explode.wav"));
    audio.add("lose", Path::new("./assets/sounds/lose.wav"));
    audio.add("move", Path::new("./assets/sounds/move.wav"));
    audio.add("pew", Path::new("./assets/sounds/pew.wav"));
    audio.add("startup", Path::new("./assets/sounds/startup.wav"));
    audio.add("win", Path::new("./assets/sounds/win.wav"));
    audio.play("startup");

    // Terminal
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, cursor::Hide)?;

    // render loop in different thread
    let (render_tx, render_rx) = mpsc::channel();
    let render_handle = thread::spawn(move || {
        let mut last_frame = frame::new_frame();
        let mut stdout = io::stdout();
        render::render(&mut stdout, &last_frame, &last_frame, true);
        loop {
            let curr_frame = match render_rx.recv() {
                Ok(x) => x,
                Err(_) => break
            };
            render::render(&mut stdout, &last_frame, &curr_frame, false);
            last_frame = curr_frame;
        }
    });

    let mut player = Player::new();
    let mut instant = Instant::now();
    let mut invaders = Invaders::new();
    'gameloop: loop {
        // Per-frame init
        let mut curr_frame = new_frame();
        let delta = instant.elapsed();
        instant = Instant::now();

        while event::poll(Duration::default())? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        audio.play("lose");
                        break 'gameloop;
                    }
                    KeyCode::Left => player.move_left(),
                    KeyCode::Right => player.move_right(),
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        if player.shoot() {
                            audio.play("pew");
                        }
                    }
                    _ => {}
                }
            }
        }

        // Updates
        player.update(delta);
        if invaders.update(delta) {
            audio.play("move");
        }
        if player.detect_hits(&mut invaders) {
            audio.play("explode");
        }
        
        // Draw & Render
        let drawables: Vec<&dyn Drawable> = vec![&player, &invaders];
        for drawable in drawables {
            drawable.draw(&mut curr_frame);
        }

        let _ = render_tx.send(curr_frame);
        thread::sleep(Duration::from_millis(1));

        // Win or lose
        if invaders.all_killed() {
            audio.play("win");
            break 'gameloop;
        }

        if invaders.reached_bottom() {
           audio.play("lose") ;
           break 'gameloop;
        }
    }

    drop(render_tx);
    render_handle.join().unwrap();
    
    audio.wait();
    execute!(stdout, cursor::Show)?;
    execute!(stdout, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    Ok(())
}
