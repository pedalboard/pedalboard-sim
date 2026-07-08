use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use pedalboard_protocol::config::Config;

use crate::midi::MidiOut;
use crate::sim::Pedalboard;

const BUTTON_KEYS: &[char] = &['1', '2', '3', '4', '5', '6'];
const BUTTON_LABELS: &[&str] = &["A", "B", "C", "D", "E", "F"];

/// Run the TUI without shared state (standalone mode, no web server).
pub fn run(mut midi: MidiOut, config: Option<Config>, preset_index: usize) -> anyhow::Result<()> {
    let mut pedalboard = config.map(|c| Pedalboard::new(c, preset_index));
    let mut last_action = String::new();

    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    render(&mut stdout, &pedalboard, &last_action)?;

    loop {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key {
                    // Quit
                    KeyEvent {
                        code: KeyCode::Char('q'),
                        ..
                    }
                    | KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    } => {
                        break;
                    }

                    // Button press (1-6)
                    KeyEvent {
                        code: KeyCode::Char(c @ '1'..='6'),
                        ..
                    } => {
                        let index = (c as u8 - b'1') as usize;
                        if let Some(ref mut pb) = pedalboard {
                            pb.press_button(index, &mut midi);
                            pb.release_button(index, &mut midi);
                        } else {
                            midi.cc(0, 20 + index as u8, 127);
                        }
                        last_action = format!("Button {} pressed", BUTTON_LABELS[index]);
                    }

                    // Encoder 0: left/right
                    KeyEvent {
                        code: KeyCode::Left,
                        ..
                    } => {
                        if let Some(ref mut pb) = pedalboard {
                            pb.turn_encoder(0, false, &mut midi);
                        }
                        last_action = "Encoder 0 ◀".to_string();
                    }
                    KeyEvent {
                        code: KeyCode::Right,
                        ..
                    } => {
                        if let Some(ref mut pb) = pedalboard {
                            pb.turn_encoder(0, true, &mut midi);
                        }
                        last_action = "Encoder 0 ▶".to_string();
                    }

                    // Encoder 1: up/down
                    KeyEvent {
                        code: KeyCode::Up, ..
                    } => {
                        if let Some(ref mut pb) = pedalboard {
                            pb.turn_encoder(1, true, &mut midi);
                        }
                        last_action = "Encoder 1 ▲".to_string();
                    }
                    KeyEvent {
                        code: KeyCode::Down,
                        ..
                    } => {
                        if let Some(ref mut pb) = pedalboard {
                            pb.turn_encoder(1, false, &mut midi);
                        }
                        last_action = "Encoder 1 ▼".to_string();
                    }

                    // Preset switching: F1-F9
                    KeyEvent {
                        code: KeyCode::F(n @ 1..=9),
                        ..
                    } => {
                        let index = (n - 1) as usize;
                        if let Some(ref mut pb) = pedalboard {
                            pb.switch_preset(index, &mut midi);
                        } else {
                            midi.program_change(0, index as u8);
                        }
                        last_action = format!("Switched to preset {}", index);
                    }

                    _ => continue,
                }

                render(&mut stdout, &pedalboard, &last_action)?;
            }
        }
    }

    execute!(stdout, LeaveAlternateScreen, cursor::Show)?;
    disable_raw_mode()?;
    Ok(())
}

/// Run the TUI with shared state and broadcast channel (web mode).
/// The pedalboard and midi are shared with the web server via Arc<Mutex<>>.
pub fn run_shared(
    pedalboard: Arc<Mutex<Option<Pedalboard>>>,
    midi: Arc<Mutex<MidiOut>>,
    notify: tokio::sync::broadcast::Sender<()>,
) -> anyhow::Result<()> {
    let mut last_action = String::new();

    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    {
        let pb = pedalboard.lock().unwrap();
        render(&mut stdout, &*pb, &last_action)?;
    }

    loop {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key {
                    // Quit
                    KeyEvent {
                        code: KeyCode::Char('q'),
                        ..
                    }
                    | KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    } => {
                        break;
                    }

                    // Button press (1-6)
                    KeyEvent {
                        code: KeyCode::Char(c @ '1'..='6'),
                        ..
                    } => {
                        let index = (c as u8 - b'1') as usize;
                        {
                            let mut pb = pedalboard.lock().unwrap();
                            let mut m = midi.lock().unwrap();
                            if let Some(ref mut pb) = *pb {
                                pb.press_button(index, &mut m);
                                pb.release_button(index, &mut m);
                            } else {
                                m.cc(0, 20 + index as u8, 127);
                            }
                        }
                        last_action = format!("Button {} pressed", BUTTON_LABELS[index]);
                    }

                    // Encoder 0: left/right
                    KeyEvent {
                        code: KeyCode::Left,
                        ..
                    } => {
                        let mut pb = pedalboard.lock().unwrap();
                        let mut m = midi.lock().unwrap();
                        if let Some(ref mut pb) = *pb {
                            pb.turn_encoder(0, false, &mut m);
                        }
                        last_action = "Encoder 0 ◀".to_string();
                    }
                    KeyEvent {
                        code: KeyCode::Right,
                        ..
                    } => {
                        let mut pb = pedalboard.lock().unwrap();
                        let mut m = midi.lock().unwrap();
                        if let Some(ref mut pb) = *pb {
                            pb.turn_encoder(0, true, &mut m);
                        }
                        last_action = "Encoder 0 ▶".to_string();
                    }

                    // Encoder 1: up/down
                    KeyEvent {
                        code: KeyCode::Up, ..
                    } => {
                        let mut pb = pedalboard.lock().unwrap();
                        let mut m = midi.lock().unwrap();
                        if let Some(ref mut pb) = *pb {
                            pb.turn_encoder(1, true, &mut m);
                        }
                        last_action = "Encoder 1 ▲".to_string();
                    }
                    KeyEvent {
                        code: KeyCode::Down,
                        ..
                    } => {
                        let mut pb = pedalboard.lock().unwrap();
                        let mut m = midi.lock().unwrap();
                        if let Some(ref mut pb) = *pb {
                            pb.turn_encoder(1, false, &mut m);
                        }
                        last_action = "Encoder 1 ▼".to_string();
                    }

                    // Preset switching: F1-F9
                    KeyEvent {
                        code: KeyCode::F(n @ 1..=9),
                        ..
                    } => {
                        let index = (n - 1) as usize;
                        {
                            let mut pb = pedalboard.lock().unwrap();
                            let mut m = midi.lock().unwrap();
                            if let Some(ref mut pb) = *pb {
                                pb.switch_preset(index, &mut m);
                            } else {
                                m.program_change(0, index as u8);
                            }
                        }
                        last_action = format!("Switched to preset {}", index);
                    }

                    _ => continue,
                }

                // Notify web clients of state change
                let _ = notify.send(());

                {
                    let pb = pedalboard.lock().unwrap();
                    render(&mut stdout, &*pb, &last_action)?;
                }
            }
        }
    }

    execute!(stdout, LeaveAlternateScreen, cursor::Show)?;
    disable_raw_mode()?;
    Ok(())
}

fn render(
    stdout: &mut impl Write,
    pedalboard: &Option<Pedalboard>,
    last_action: &str,
) -> io::Result<()> {
    execute!(io::stdout(), cursor::MoveTo(0, 0), Clear(ClearType::All))?;

    write!(stdout, "+-------------------------------------------+\r\n")?;
    write!(stdout, "|  PEDALBOARD SIMULATOR                     |\r\n")?;
    write!(stdout, "+-------------------------------------------+\r\n")?;

    if let Some(pb) = pedalboard {
        write!(
            stdout,
            "|  Preset {}: {:<30}|\r\n",
            pb.active_preset(),
            pb.preset_name()
        )?;
        write!(stdout, "+-------------------------------------------+\r\n")?;
        let labels = pb.button_labels();
        for i in 0..6 {
            let label = labels.get(i).copied().unwrap_or(BUTTON_LABELS[i]);
            write!(stdout, "|  [{}]  {:<36}|\r\n", BUTTON_KEYS[i], label)?;
        }
    } else {
        write!(stdout, "|  No config loaded (raw MIDI mode)         |\r\n")?;
        write!(stdout, "+-------------------------------------------+\r\n")?;
        for i in 0..6 {
            write!(stdout, "|  [{}]  CC {:<33}|\r\n", BUTTON_KEYS[i], 20 + i)?;
        }
    }

    write!(stdout, "+-------------------------------------------+\r\n")?;
    write!(stdout, "|  <-/->  Encoder 0   up/dn  Encoder 1      |\r\n")?;
    write!(stdout, "|  F1-F9  Switch preset       q  Quit       |\r\n")?;
    write!(stdout, "+-------------------------------------------+\r\n")?;
    write!(stdout, "|  > {:<39}|\r\n", last_action)?;
    write!(stdout, "+-------------------------------------------+\r\n")?;

    stdout.flush()?;
    Ok(())
}
