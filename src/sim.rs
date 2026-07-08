use pedalboard_protocol::config::{Color, Config};
use pedalboard_protocol::controller::{Controller, ControllerResult, InputEvent};
use pedalboard_protocol::engine::{ActionStep, SystemAction};
use pedalboard_protocol::long_press::Edge;
use serde::Serialize;
use std::time::Instant;

use crate::midi::MidiOut;

/// Serializable state snapshot for the web UI
#[derive(Debug, Clone, Serialize)]
pub struct SimState {
    pub active_preset: usize,
    pub preset_name: String,
    pub num_presets: usize,
    pub buttons: Vec<ButtonState>,
    pub encoders: Vec<EncoderState>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ButtonState {
    pub index: usize,
    pub label: String,
    pub active: bool,
    pub color: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EncoderState {
    pub index: usize,
    pub label: String,
    pub value: u8,
}

fn color_to_css(color: &Color) -> String {
    match color {
        Color::Off => "#333333".to_string(),
        Color::Red => "#ff2222".to_string(),
        Color::Green => "#22ff22".to_string(),
        Color::Blue => "#2266ff".to_string(),
        Color::Yellow => "#ffdd00".to_string(),
        Color::Cyan => "#00ddff".to_string(),
        Color::Magenta => "#ff22ff".to_string(),
        Color::White => "#ffffff".to_string(),
        Color::Orange => "#ff8800".to_string(),
        Color::Purple => "#8800ff".to_string(),
        Color::Custom(r, g, b) => format!("#{:02x}{:02x}{:02x}", r, g, b),
    }
}

/// Simulated pedalboard — wraps the protocol crate's Controller.
pub struct Pedalboard {
    pub config: Config,
    pub active_preset: usize,
    controller: Controller,
    start_time: Instant,
}

impl Pedalboard {
    pub fn new(config: Config, preset_index: usize) -> Self {
        let mut ctrl = Controller::new();
        // Switch to the requested preset
        if preset_index > 0 && preset_index < config.presets.len() {
            if let Some(preset) = config.presets.get(preset_index) {
                ctrl.switch_preset(preset_index as u8, preset);
            }
        }
        Self {
            config,
            active_preset: preset_index,
            controller: ctrl,
            start_time: Instant::now(),
        }
    }

    /// Current monotonic time in milliseconds.
    fn now_ms(&self) -> u32 {
        self.start_time.elapsed().as_millis() as u32
    }

    /// Get the current preset name
    pub fn preset_name(&self) -> &str {
        self.config
            .presets
            .get(self.active_preset)
            .map(|p| p.name.as_str())
            .unwrap_or("(none)")
    }

    /// Process a button press (activate edge) and emit MIDI.
    pub fn press_button(&mut self, button_index: usize, midi: &mut MidiOut) {
        if let Some(preset) = self.config.presets.get(self.active_preset) {
            let now = self.now_ms();
            let result = self.controller.process(
                InputEvent::ButtonEdge {
                    index: button_index as u8,
                    edge: Edge::Activate,
                },
                now,
                preset,
            );
            self.emit_result(&result, midi);
        }
    }

    /// Process a button release (deactivate edge) and emit MIDI.
    pub fn release_button(&mut self, button_index: usize, midi: &mut MidiOut) {
        if let Some(preset) = self.config.presets.get(self.active_preset) {
            let now = self.now_ms();
            let result = self.controller.process(
                InputEvent::ButtonEdge {
                    index: button_index as u8,
                    edge: Edge::Deactivate,
                },
                now,
                preset,
            );
            self.emit_result(&result, midi);
        }
    }

    /// Tick the controller for long-press detection. Call periodically while buttons are held.
    pub fn tick(&mut self, midi: &mut MidiOut) {
        if self.controller.any_active() {
            if let Some(preset) = self.config.presets.get(self.active_preset) {
                let now = self.now_ms();
                let result = self.controller.tick(now, preset);
                self.emit_result(&result, midi);
            }
        }
    }

    /// Process an encoder turn and emit MIDI.
    /// Encoder acceleration is handled automatically by the Controller.
    pub fn turn_encoder(&mut self, encoder_index: usize, clockwise: bool, midi: &mut MidiOut) {
        if let Some(preset) = self.config.presets.get(self.active_preset) {
            let now = self.now_ms();
            let result = self.controller.process(
                InputEvent::EncoderTurn {
                    index: encoder_index as u8,
                    clockwise,
                },
                now,
                preset,
            );
            self.emit_result(&result, midi);
        }
    }

    /// Switch to a different preset
    pub fn switch_preset(&mut self, index: usize) {
        if index < self.config.presets.len() {
            if let Some(preset) = self.config.presets.get(index) {
                self.controller.switch_preset(index as u8, preset);
            }
            self.active_preset = index;
        }
    }

    /// Get button labels for the current preset
    pub fn button_labels(&self) -> Vec<&str> {
        self.config
            .presets
            .get(self.active_preset)
            .map(|p| p.buttons.iter().map(|b| b.label.as_str()).collect())
            .unwrap_or_default()
    }

    /// Returns true if any button is currently held.
    pub fn any_active(&self) -> bool {
        self.controller.any_active()
    }

    /// Create a serializable state snapshot for the web UI
    pub fn snapshot(&self) -> SimState {
        let preset = self.config.presets.get(self.active_preset);
        let button_active = self.controller.button_active();
        let encoder_values = self.controller.encoder_values();

        let buttons = (0..6)
            .map(|i| {
                let (label, color) = preset
                    .and_then(|p| p.buttons.get(i))
                    .map(|b| {
                        let c = if button_active[i] {
                            &b.color.on
                        } else {
                            &b.color.off
                        };
                        (b.label.as_str().to_string(), color_to_css(c))
                    })
                    .unwrap_or_else(|| (format!("Btn {}", i + 1), "#333333".to_string()));
                ButtonState {
                    index: i,
                    label,
                    active: button_active[i],
                    color,
                }
            })
            .collect();

        let encoders = (0..2)
            .map(|i| {
                let label = preset
                    .and_then(|p| p.encoders.get(i))
                    .map(|e| e.label.as_str().to_string())
                    .unwrap_or_else(|| format!("Enc {}", i));
                EncoderState {
                    index: i,
                    label,
                    value: encoder_values[i],
                }
            })
            .collect();

        SimState {
            active_preset: self.active_preset,
            preset_name: self.preset_name().to_string(),
            num_presets: self.config.presets.len(),
            buttons,
            encoders,
        }
    }

    /// Emit MIDI from a ControllerResult and handle system actions.
    fn emit_result(&mut self, result: &ControllerResult, midi: &mut MidiOut) {
        for step in &result.midi {
            match step {
                ActionStep::Send(msg) => {
                    midi.send(&msg.data[..msg.len]);
                }
                ActionStep::Delay(_ms) => {}
                ActionStep::SetLed { .. } => {}
            }
        }
        // Handle system actions (preset switching)
        for action in &result.system {
            match action {
                SystemAction::PresetNext => {
                    let next = (self.active_preset + 1) % self.config.presets.len();
                    self.switch_preset(next);
                }
                SystemAction::PresetPrev => {
                    let prev = if self.active_preset == 0 {
                        self.config.presets.len() - 1
                    } else {
                        self.active_preset - 1
                    };
                    self.switch_preset(prev);
                }
                SystemAction::PresetSelect(idx) => {
                    self.switch_preset(*idx as usize);
                }
                _ => {}
            }
        }
    }
}

/// Load a config from postcard bytes (same binary format the firmware uses)
pub fn load_config_binary(data: &[u8]) -> anyhow::Result<Config> {
    let config: Config = postcard::from_bytes(data)?;
    Ok(config)
}
