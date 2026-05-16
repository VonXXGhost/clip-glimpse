# ClipGlimpse — Agent Guide

## Commands
```bash
cargo build --release
cargo run --release -- generate
cargo run --release -- read
cargo test
cargo check
```

Use explicit MSVC toolchain if default isn't set: `cargo +stable-x86_64-pc-windows-msvc build --release`.

## Key Facts

- **Windows-only** (GDI BitBlt, GetAsyncKeyState). Won't compile elsewhere.
- Two subcommands via `clap`: `generate` (encode text→QR, cycle-display) and `read` (screen capture→decode→reassemble). Running with no subcommand falls back to `config.toml` `default_mode` field.
- **Detach quirk** (`main.rs:29-41`): first run spawns child with hidden `--detached` flag then exits. Debugging — pass `--detached` yourself or run in debug mode to keep foreground.
- **No console in release** (`main.rs:1`): `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` — stdout/stderr invisible in release builds.
- Crate mirror in `.cargo/config.toml` (rsproxy.cn behind GFW). Do not remove.
- Config stored in CWD as `./config.toml` (see `config.example.toml`). Read mode re-saves `[region]` after first selection. Both generate and read share the same `Config` struct (`read/mod.rs`).
- CJK fonts loaded from `C:\Windows\Fonts\msyh.ttc` / `simsun.ttc` / `simhei.ttf`. At least one must exist or UI text breaks.
- Protocol v2: `CG` + version(1B 0x02) + flags(1B bit0=lz4) + seq(u16 BE) + total(u16 BE) + CRC32(u32 BE) + lz4-compressed payload. All chunks uniform (no SOS/DATA/EOS). Each chunk carries CRC32 of **full uncompressed text**. Max 100 chunks.
- History is **in-memory only** (max 100 entries, `history.rs`). No persistence.
- Logs to `./clip_glimpse.log` via `log_debug!(tag, ...)` macro (`logger.rs`). Disable with `log_enabled = false` in config. Tests disable logging automatically.
- Scanner thread: capture→decode→reassemble→clipboard (`clipboard.rs`, `SetClipboardData(CF_UNICODETEXT)`)→toast (`notify.rs`, `Shell_NotifyIconW`)→auto-stop. 30s assembly timeout resets the assembler.
- Hotkey uses **polling** (`GetAsyncKeyState` every 50ms in a dedicated thread), NOT `RegisterHotKey`. Edge-triggered to avoid repeated toggles.
- Generate mode auto-starts cycling on any text change (`generate/ui.rs:183-187`).
- Region can be reselected at runtime (click "Change Region") without restart — backed by `needs_reselect` atomic in read's outer loop (`read/mod.rs:222-229`).

## Testing

- All tests in `#[cfg(test)]` blocks. `cargo test` runs them all.
- Key tests: `protocol.rs` roundtrip (encode→decode→assemble, out-of-order, cyclic consumption), `qr_read.rs` roundtrip (no screen needed), `hotkey.rs` parse/normalize.

## Source map (modules not detailed in README)

| File | Purpose |
|------|---------|
| `src/clipboard.rs` | Win32 clipboard write (CF_UNICODETEXT) |
| `src/notify.rs` | Windows toast notification via `Shell_NotifyIconW` |
| `src/icon.rs` | Programmatic 16×16 `egui::IconData` |
| `src/tray.rs` | System tray icon (compiles, not wired into any mode) |
| `build.rs` | Generates `.ico` file + embeds via `winresource` |
