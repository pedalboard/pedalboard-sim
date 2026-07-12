.PHONY: run dev bridge build help

CONFIG ?= ../pedalboard-cli/examples/practice.yaml
PORT ?= 3210
MIDI_FIFO ?= /tmp/midi-fifo

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

build: ## Build the simulator
	cargo build

run: build ## Run with YAML config and web UI
	cargo run -- --yaml $(CONFIG) --web 0.0.0.0:$(PORT)

dev: ## Run without config (raw MIDI mode)
	cargo run -- --web 0.0.0.0:$(PORT)

bridge: build ## Run with bridge integration (raw MIDI to FIFO)
	@test -p $(MIDI_FIFO) || (echo "Create FIFO first: mkfifo $(MIDI_FIFO)" && exit 1)
	cargo run -- --yaml $(CONFIG) --raw $(MIDI_FIFO) --web 0.0.0.0:$(PORT)
