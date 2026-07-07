use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    cursor,
};
use std::io::{self, Write};

use pedalboard_protocol::config::Config;

use crate::midi::MidiOut;
use crate::sim::Pedalboard;

/// Key mapping for the virtual pedalboard
/// Buttons A-F mapped to keys 1-6 (or a-f)
/// Encoders: left/right arrows (encoder 0), up/down for encoder 1
/// Preset switching: F1-F9

const BUTTON_KEYS: &[(char, &str)] = &[
    ('1', "A"),
    ('2', "B"),
    ('3', "C"),
    ('4', "D"),
    ('5', "E"),
    ('6', "F"),
];

pub fn run(mut midi: MidiOut, config: Option<Config>, preset_index: usize) -> anyhow::Result<()> {
    let mut pedalboard = config.map(|c| Pedalboard::new(c, preset_index));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;

    print_ui(&mut stdout, &pedalboard)?;

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
                            // Simulate immediate release for toggle behavior
                            pb.release_button(index, &mut midi);
                        } else {
                            // No config — send raw CC
                            midi.cc(0, 20 + index as u8, 127);
                        }
                        execute!(stdout, cursor::MoveTo(0, 12))?;
                        writeln!(stdout, "  → Button {} pressed", BUTTON_KEYS[index as usize].1)?;
                    }

                    // Encoder 0: left/right
                    KeyEvent { code: KeyCode::Left, .. } => {
                        if let Some(ref mut pb) = pedalboard {
                            pb.turn_encoder(0, false, &mut midi);
                        }
                        execute!(stdout, cursor::MoveTo(0, 12))?;
                        writeln!(stdout, "  → Encoder 0 ←")?;
                    }
                    KeyEvent { code: KeyCode::Right, .. } => {
                        if let Some(ref mut pb) = pedalboard {
                            pb.turn_encoder(0, true, &mut midi);
                        }
                        execute!(stdout, cursor::MoveTo(0, 12))?;
                        writeln!(stdout, "  → Encoder 0 →")?;
                    }

                    // Encoder 1: up/down
                    KeyEvent { code: KeyCode::Up, .. } => {
                        if let Some(ref mut pb) = pedalboard {
                            pb.turn_encoder(1, true, &mut midi);
                        }
                        execute!(stdout, cursor::MoveTo(0, 12))?;
                        writeln!(stdout, "  → Encoder 1 ↑")?;
                    }
                    KeyEvent { code: KeyCode::Down, .. } => {
                        if let Some(ref mut pb) = pedalboard {
                            pb.turn_encoder(1, false, &mut midi);
                        }
                        execute!(stdout, cursor::MoveTo(0, 12))?;
                        writeln!(stdout, "  → Encoder 1 ↓")?;
                    }

                    // Preset switching: F1-F9
                    KeyEvent { code: KeyCode::F(n @ 1..=9), .. } => {
                        let index = (n - 1) as usize;
                        if let Some(ref mut pb) = pedalboard {
                            pb.switch_preset(index);
                        }
                        midi.program_change(0, index as u8);
                        execute!(stdout, cursor::MoveTo(0, 0))?;
                        print_ui(&mut stdout, &pedalboard)?;
                    }

                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(stdout, cursor::Show)?;
    println!("\nBye!");
    Ok(())
}

fn print_ui(stdout: &mut impl Write, pedalboard: &Option<Pedalboard>) -> io::Result<()> {
    writeln!(stdout, "┌─────────────────────────────────────────┐")?;
    writeln!(stdout, "│  PEDALBOARD SIMULATOR                   │")?;
    writeln!(stdout, "├─────────────────────────────────────────┤")?;

    if let Some(pb) = pedalboard {
        let labels = pb.button_labels();
        writeln!(stdout, "│  Preset {}: {:<28} │", pb.active_preset, pb.preset_name())?;
        writeln!(stdout, "├─────────────────────────────────────────┤")?;
        for (i, (key, default_label)) in BUTTON_KEYS.iter().enumerate() {
            if let Some(label) = labels.get(i) {
                writeln!(stdout, "│  [{}] {:<35} │", key, label)?;
            } else {
                writeln!(stdout, "│  [{}] {:<35} │", key, default_label)?;
            }
        }
    } else {
        writeln!(stdout, "│  No config loaded (raw MIDI mode)       │")?;
        writeln!(stdout, "│                                         │")?;
        for (key, label) in BUTTON_KEYS {
            writeln!(stdout, "│  [{}] → CC {:<28} │", key, label)?;
        }
    }

    writeln!(stdout, "├─────────────────────────────────────────┤")?;
    writeln!(stdout, "│  ←/→ Encoder 0   ↑/↓ Encoder 1         │")?;
    writeln!(stdout, "│  F1-F9 Switch preset   q Quit           │")?;
    writeln!(stdout, "└─────────────────────────────────────────┘")?;
    writeln!(stdout)?;
    stdout.flush()?;
    Ok(())
}
