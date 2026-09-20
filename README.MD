# Capynes

Capynes is a small NES emulator written in Rust. It started as a hands-on way to learn how the NES works from the CPU bus upward, and the current version is focused on the first straightforward runtime approach rather than a cycle-perfect rewrite.

What is here:

- A 6502 CPU core with official and unofficial opcode coverage.
- PPU, APU, controller, MMIO, and mapper code wired into a playable desktop runtime.
- Mapper support for the boards used by the bundled test suite and the games used during local development.
- A WGPU/Winit window, keyboard input, and CPAL audio output.
- A fairly large automated conformance suite built around community NES test ROMs.
- A tiny `snake` binary used as a local experiment/example.

What is not here:

- Commercial game ROMs. Bring your own legally obtained `.nes` file when you want to play something.
- A polished end-user emulator UI. This is still very much an engineering project.

## Building

You need a recent Rust toolchain. The project uses the 2024 edition, so `rustup update stable` is a good idea if your compiler is old.

```sh
cargo build
```

For a release build:

```sh
cargo build --release
```

## Running

Run the emulator with an explicit ROM path:

```sh
cargo run -- path/to/game.nes
```

If no ROM is passed, the binary looks for `./test_rom/smb.nes`. That path is ignored by git and is only meant as a local convenience.

Controls:

- D-pad: `W`, `A`, `S`, `D`
- A / B: `J`, `K`
- Select / Start: `T`, `Y`
- Quit: `Esc`

You can also print the short CLI help:

```sh
cargo run -- --help
```

## Tests

Run the full suite with:

```sh
cargo test
```

The tests use NES validation ROMs under `tests/test_roms`. Most of them are community emulator-test programs by authors such as Shay Green / blargg, Damian Yerrick / tepples, kevtris, Quietust, bisqwit, Sour, and others. The collection follows the common NESdev emulator-test set, and many files line up with the public `christopherpow/nes-test-roms` archive.

Legal note: these are test and homebrew ROMs, not commercial game dumps. Several included readmes have permissive redistribution language. A few older test ROMs do not carry a clear license in this tree, so treat them as third-party test artifacts rather than code I authored. Commercial NES game ROMs are not included, and publishing downloaded commercial ROM sets is not OK.
