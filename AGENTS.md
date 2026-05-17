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
- Generate mode auto-starts cycling on any text change when >1 frame; single-frame text shows static QR (`generate/ui.rs` `sync_display_state`).
- Region can be reselected at runtime (click "Change Region") without restart — backed by `needs_reselect` atomic in read's outer loop (`read/mod.rs:222-229`).

## Testing

- All tests in `#[cfg(test)]` blocks. `cargo test` runs them all.
- Key tests: `protocol.rs` roundtrip (encode→decode→assemble, out-of-order, cyclic consumption), `qr_read.rs` roundtrip (no screen needed), `hotkey.rs` parse/normalize.

## Color QR Mode

- **Color mode** (`color_mode` in config, toggleable via Generate UI checkbox): each display frame composites **3 independent B&W QR codes** into one color image via R/G/B channels.
- **Color scheme**: only extreme values (0 or 255) per channel → 8 distinct colors (RGB cube corners). Maximal separation, robust against compression/capture artifacts.
- **Generation** (`generate_color_qr` in `qr_gen.rs`): calls `QrCode::with_version` for each of 1-3 chunks, then composites by setting channel R/G/B to 0 if the corresponding QR module is Dark, else 255. Empty channels filled with white.
- **Reading** (`extract_channel_from_bgra` in `qr_read.rs`): extracts single channel (R, G, or B) from BGRA capture as grayscale data; applies `stretch_contrast` to recover full dynamic range, then feeds each to `rxing::decode_qr` independently.
- **Bandwidth**: ~3x improvement over B&W mode per frame-cycle. E.g. V25-M goes from 767 B/frame → 2301 B/frame.
- **Compatibility**: B&W mode is preserved as default. Color mode only activates when explicitly enabled via config or GUI toggle.

## Source map (modules not detailed in README)

| File | Purpose |
|------|---------|
| `src/clipboard.rs` | Win32 clipboard write (CF_UNICODETEXT) |
| `src/notify.rs` | Windows toast notification via `Shell_NotifyIconW` |
| `src/icon.rs` | Programmatic 16×16 `egui::IconData` |
| `src/tray.rs` | System tray icon (compiles, not wired into any mode) |
| `build.rs` | Generates `.ico` file + embeds via `winresource` |
