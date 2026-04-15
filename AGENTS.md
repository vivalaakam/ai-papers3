# Repository Guidelines

## Project Structure & Module Organization

Embedded Rust application for the M5Stack PaperS3 (ESP32-S3, 960×540 e-ink display). The project has two independent workspaces:

- **Root workspace** (`Cargo.toml`) — the embedded app, targeting `xtensa-esp32s3-espidf`. Binary entry point is `src/bin/ai-papers3.rs`.
- **`iocraft/`** — a standalone Rust workspace (separate `Cargo.toml`): a React-inspired terminal UI library with its own CI. Not part of the embedded build.

Key modules in `src/`:
- `display.rs` — e-ink display driver and rendering (960×540, SPI-based via `components/papers3_display/`)
- `image.rs` — BMP image loading
- `wifi.rs` — WiFi connection via esp-idf-svc
- `touch.rs` — GT911 capacitive touch controller
- `config.rs` / `file.rs` — YAML config and SD card I/O

The custom ESP-IDF C component (`components/papers3_display/`) wraps the `epdiy` e-paper driver. Font headers in that component are pre-generated; use `tools/font_to_c_font.py` to regenerate them from `assets/fonts/`.

## Build, Test, and Development Commands

Source the ESP environment before any build command:
```bash
source $HOME/export-esp.sh
```

| Task | Command |
|------|---------|
| Dev build | `cargo build` |
| Release build | `cargo build --release` |
| Flash + monitor | `cargo run --release` |
| Flash separately | `espflash flash target/xtensa-esp32s3-espidf/release/papers3-rust --monitor` |
| Format check | `cargo fmt --all -- --check` |
| Auto-format | `cargo fmt` |
| Lint (strict) | `cargo clippy --all-targets --all-features --workspace -- -D warnings` |

To enter download mode on the device: long-press the power button until the red LED blinks.

The iocraft subproject has its own commands (run from `iocraft/`):
```bash
cargo make checks   # format + build + test + clippy + doc
cargo test
```

## Coding Style & Naming Conventions

Standard Rust conventions enforced by `cargo fmt` and `cargo clippy -- -D warnings`:

- Modules and functions: `snake_case`
- Structs and enums: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Zero compiler warnings — all clippy warnings are treated as errors.

Use `Result` for all fallible operations. Define custom error types per domain (see `src/enums.rs`).

**FreeRTOS constraint:** Never use busy loops. Always yield to the scheduler:
```rust
// BAD
for _ in 0..1_000_000 { spin_loop_hint(); }
// GOOD
delay::FreeRtos::delay_ms(100);
```

## Testing Guidelines

There is no automated test suite for the embedded binary — hardware integration tests require flashing to the physical device and monitoring serial output:
```bash
cargo run --release          # flash and observe logs
espflash monitor /dev/ttyUSB0
```

Use `log::{info, warn, error, debug}` macros for serial debug output.

## Commit & Pull Request Guidelines

Conventional commit format (defined in `dev-guide.md`):
```
<type>: <subject>
```
Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`.

Branches: `main` (stable, flashable), `feature/*`, `fix/*`.
