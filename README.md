# pedalboard-sim

Virtual pedalboard simulator — develop and test without hardware.

Uses the same `Controller` from the protocol crate as the real firmware. Button logic, long-press detection, encoder acceleration, and MIDI output are identical.

## Quick Start

```bash
# Run with a YAML setlist (same file you upload to the device)
make run

# Or directly:
pedalboard-sim --yaml ../pedalboard-cli/examples/practice.yaml --web 0.0.0.0:3001
```

Open http://localhost:3001 for the web UI.

## Modes

```bash
# TUI only (terminal)
pedalboard-sim --yaml my-setlist.yaml

# TUI + Web UI
pedalboard-sim --yaml my-setlist.yaml --web 0.0.0.0:3001

# Binary config (legacy, postcard format)
pedalboard-sim --config config.bin

# Raw MIDI mode (no config, buttons send CCs)
make dev
```

## Controls

| Key | Action |
|-----|--------|
| A-F | Press button A-F |
| ←/→ | Turn encoder Vol |
| ↑/↓ | Turn encoder Gain |
| q | Quit |

## Web UI

The web UI at `--web <addr>` renders the pedalboard layout matching the real hardware:
- 6 foot buttons with LED rings (3×2 grid)
- 2 rotary encoders with heatmap rings
- 2 OLED display areas
- Mode/Mon indicator LEDs
- Keyboard + mouse + touch support
- Long-press detection (hold button > 500ms)

Both TUI and web UI control the same virtual pedalboard simultaneously.

## MIDI Output

Two output modes:

### Virtual ALSA port (default)
Creates a virtual ALSA sequencer port that any application can connect to:
- DAW (Ardour, Reaper, etc.)
- MOD UI (via JACK-MIDI)

### Raw output (`--raw <path>`)
Writes raw MIDI bytes to a file path (FIFO or device node). Use this for direct pedalboard-bridge integration:

```bash
# Terminal 1: Create FIFO and start bridge
mkfifo /tmp/midi-fifo
pedalboard-bridge -port /tmp/midi-fifo -addr :8080 -audio /etc/pedalboard/audio-patches.json -modhost localhost:5555

# Terminal 2: Start simulator writing to FIFO
make bridge
```

The bridge reads raw MIDI bytes from the FIFO exactly as it would from the real RP2040 USB device (`/dev/snd/midiC*D*`). Program Change messages trigger audio patch switching.

## Makefile

```bash
make run                          # run with practice.yaml + web UI
make run CONFIG=my-setlist.yaml   # use different config
make dev                          # raw MIDI mode
make bridge                       # run with bridge integration (FIFO)
```

## Architecture

```
midi-controller::Controller
       ├── pedalboard-midi     (firmware: RP2040 + RTIC)
       └── pedalboard-sim      (simulator: native Linux)
              ├── TUI          (crossterm terminal)
              └── Web UI       (axum + WebSocket)
```
