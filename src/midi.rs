use midir::os::unix::VirtualOutput;
use midir::{MidiOutput, MidiOutputConnection};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

enum Backend {
    Alsa(MidiOutputConnection),
    Raw(File),
}

pub struct MidiOut {
    backend: Backend,
}

impl MidiOut {
    pub fn send(&mut self, msg: &[u8]) {
        match &mut self.backend {
            Backend::Alsa(conn) => {
                if let Err(e) = conn.send(msg) {
                    eprintln!("MIDI send error: {}", e);
                }
            }
            Backend::Raw(file) => {
                if let Err(e) = file.write_all(msg) {
                    eprintln!("MIDI raw write error: {}", e);
                }
            }
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

/// Create a virtual ALSA MIDI port (for DAW / sequencer use).
pub fn open_output(port_name: &str) -> anyhow::Result<MidiOut> {
    let output = MidiOutput::new("pedalboard-sim")?;

    let conn = output.create_virtual(port_name).map_err(|e| {
        anyhow::anyhow!("Failed to create virtual MIDI port '{}': {}", port_name, e)
    })?;

    eprintln!("✓ Virtual MIDI port created: \"{}\"", port_name);
    eprintln!("  Connect your DAW or bridge to this port to receive MIDI.");
    Ok(MidiOut {
        backend: Backend::Alsa(conn),
    })
}

/// Open a raw MIDI output (file, FIFO, or device node).
/// Writes raw MIDI bytes directly — suitable for pedalboard-bridge integration.
pub fn open_raw(path: &Path) -> anyhow::Result<MidiOut> {
    let file = OpenOptions::new().write(true).open(path).map_err(|e| {
        anyhow::anyhow!("Failed to open raw MIDI output '{}': {}", path.display(), e)
    })?;

    eprintln!("✓ Raw MIDI output: {}", path.display());
    eprintln!("  Bytes written here are read by pedalboard-bridge as MIDI input.");
    Ok(MidiOut {
        backend: Backend::Raw(file),
    })
}
