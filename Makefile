.PHONY: build client client-dev

build: bin/repl-rs client

dev: target/debug/server
	cd server && cargo run

bin/repl-rs: target/release/server
	mkdir -p bin
	cp target/release/server bin/repl-rs

target/release/server: server/src/*
	cd server && cargo build --release

client: target/deploy/client-rs.wasm
client-dev:
	cd client-rs && cargo web start --target=wasm32-unknown-unknown

target/deploy/client-rs.wasm: client-rs/src/*
	cd client-rs && cargo web deploy --target=wasm32-unknown-unknown
