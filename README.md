# repl-rs

_A pseudo-repl for Rust_

## Requirements

To build repl-rs, one needs to have a somewhat new Rust version installed (`v1.27`), and all the dependencies listed for [yew](https://github.com/DenisKolodin/yew#user-content-development-setup).

## Running

Building the server as a static binary does not seem to work at the moment. Help would be much appreciated.

The repl consists of two moving parts, one is the server/kernel located in `server/`, and the other is the interface located in `client/`.

One needs to run both the server and the frontend when developing, so:

```bash
# In terminal one
$ make server-dev

# In terminal two
$ make client-dev
```

The frontend is hot reloaded, so no need to restart that, just refresh, but the server needs manual restarts when changes are made, simply press `CTRL+C` and run `make server-dev` again.

If the port was not already taken, the interface can be found in your browser at [http://[::1]:8000](http://[::1]:8000), and the server at [localhost:8080](http://localhost:8080). Accessing the server directly is only ever useful when running the server packaged.
