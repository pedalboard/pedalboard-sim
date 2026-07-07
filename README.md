# pedalboard-sim

Virtual pedalboard simulator — develop and test without hardware.

Uses the same `pedalboard-protocol` crate as the real firmware, so button logic, encoder acceleration, LED behavior, and MIDI output are identical.

## Usage

```bash
# Run without config (raw MIDI mode — buttons send CCs)
pedalboard-sim

# Run with a config (same binary format uploaded to the device)
pedalboard-sim -c my-preset.bin

# Custom MIDI port name
pedalboard-sim -p "My Pedalboard"
```

## Controls

| Key | Action |
|-----|--------|
| 1-6 | Press button A-F |
| ←/→ | Turn encoder 0 |
| ↑/↓ | Turn encoder 1 |
| F1-F9 | Switch preset |
| q | Quit |

## MIDI Output

The simulator creates a virtual ALSA MIDI port that any application can connect to:
- Bridge (`pedalboard-bridge`)
- DAW (Ardour, Reaper, etc.)
- `aconnect` / `jack_connect` for manual routing
- MOD UI (via mod-host MIDI input)

## Development

```bash
# Build
cargo build

# Run (requires ALSA for virtual MIDI)
cargo run

# Connect bridge to the simulator
pedalboard-bridge -port "Pedalboard Sim" -addr :8080 -audio audio-patches.json
```

## Architecture

```
pedalboard-protocol (shared logic: button state machines, MIDI generation)
       ├── pedalboard-midi     (real hardware: RP2040 + RTIC)
       └── pedalboard-sim      (simulator: native Linux + virtual MIDI)
```

The simulator proves the protocol crate's abstraction is correct — if it works on both RP2040 and x86_64 Linux, the logic is hardware-independent.
