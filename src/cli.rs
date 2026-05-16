use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "clip_glimpse")]
#[command(about = "Transfer text across air-gapped cloud desktops via QR codes")]
#[command(
    long_about = "ClipGlimpse transfers clipboard text between air-gapped cloud desktops \
using QR codes displayed on screen and captured via screen capture.\n\
\n\
USAGE:\n\
    clip_glimpse [COMMAND]\n\
    clip_glimpse generate    — Open GUI to type/paste text, encode as QR, cycle-display\n\
    clip_glimpse read        — Open GUI to scan QR region, decode & reassemble messages\n\
\n\
When no subcommand is given, the default mode is read from config.toml (field: default_mode).\n\
\n\
CONFIG:\n\
    config.toml in CWD controls:\n\
      - default_mode        : Default mode when no subcommand (\"generate\" / \"read\")\n\
      - scan_interval_ms    : Scanner polling interval (default: 200)\n\
      - hotkey_enabled      : Enable hotkey toggle (default: true)\n\
      - hotkey              : Hotkey string, e.g. \"Ctrl+Shift+V\" (default: \"Ctrl+Shift+V\")\n\
      - log_enabled         : Write clip_glimpse.log (default: true)\n\
      - generate_preset_index : Default QR preset index in generate mode (default: 1)\n\
      - generate_interval_ms  : Default cycle interval in generate mode (default: 500)\n\
\n\
HOTKEYS:\n\
    configurable via hotkey string in config.toml (default: Ctrl+Shift+V)" )]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Internal flag to avoid re-spawn loop
    #[arg(long, global = true, hide = true)]
    pub detached: bool,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Generate mode: encode text as QR codes and cycle-display them")]
    Generate,
    #[command(about = "Read mode: scan QR codes from screen region, decode and reassemble text")]
    Read,
}
