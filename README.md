# repl-rs

_A bad pseudo-repl for Rust 💩_

## Requirements

To build repl-rs, one needs to have a somewhat new Rust version installed (`v1.27`), and a somewhat new Node.js version installed (`v9.11`).

## Running

**Bad TL:DR:** One can build the project using `make build`, and then have a binary ready at `bin/repl-rs`. This does not work at the moment, due to some linking errors I don't understand, help would be much appreciated 😅.

The repl consists of two moving parts, one is the server/kernel located in `server/`, and the other is the interface written in TypeScript located in `client/`.

One needs to run both the server and the frontend when developing, so:

```bash
# In terminal one
$ cd server && cargo run

# in terminal two
$ cd client && npm start
```

The frontend is hot reloaded, so no need to restart that, but the server needs manual restarts when changes are made, simply press `CTRL+C` and run `cargo run` again.

If the port was not already taken, the interface can be found in your browser at [localhost:1234](http://localhost:1234), and the server at [localhost:8080](http://localhost:8080). Accessing the server directly is only ever useful when running the server packaged.
