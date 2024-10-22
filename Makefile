
all: env.tmp
	cargo build --release
	cargo build --release --examples
	cargo build
	cargo build --examples

env.tmp:
	sudo sysctl -w vm.max_map_count=$(shell python3 -c "print(1<<30)") && touch $@
