use clap::Parser;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

mod midi;
mod sim;
mod terminal;
mod web;

#[derive(Parser)]
#[command(
    name = "pedalboard-sim",
    about = "Virtual pedalboard simulator — develop and test without hardware"
)]
struct Cli {
    /// Binary config file (postcard format, as uploaded to device)
    #[arg(short, long, conflicts_with = "yaml")]
    config: Option<PathBuf>,

    /// YAML setlist file (runs compiler, same as what gets uploaded to device)
    #[arg(short = 'y', long)]
    yaml: Option<PathBuf>,

    /// MIDI output port name (creates virtual ALSA sequencer port)
    #[arg(short, long, default_value = "Pedalboard Sim")]
    port: String,

    /// Raw MIDI output path (FIFO or device, e.g. /tmp/midi-fifo).
    /// When set, writes raw bytes instead of using ALSA sequencer.
    /// Use this for pedalboard-bridge integration.
    #[arg(long)]
    raw: Option<PathBuf>,

    /// Use JACK MIDI output (connects to pedalboard-bridge via JACK).
    #[arg(long)]
    jack: bool,

    /// Start on this preset index
    #[arg(short = 'i', long, default_value = "0")]
    preset: usize,

    /// Start web UI server on this address (e.g. 0.0.0.0:3000)
    #[arg(short, long)]
    web: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Create MIDI output (virtual ALSA port, raw file, or JACK)
    let midi_out = if cli.jack {
        midi::open_jack("pedalboard-sim")?
    } else if let Some(path) = &cli.raw {
        midi::open_raw(path)?
    } else {
        midi::open_output(&cli.port)?
    };

    // Load config if provided
    let config = if let Some(path) = &cli.yaml {
        Some(sim::load_config_yaml(path)?)
    } else if let Some(path) = &cli.config {
        let data = std::fs::read(path)?;
        Some(sim::load_config_binary(&data)?)
    } else {
        None
    };

    match cli.web {
        Some(addr) => run_with_web(midi_out, config, cli.preset, addr),
        None => {
            // Run the interactive TUI without web server
            terminal::run(midi_out, config, cli.preset)?;
            Ok(())
        }
    }
}

fn run_with_web(
    midi_out: midi::MidiOut,
    config: Option<midi_controller::config::Config>,
    preset_index: usize,
    addr: String,
) -> anyhow::Result<()> {
    let pedalboard = config.map(|c| sim::Pedalboard::new(c, preset_index));
    let pedalboard = Arc::new(Mutex::new(pedalboard));
    let midi = Arc::new(Mutex::new(midi_out));
    let (notify_tx, _) = tokio::sync::broadcast::channel::<()>(64);

    let app_state = web::AppState {
        pedalboard: pedalboard.clone(),
        midi: midi.clone(),
        notify: notify_tx.clone(),
    };

    // Build the tokio runtime
    let rt = tokio::runtime::Runtime::new()?;

    // Spawn the web server in the runtime
    let web_addr = addr.clone();
    let web_handle = rt.spawn(async move {
        let app = web::router(app_state);
        let listener = tokio::net::TcpListener::bind(&web_addr).await.unwrap();
        eprintln!("✓ Web UI running at http://{}", web_addr);
        axum::serve(listener, app).await.unwrap();
    });

    // Spawn a tick task for long-press detection (10ms poll)
    let tick_pb = pedalboard.clone();
    let tick_midi = midi.clone();
    let tick_notify = notify_tx.clone();
    rt.spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(10));
        loop {
            interval.tick().await;
            let changed = {
                let mut pb = tick_pb.lock().unwrap();
                let mut m = tick_midi.lock().unwrap();
                if let Some(ref mut pb) = *pb {
                    if pb.any_active() {
                        pb.tick(&mut m);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            if changed {
                let _ = tick_notify.send(());
            }
        }
    });

    // Run the TUI on the main thread if we have a terminal, otherwise block on web
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() {
        let tui_result = terminal::run_shared(pedalboard, midi, notify_tx);
        rt.block_on(async {
            web_handle.abort();
        });
        tui_result
    } else {
        eprintln!("  (no terminal — web-only mode, Ctrl-C to quit)");
        rt.block_on(async {
            web_handle.await.unwrap();
        });
        Ok(())
    }
}
