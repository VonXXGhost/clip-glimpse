# ClipGlimpse — Agent Guide

## Commands
```bash
# Build (release)
cargo build --release

# Run (MSVC required on Windows)
cargo +stable-x86_64-pc-windows-msvc run --release -- generate
cargo +stable-x86_64-pc-windows-msvc run --release -- read

# Test all
cargo test

# Typecheck (Rust standard)
cargo check
```

## Key Facts

- **Windows-only** (GDI BitBlt screen capture, GetAsyncKeyState polling for hotkey). Will not compile on Linux/macOS.
- Single binary, two subcommands via `clap`: `generate` (encode text→QR, cycle-display) and `read` (screen capture→decode→reassemble).
- Auto-detaches from console on first run: spawns itself with `--detached` flag (see `main.rs:18-33`). Be aware of this when debugging.
- Crate mirror configured in `.cargo/config.toml` (USTC — needed behind Chinese firewall). Do not remove.
- Config stored in CWD as `./config.toml` (not `%APPDATA%`). Region + scan interval + hotkey toggle.
- CJK fonts loaded at startup from `C:\Windows\Fonts\msyh.ttc` / `simsun.ttc` / `simhei.ttf`. At least one must exist.
- Protocol: binary header `CG` + type(1B) + version(1B) + seq(u16 BE) + total(u16 BE) + payload. CRC32 on SOS/EOS. Max 100 chunks (`MAX_CHUNKS = 100`).
- History is **in-memory only**. No persistence across restarts.
- Logs written to `./clip_glimpse.log` via `log_debug!(tag, ...)` macro (defined in `logger.rs`). Useful when debugging the scanner thread.

## Testing

- All tests live in `#[cfg(test)]` blocks within source files. No separate test directory.
- The `protocol.rs` roundtrip tests are the most important. Run `cargo test` to verify encode→decode→assembly.
- QR roundtrip test in `qr_read.rs` generates, renders, and decodes a QR code — requires no real screen.

## Structure

| Path | Purpose |
|------|---------|
| `src/main.rs` | Entrypoint: parse CLI, optionally re-spawn detached |
| `src/cli.rs` | `generate` / `read` subcommand definitions |
| `src/protocol.rs` | Chunk encode/decode, `MessageAssembler`, CRC32 |
| `src/qr_gen.rs` | QR bitmap generation via `qrcode` crate |
| `src/qr_read.rs` | QR decode via `rxing`, BGRA→gray conversion |
| `src/screen.rs` | Windows GDI screen capture (BitBlt + GetDIBits) |
| `src/hotkey.rs` | `GetAsyncKeyState` polling for Ctrl+Shift+V |
| `src/history.rs` | In-memory message history (ring buffer, max 100) |
| `src/logger.rs` | File logger, `log_debug!` macro, writes `./clip_glimpse.log` |
| `src/tray.rs` | System tray icon (read mode only) |
| `src/generate/` | Generate mode UI (eframe window) |
| `src/read/` | Read mode UI, scanner thread, region selection overlay |
