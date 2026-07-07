use clap::Parser;
use std::path::PathBuf;

mod midi;
mod sim;
mod terminal;

#[derive(Parser)]
#[command(name = "pedalboard-sim", about = "Virtual pedalboard simulator — develop and test without hardware")]
struct Cli {
    /// Binary config file (postcard format, as uploaded to device)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// MIDI output port name (creates virtual port)
    #[arg(short, long, default_value = "Pedalboard Sim")]
    port: String,

    /// Start on this preset index
    #[arg(short = 'i', long, default_value = "0")]
    preset: usize,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Create virtual MIDI output
    let midi_out = midi::open_output(&cli.port)?;

    // Load config if provided
    let config = match &cli.config {
        Some(path) => {
            let data = std::fs::read(path)?;
            Some(sim::load_config_binary(&data)?)
        }
        None => None,
    };

    // Run the interactive TUI
    terminal::run(midi_out, config, cli.preset)?;

    Ok(())
}
