use midir::os::unix::VirtualOutput;
use midir::{MidiOutput, MidiOutputConnection};

pub struct MidiOut {
    conn: MidiOutputConnection,
}

impl MidiOut {
    pub fn send(&mut self, msg: &[u8]) {
        if let Err(e) = self.conn.send(msg) {
            eprintln!("MIDI send error: {}", e);
        }
    }

    /// Send a MIDI CC message
    pub fn cc(&mut self, channel: u8, cc: u8, value: u8) {
        self.send(&[0xB0 | (channel & 0x0F), cc & 0x7F, value & 0x7F]);
    }

    /// Send a MIDI Program Change
    pub fn program_change(&mut self, channel: u8, program: u8) {
        self.send(&[0xC0 | (channel & 0x0F), program & 0x7F]);
    }
}

pub fn open_output(port_name: &str) -> anyhow::Result<MidiOut> {
    let output = MidiOutput::new("pedalboard-sim")?;

    let conn = output
        .create_virtual(port_name)
        .map_err(|e| anyhow::anyhow!("Failed to create virtual MIDI port '{}': {}", port_name, e))?;

    eprintln!("✓ Virtual MIDI port created: \"{}\"", port_name);
    eprintln!("  Connect your DAW or bridge to this port to receive MIDI.");
    Ok(MidiOut { conn })
}
