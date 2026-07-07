use pedalboard_protocol::action::EncoderDirection;
use pedalboard_protocol::config::Config;
use pedalboard_protocol::engine::{self, ActionStep, ButtonEvent, EngineResult};
use pedalboard_protocol::state::PresetState;

use crate::midi::MidiOut;

/// Simulated pedalboard state
pub struct Pedalboard {
    pub config: Config,
    pub state: PresetState,
    pub active_preset: usize,
}

impl Pedalboard {
    pub fn new(config: Config, preset_index: usize) -> Self {
        let state = config
            .presets
            .get(preset_index)
            .map(PresetState::from_defaults)
            .unwrap_or_default();
        Self {
            config,
            state,
            active_preset: preset_index,
        }
    }

    /// Get the current preset name
    pub fn preset_name(&self) -> &str {
        self.config
            .presets
            .get(self.active_preset)
            .map(|p| p.name.as_str())
            .unwrap_or("(none)")
    }

    /// Process a button press event and emit MIDI
    pub fn press_button(&mut self, button_index: usize, midi: &mut MidiOut) {
        if let Some(preset) = self.config.presets.get(self.active_preset) {
            let result = engine::process_button(
                &mut self.state,
                preset,
                button_index,
                ButtonEvent::Press,
            );
            self.emit_midi(&result, midi);
            self.handle_system(&result);
        }
    }

    /// Process a button release event and emit MIDI
    pub fn release_button(&mut self, button_index: usize, midi: &mut MidiOut) {
        if let Some(preset) = self.config.presets.get(self.active_preset) {
            let result = engine::process_button(
                &mut self.state,
                preset,
                button_index,
                ButtonEvent::Release,
            );
            self.emit_midi(&result, midi);
            self.handle_system(&result);
        }
    }

    /// Process an encoder turn and emit MIDI
    pub fn turn_encoder(&mut self, encoder_index: usize, clockwise: bool, midi: &mut MidiOut) {
        if let Some(preset) = self.config.presets.get(self.active_preset) {
            let direction = if clockwise {
                EncoderDirection::Clockwise
            } else {
                EncoderDirection::CounterClockwise
            };
            let result = engine::process_encoder(
                &mut self.state,
                preset,
                encoder_index,
                direction,
                1,
            );
            self.emit_midi(&result, midi);
            self.handle_system(&result);
        }
    }

    /// Switch to a different preset
    pub fn switch_preset(&mut self, index: usize) {
        if index < self.config.presets.len() {
            self.active_preset = index;
            self.state = self
                .config
                .presets
                .get(index)
                .map(PresetState::from_defaults)
                .unwrap_or_default();
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

    /// Convert engine result into MIDI messages
    fn emit_midi(&self, result: &EngineResult, midi: &mut MidiOut) {
        for step in &result.midi {
            match step {
                ActionStep::Send(msg) => {
                    midi.send(&msg.data[..msg.len]);
                }
                ActionStep::Delay(_ms) => {
                    // In the simulator we could sleep, but skip for now
                }
                ActionStep::SetLed { .. } => {
                    // LED state changes — could render in TUI
                }
            }
        }
    }

    /// Handle system actions (preset switching)
    fn handle_system(&mut self, result: &EngineResult) {
        use pedalboard_protocol::engine::SystemAction;
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
