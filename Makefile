.PHONY: build client client-dev

build: bin/repl-rs client

server-dev:
	cd server && cargo run

bin/repl-rs: target/release/server
	mkdir -p bin
	cp target/release/server bin/repl-rs

target/release/server: server/src/*
	cd server && cargo build --release

client: target/deploy/client.wasm
client-dev:
	cd client && cargo web start --target=wasm32-unknown-unknown

target/deploy/client.wasm: client/src/*
	cd client && cargo web deploy --target=wasm32-unknown-unknown
