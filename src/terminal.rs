use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    cursor,
};
use std::io::{self, Write};

use pedalboard_protocol::config::Config;

use crate::midi::MidiOut;
use crate::sim::Pedalboard;

const BUTTON_KEYS: &[char] = &['1', '2', '3', '4', '5', '6'];
const BUTTON_LABELS: &[&str] = &["A", "B", "C", "D", "E", "F"];

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
                    KeyEvent { code: KeyCode::Char('q'), .. } |
                    KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, .. } => {
                        break;
                    }

                    // Button press (1-6)
                    KeyEvent { code: KeyCode::Char(c @ '1'..='6'), .. } => {
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
                    KeyEvent { code: KeyCode::Left, .. } => {
                        if let Some(ref mut pb) = pedalboard {
                            pb.turn_encoder(0, false, &mut midi);
                        }
                        last_action = "Encoder 0 ◀".to_string();
                    }
                    KeyEvent { code: KeyCode::Right, .. } => {
                        if let Some(ref mut pb) = pedalboard {
                            pb.turn_encoder(0, true, &mut midi);
                        }
                        last_action = "Encoder 0 ▶".to_string();
                    }

                    // Encoder 1: up/down
                    KeyEvent { code: KeyCode::Up, .. } => {
                        if let Some(ref mut pb) = pedalboard {
                            pb.turn_encoder(1, true, &mut midi);
                        }
                        last_action = "Encoder 1 ▲".to_string();
                    }
                    KeyEvent { code: KeyCode::Down, .. } => {
                        if let Some(ref mut pb) = pedalboard {
                            pb.turn_encoder(1, false, &mut midi);
                        }
                        last_action = "Encoder 1 ▼".to_string();
                    }

                    // Preset switching: F1-F9
                    KeyEvent { code: KeyCode::F(n @ 1..=9), .. } => {
                        let index = (n - 1) as usize;
                        if let Some(ref mut pb) = pedalboard {
                            pb.switch_preset(index);
                        }
                        midi.program_change(0, index as u8);
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

fn render(stdout: &mut impl Write, pedalboard: &Option<Pedalboard>, last_action: &str) -> io::Result<()> {
    execute!(io::stdout(), cursor::MoveTo(0, 0), Clear(ClearType::All))?;

    writeln!(stdout, "╔═══════════════════════════════════════════╗")?;
    writeln!(stdout, "║  PEDALBOARD SIMULATOR                     ║")?;
    writeln!(stdout, "╠═══════════════════════════════════════════╣")?;

    if let Some(pb) = pedalboard {
        writeln!(stdout, "║  Preset {}: {:<30}║", pb.active_preset, pb.preset_name())?;
        writeln!(stdout, "╠═══════════════════════════════════════════╣")?;
        let labels = pb.button_labels();
        for i in 0..6 {
            let label = labels.get(i).copied().unwrap_or(BUTTON_LABELS[i]);
            writeln!(stdout, "║  [{}]  {:<36}║", BUTTON_KEYS[i], label)?;
        }
    } else {
        writeln!(stdout, "║  No config loaded (raw MIDI mode)         ║")?;
        writeln!(stdout, "╠═══════════════════════════════════════════╣")?;
        for i in 0..6 {
            writeln!(stdout, "║  [{}]  CC {:<33}║", BUTTON_KEYS[i], 20 + i)?;
        }
    }

    writeln!(stdout, "╠═══════════════════════════════════════════╣")?;
    writeln!(stdout, "║  ←/→  Encoder 0     ↑/↓  Encoder 1       ║")?;
    writeln!(stdout, "║  F1-F9 Switch preset     q Quit           ║")?;
    writeln!(stdout, "╠═══════════════════════════════════════════╣")?;
    writeln!(stdout, "║  > {:<39}║", last_action)?;
    writeln!(stdout, "╚═══════════════════════════════════════════╝")?;

    stdout.flush()?;
    Ok(())
}
