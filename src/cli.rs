use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "clip_glimpse")]
#[command(about = "Transfer text across air-gapped cloud desktops via QR codes", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

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
