use midir::os::unix::VirtualOutput;
use midir::{MidiOutput, MidiOutputConnection};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

enum Backend {
    Alsa(MidiOutputConnection),
    Raw(File),
    Jack(Arc<Mutex<Vec<Vec<u8>>>>),
}

pub struct MidiOut {
    backend: Backend,
    // Keep the JACK client alive
    _jack_client: Option<jack::AsyncClient<(), JackMidiProcess>>,
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
            Backend::Jack(pending) => {
                pending.lock().unwrap().push(msg.to_vec());
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
        _jack_client: None,
    })
}

/// Open a raw MIDI output (file, FIFO, or device node).
pub fn open_raw(path: &Path) -> anyhow::Result<MidiOut> {
    let file = OpenOptions::new().write(true).open(path).map_err(|e| {
        anyhow::anyhow!("Failed to open raw MIDI output '{}': {}", path.display(), e)
    })?;

    eprintln!("✓ Raw MIDI output: {}", path.display());
    Ok(MidiOut {
        backend: Backend::Raw(file),
        _jack_client: None,
    })
}

/// Open a JACK MIDI output port.
pub fn open_jack(client_name: &str) -> anyhow::Result<MidiOut> {
    let (client, _status) = jack::Client::new(client_name, jack::ClientOptions::NO_START_SERVER)?;

    let midi_out = client.register_port("midi_out", jack::MidiOut::default())?;
    let pending: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));

    let process = JackMidiProcess {
        port: midi_out,
        pending: pending.clone(),
    };

    let active_client = client.activate_async((), process)?;

    eprintln!("✓ JACK MIDI output: {}:midi_out", client_name);
    eprintln!(
        "  Connect with: jack_connect {}:midi_out pedalboard-bridge:midi_in",
        client_name
    );
    Ok(MidiOut {
        backend: Backend::Jack(pending),
        _jack_client: Some(active_client),
    })
}

/// JACK process handler that writes pending MIDI messages to the output port.
struct JackMidiProcess {
    port: jack::Port<jack::MidiOut>,
    pending: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl jack::ProcessHandler for JackMidiProcess {
    fn process(&mut self, _client: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        let mut writer = self.port.writer(ps);
        let mut pending = self.pending.lock().unwrap();
        for msg in pending.drain(..) {
            let _ = writer.write(&jack::RawMidi {
                time: 0,
                bytes: &msg,
            });
        }
        jack::Control::Continue
    }
}
