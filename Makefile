.PHONY: run dev bridge build compile help

CONFIG ?= ../pedalboard-cli/examples/practice.yaml
BIN = config.bin
PORT ?= 3001
MIDI_FIFO ?= /tmp/midi-fifo

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

compile: ## Compile YAML config to binary
	cd ../pedalboard-cli && cargo run -- compile ../pedalboard-sim/$(CONFIG) -o ../pedalboard-sim/$(BIN)

build: ## Build the simulator
	cargo build

run: compile build ## Compile config and run with web UI
	cargo run -- -c $(BIN) --web 0.0.0.0:$(PORT)

dev: ## Run without config (raw MIDI mode)
	cargo run -- --web 0.0.0.0:$(PORT)

bridge: compile build ## Run with bridge integration (raw MIDI to FIFO)
	@test -p $(MIDI_FIFO) || (echo "Create FIFO first: mkfifo $(MIDI_FIFO)" && exit 1)
	cargo run -- -c $(BIN) --raw $(MIDI_FIFO) --web 0.0.0.0:$(PORT)
