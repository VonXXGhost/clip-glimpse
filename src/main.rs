#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
mod logger;
mod protocol;
mod qr_gen;
mod qr_read;
mod screen;
mod hotkey;
mod history;
mod tray;
mod generate;
mod read;
mod cli;
mod clipboard;
mod notify;
mod icon;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();

    // Spawn a child process with the same args and exit immediately,
    // so PowerShell/cmd gets its prompt back. The child inherits the
    // console (fine for a GUI app) and will not re-spawn because
    // of the --detached flag.
    #[cfg(windows)]
    if !cli.detached {
        if let Ok(exe) = std::env::current_exe() {
            let args: Vec<String> = std::env::args().skip(1).collect();
            let child = std::process::Command::new(&exe)
                .args(&args)
                .arg("--detached")
                .spawn();
            if child.is_ok() {
                return Ok(());
            }
        }
        // fall through to run in foreground if spawning fails
    }

    match cli.command {
        Some(cli::Command::Generate) => {
            generate::run()?;
        }
        Some(cli::Command::Read) => {
            read::run()?;
        }
        None => {
            let config = crate::read::Config::load();
            match config.default_mode.as_deref() {
                Some("generate") => generate::run()?,
                Some("read") => read::run()?,
                Some(other) => {
                    eprintln!("Unknown default_mode '{other}' in config.toml. Use 'generate' or 'read'.");
                    std::process::exit(1);
                }
                None => {
                    eprintln!("No subcommand specified. Use 'clip_glimpse generate' or 'clip_glimpse read'.");
                    eprintln!("Set 'default_mode' in config.toml to launch a mode automatically.");
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
