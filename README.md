# rmemdump

`/dev/mem`-based memory dumper for Linux systems.
Similar to [memdump](https://github.com/tchebb/memdump), but in Rust and a bit fancier.

## Features

* Easy to build, all C dependencies vendored
* Reasonably small (ca. 700KiB release binary for armv7 Android)
* Fancy progress output
* Optional compression of dump (much faster dumping on some targets!)
* Reads in chunks; no memory exhaustion when reading big regions

## Usage

Run the binary for up-to-date help output.

## Compilation

Run `cargo build --release`. If cross-compiling for Android, I recommend [cargo-ndk](https://github.com/bbqsrc/cargo-ndk).
