# pedalboard-sim

Virtual pedalboard simulator — develop and test without hardware.

Uses the same `Controller` from the protocol crate as the real firmware. Button logic, long-press detection, encoder acceleration, and MIDI output are identical.

## Quick Start

```bash
# Compile a config and run with web UI
make run

# Or step by step:
pedalboard-cli compile ../pedalboard-cli/examples/practice.yaml -o config.bin
pedalboard-sim -c config.bin --web 0.0.0.0:3001
```

Open http://localhost:3001 for the web UI.

## Modes

```bash
# TUI only (terminal)
pedalboard-sim -c config.bin

# TUI + Web UI
pedalboard-sim -c config.bin --web 0.0.0.0:3001

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

Creates a virtual ALSA MIDI port that any application can connect to:
- `pedalboard-bridge` (for full audio chain testing)
- DAW (Ardour, Reaper, etc.)
- MOD UI (via mod-host)

## Makefile

```bash
make run                          # compile practice.yaml + run
make run CONFIG=my-config.yaml    # use different config
make dev                          # raw MIDI mode
make compile                      # just compile, don't run
```

## Architecture

```
pedalboard-protocol::Controller
       ├── pedalboard-midi     (firmware: RP2040 + RTIC)
       └── pedalboard-sim      (simulator: native Linux)
              ├── TUI          (crossterm terminal)
              └── Web UI       (axum + WebSocket)
```
