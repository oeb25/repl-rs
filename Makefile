.PHONY: build

build: bin/repl-rs

dev: target/debug/server
	cd server && cargo run

bin/repl-rs: target/release/server
	mkdir -p bin
	cp target/release/server bin/repl-rs

target/release/server: client/dist/index.html server/src/*.rs
	cd server && cargo build --release

client/dist/index.html: client/src/*
	cd client && npx parcel build
