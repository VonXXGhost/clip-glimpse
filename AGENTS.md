# ClipGlimpse — Agent Guide

## Commands
```bash
cargo build --release
cargo +stable-x86_64-pc-windows-msvc run --release -- generate
cargo +stable-x86_64-pc-windows-msvc run --release -- read
cargo test
cargo check
```

## Key Facts

- **Windows-only** (GDI BitBlt, GetAsyncKeyState, RegisterHotKey). Won't compile elsewhere.
- Two subcommands via `clap`: `generate` (encode text→QR, cycle-display) and `read` (screen capture→decode→reassemble).
- **Detach quirk**: first run spawns child with hidden `--detached` flag then exits (see `main.rs:26-38`). The child inherits the console and won't re-spawn. Important when debugging — you may need to pass `--detached` yourself or run via MSVC toolchain to keep foreground.
- Crate mirror in `.cargo/config.toml` (USTC behind GFW). Do not remove.
- Config stored in CWD as `./config.toml` (see `config.example.toml` for all fields). Read mode re-saves region after first selection.
- CJK fonts loaded from `C:\Windows\Fonts\msyh.ttc` / `simsun.ttc` / `simhei.ttf`. At least one must exist or UI text breaks.
- Protocol: `CG` + type(1B SOS/DATA/EOS) + version(1B 0x01) + seq(u16 BE) + total(u16 BE) + payload. CRC32 on SOS/EOS. Max 100 chunks.
- History is **in-memory only** (max 100 entries). No persistence.
- Logs to `./clip_glimpse.log` via `log_debug!(tag, ...)` macro (`logger.rs`). Disable with `log_enabled = false` in config.
- Scanner thread (**behavior to know**): on message completion, it auto-copies to clipboard via `clipboard.rs` (Win32 `SetClipboardData`), fires a Windows toast notification via `notify.rs` (shell notify icon), and auto-stops scanning.
- Generate mode auto-starts cycling on any text change.
- App icon generated programmatically in `icon.rs` (16×16 QR-like pattern).

## Testing

- All tests in `#[cfg(test)]` blocks in source files. `cargo test` runs them all.
- Key tests: `protocol.rs` roundtrip, `qr_read.rs` roundtrip (no screen needed), `hotkey.rs` parse/normalize.

## Source map (modules not listed in README)

| File | Purpose |
|------|---------|
| `src/clipboard.rs` | Win32 clipboard write (CF_UNICODETEXT) |
| `src/notify.rs` | Windows toast notification via `Shell_NotifyIconW` |
| `src/icon.rs` | Programmatic 16×16 `egui::IconData` |
