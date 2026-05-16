# ClipGlimpse

> [中文版说明](README.cn.md)

Transfer text across air-gapped cloud desktops via QR codes on screen.

**Problem**: Cloud desktops block return traffic (cloud → local), but local → cloud works. Traditional clipboard/file transfers are unidirectional. ClipGlimpse bridges this gap using only the visual channel.

**Solution**: Encode text as QR codes on the cloud side, capture and decode from the local screen — fully offline, fully compliant.

## Architecture

```
┌──────────────────────┐         ┌──────────────────────┐
│   Cloud PC            │         │   Local PC            │
│                       │  Screen │                       │
│  Generate mode        │◄────────│  Read mode            │
│  • Text → QR chunks   │  Remote │  • Screen capture     │
│  • Cycle display      │  Desktop│  • QR decode          │
│  • Configurable speed │         │  • Text reassembly    │
└──────────────────────┘         │  • Auto-clipboard     │
                                 │  • System notification│
                                 │  • In-memory history  │
                                 └──────────────────────┘
```

## Quick Start

Double-click `clip_glimpse.exe` (requires `default_mode` in [config.toml](#configuration)),
or specify a subcommand:

### Generate mode (run on Cloud PC)

```bash
clip_glimpse generate
```

A GUI window opens:
1. Paste text into the input area
2. Choose a preset and cycle interval
3. Text changes auto-start cycling QR frames
4. Point the local PC's screen capture at the QR area

### Read mode (run on Local PC)

```bash
clip_glimpse read
```

1. **First run**: Drag to select the QR display region on screen
2. Press **Ctrl+Shift+V** or click **Start Scan** to begin
3. On message completion: auto-copies to clipboard, fires a toast notification, and auto-stops scanning
4. Received messages appear in the **History** tab — select to preview, click **Copy to Clipboard**

## Protocol (v2)

Each QR code carries a structured binary chunk with a 12-byte header:

```
┌────────┬──────────┬─────────┬─────────┬──────────┬──────────────────┐
│  MAGIC │ VERSION  │  FLAGS  │   SEQ   │  TOTAL   │      CRC32      │
│  2 B   │  1 B     │  1 B    │  2 B    │  2 B     │      4 B        │
│  "CG"  │  0x02    │bit0=lz4 │ u16 BE  │ u16 BE   │    u32 BE       │
└────────┴──────────┴─────────┴─────────┴──────────┴──────────────────┘
┌──────────────────────────────────────────────────────────────────────┐
│                       PAYLOAD (N bytes)                              │
│              lz4-compressed chunk of the message data                 │
└──────────────────────────────────────────────────────────────────────┘
```

- All chunks are uniform — no SOS/DATA/EOS distinction
- Every chunk carries the CRC32 of the full uncompressed text
- The reader can start collecting **from any chunk** (cyclic consumption, O(n))
- After collecting `total` unique chunks: concatenate → lz4 decompress → CRC32 verify → deliver
- Max 100 chunks per message

## Presets

| Preset | Version | EC Level | Module Size | Payload/Chunk | Display Size |
|--------|---------|----------|-------------|---------------|--------------|
| Conservative V20-Q | V20 | Q | 3 px | 415 B | ~291×291 |
| **Default V25-M** | **V25** | **M** | **3 px** | **767 B** | **~351×351** |
| Fast V30-M | V30 | M | 2 px | 1031 B | ~274×274 |
| Extreme V35-L | V35 | L | 2 px | 1583 B | ~314×314 |

The payload per chunk excludes the 12-byte protocol header.
All data is transparently compressed with lz4 before chunking.

## Throughput (estimate)

| Text Size | Default V25-M | Fast V30-M | Extreme V35-L |
|-----------|---------------|------------|---------------|
| 1 KB | ~0.6 s | ~0.3 s | ~0.3 s |
| 10 KB | ~4.2 s | ~3.0 s | ~2.1 s |
| 100 KB | ~40 s | ~30 s | ~20 s |
| 1 MB | ~6 min 44 s | ~5 min 2 s | ~3 min 17 s |

Based on 200 ms scan interval. Actual performance varies with screen capture speed and QR code quality.

## Configuration

Config file is `config.toml` in the working directory. See [config.example.toml](config.example.toml) for all options.

Key settings:
- `default_mode` — Default mode when no subcommand: `"generate"` or `"read"` (optional)
- `scan_interval_ms` — Scanner polling interval (default: 200 ms)
- `hotkey` — Hotkey string, e.g. `"Ctrl+Shift+V"` (case-insensitive)
- `hotkey_enabled` — Enable hotkey toggle (default: true)
- `log_enabled` — Write `clip_glimpse.log` (default: true)
- `generate_preset_index` — Default QR preset (default: 1 = V25-M)
- `generate_interval_ms` — Default cycle interval in generate mode (default: 500)

## Features

- **Auto-detach**: First run spawns a child process with `--detached` flag then exits, so the shell prompt returns immediately
- **Hotkey polling**: Toggle scan with configurable hotkey (default `Ctrl+Shift+V`), edge-triggered to avoid repeated toggles
- **Auto-clipboard**: Complete messages are automatically written to the Windows clipboard via `SetClipboardData(CF_UNICODETEXT)`
- **Toast notification**: Uses `Shell_NotifyIconW` to show a system balloon notification on message completion
- **Auto-stop**: Scanning stops automatically after a complete message is received
- **Assembly timeout**: If no new chunk arrives within 30 seconds, the assembler resets and waits for a new message
- **Region reselection**: Click "Change Region" in the scanner panel to re-select the capture area without restarting
- **History**: In-memory message history (max 100 entries), view and copy from the History tab
- **Cycle display**: In generate mode, QR frames cycle at configurable intervals (200/300/500/800/1000 ms)
- **CJK fonts**: Loads SimHei, SimSun, or Microsoft YaHei for Chinese text rendering

## Development

### Requirements

- Rust 1.75+ (2021 edition)
- Windows 10/11 (GDI-based screen capture and hotkey polling)
- MSVC toolchain (`stable-x86_64-pc-windows-msvc`)

### Build

```bash
cargo build --release
```

Or with explicit MSVC toolchain:

```bash
cargo +stable-x86_64-pc-windows-msvc build --release
```

### Test

```bash
cargo test
```

Or:

```bash
cargo +stable-x86_64-pc-windows-msvc test
```

### Module Structure

```
src/
├── main.rs              # Entry point, detach logic
├── cli.rs               # CLI argument parsing (clap)
├── protocol.rs          # Binary chunk encode/decode, CRC32, reassembly
├── qr_gen.rs            # QR code image generation (qrcode crate)
├── qr_read.rs           # QR code decode from pixels (rxing crate)
├── screen.rs            # Windows GDI screen capture (BitBlt, GetDIBits)
├── hotkey.rs            # Global hotkey parsing and polling (GetAsyncKeyState)
├── clipboard.rs         # Win32 clipboard write (SetClipboardData)
├── notify.rs            # Windows toast notification (Shell_NotifyIconW)
├── history.rs           # In-memory message history (max 100)
├── logger.rs            # File-based debug logging (clip_glimpse.log)
├── icon.rs              # Programmatic 16×16 app icon
├── tray.rs              # System tray icon (planned integration)
├── generate/
│   ├── mod.rs           # Generate mode entry point + font setup
│   └── ui.rs            # Generate mode GUI (eframe/egui)
└── read/
    ├── mod.rs           # Read mode entry point + config + hotkey thread
    ├── ui.rs            # Read mode GUI (eframe/egui)
    ├── scanner.rs       # Background scanner thread: capture → decode → assemble → notify
    └── region.rs        # Full-screen region selector overlay
```

### Key Dependencies

| Purpose | Crate |
|---------|-------|
| GUI | `eframe` / `egui` |
| QR encode | `qrcode` |
| QR decode | `rxing` |
| Screen capture | `windows` (GDI) |
| CLI | `clap` |
| Image | `image` |
| Serialization | `serde` + `toml` |
| Time | `chrono` |

## Usage

After build, the binary is at:

```
target/release/clip_glimpse.exe
```

**Generate mode** (on Cloud PC):
```bash
clip_glimpse generate
```

**Read mode** (on Local PC):
```bash
clip_glimpse read
```

## License

MIT
