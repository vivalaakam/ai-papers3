# Development Guide - papers3-rust

This guide provides practical development workflows and best practices for the PaperS3 Rust project.

## Quick Start Checklist

- [ ] ESP toolchain installed (`espup` + `export-esp.sh`)
- [ ] Environment sourced (`source $HOME/export-esp.sh`)
- [ ] Device connected via USB (data cable!)
- [ ] Project builds (`cargo build --release`)
- [ ] Can flash to device (`cargo run --release`)

## Daily Development Workflow

### 1. Start Session
```bash
# Source ESP environment
source $HOME/export-esp.sh

# Verify toolchain
cargo --version
rustc --version
```

### 2. Make Changes
```bash
# Edit code with your editor
# ...

# Check formatting
cargo fmt --all -- --check --color always

# Auto-format if needed
cargo fmt
```

### 3. Build & Test
```bash
# Development build (faster)
cargo build

# Release build (for flashing)
cargo build --release

# Run clippy for linting
cargo clippy --all-targets --all-features --workspace -- -D warnings
```

### 4. Flash & Test
```bash
# Enter download mode: Long-press power button until red LED blinks

# Flash and run with monitor
cargo run --release

# Or flash separately
espflash flash target/xtensa-esp32s3-espidf/release/papers3-rust --monitor
```

## Coding Standards

### Rust Style
- Follow standard Rust formatting (`cargo fmt`)
- Use `cargo clippy` for additional linting
- No compiler warnings allowed (`-D warnings`)

### Naming Conventions
```rust
// Modules: snake_case
mod display_manager;
mod wifi_handler;

// Structs: PascalCase
struct TodoItem {
    id: String,
    title: String,
}

// Functions: snake_case
fn fetch_todos() -> Result<Vec<Todo>> {
    // ...
}

// Constants: SCREAMING_SNAKE_CASE
const WIFI_SSID: &str = "your_network";
const MAX_TODOS: usize = 100;
```

### Error Handling
```rust
// Use Result for fallible operations
pub fn connect_wifi() -> Result<(), WifiError> {
    // ...
}

// Define custom error types
#[derive(Debug)]
pub enum WifiError {
    NotConfigured,
    ConnectionFailed,
    Timeout,
}
```

### FreeRTOS Constraints
```rust
// NEVER use busy loops - ALWAYS use FreeRtos::delay_ms()
use esp_idf_svc::hal::delay;

// BAD - busy waits CPU
// for _ in 0..1_000_000 { spin_loop_hint(); }

// GOOD - yields to FreeRTOS scheduler
delay::FreeRtos::delay_ms(100);

// For async operations, use appropriate blocking with timeout
```

## Project Structure Best Practices

### Adding a New Module

1. **Create module file** (e.g., `src/display/mod.rs`)
2. **Add submodule declarations** in `src/main.rs`:
   ```rust
   mod display;
   ```
3. **Export public items** from module's `mod.rs`:
   ```rust
   pub use self::renderer::Renderer;
   ```
4. **Update AGENTS.md** if architecture changes

### Module Template
```rust
//! # Module Name
//!
//! Brief description of what this module does.

use anyhow::Result;

pub struct MyComponent {
    // Fields
}

impl MyComponent {
    /// Creates a new instance
    pub fn new() -> Result<Self> {
        // Initialization
        Ok(Self {})
    }

    /// Does something useful
    pub fn do_work(&mut self) -> Result<()> {
        // Implementation
        Ok(())
    }
}
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        let result = something();
        assert_eq!(result, expected);
    }
}
```

### Integration Testing (Device Required)
```bash
# Flash test build to device
cargo run --release

# Monitor serial output for test results
espflash monitor /dev/ttyUSB0
```

## Debugging Tips

### Serial Output
```rust
use log::{info, warn, error, debug};

// Different log levels
info!("Connecting to WiFi...");
warn!("Connection timeout, retrying...");
error!("Failed to initialize display");
debug!("Raw data: {:?}", data);
```

### Common Issues & Solutions

| Issue | Cause | Solution |
|-------|-------|----------|
| `cargo: ESP_IDF not defined` | Environment not sourced | Run `source $HOME/export-esp.sh` |
| Build hangs at `Compiling...` | Low memory or toolchain issue | `cargo clean`, check free RAM |
| Flash fails "no device found" | Wrong USB port or cable | Check `/dev/ttyUSB*`, use data cable |
| Device doesn't boot | WDT timeout | Check for busy loops, add delays |
| Display garbled | Wrong refresh timing | Add delays between refresh commands |

### Memory Profiling
```rust
use esp_idf_svc::hal::heap;

// Check available heap
let free_heap = heap::caps_get_free_heap_size(heap::HeapCaps::Internal);
let psram_heap = heap::caps_get_free_heap_size(heap::HeapCaps::SpiRam);

info!("Free internal RAM: {} KB", free_heap / 1024);
info!("Free PSRAM: {} KB", psram_heap / 1024);
```

## Git Workflow

### Commit Message Format
```
<type>: <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code refactoring
- `docs`: Documentation changes
- `test`: Test additions/changes
- `chore`: Build/config changes

**Example:**
```
feat: add basic e-ink display driver

Implement EPD driver for 960x540 display with:
- Full refresh support
- Basic drawing primitives
- Framebuffer management

Closes #3
```

### Branch Strategy
- `main`: Stable, flashable code
- `feature/*`: Feature development
- `fix/*`: Bug fixes

## Hardware Safety

### Before Flashing
- Double-check GPIO assignments
- Verify power supply capacity (PaperS3 needs ~500mA during refresh)
- Don't disconnect USB during flash

### E-ink Display Care
- Minimize full refreshes (wear ~100k cycles)
- Don't leave display partially refreshed
- Add delay between refreshes

## Performance Optimization

### Build Size
```bash
# Check binary size
ls -lh target/xtensa-esp32s3-espidf/release/papers3-rust

# Optimize for size (add to Cargo.toml)
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization
```

### Boot Time
- Lazy initialize non-critical components
- Defer WiFi connection until needed
- Use PSRAM for large buffers

## Next Steps

1. **Hardware Verification**: Create a simple "blink" or display test
2. **Mock Data**: Build UI with hardcoded todos
3. **WiFi Integration**: Connect to network
4. **API Client**: Fetch real data
5. **Persistence**: Add NVS storage

## Resources

- **Project Docs**: See `AGENTS.md` for architecture
- **ESP Rust Book**: https://docs.espressif.com/projects/rust/book/
- **esp-idf-svc Docs**: https://docs.rs/crate/esp-idf-svc/latest
- **PaperS3 Hardware**: https://docs.m5stack.com/en/core/papers3
