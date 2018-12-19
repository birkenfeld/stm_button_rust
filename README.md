## Make sound with STM32L0

### Build

Using nightly Rust and cargo, just do a `cargo build --release`.

### Flash

The default method uses openocd and GDB.  Start openocd using a config
matching your programming adapter (the provided `openocd.cfg` assumes
ST-Link v2).  Then `cargo run --release` runs GDB, flashes and runs the
image.  `openocd` should just keep running in the background.

An alternate way is to use the `st-flash` utility.  To use this from
`cargo run --release`, change the runner in `.cargo/config`.
