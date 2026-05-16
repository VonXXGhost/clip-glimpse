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
        cli::Command::Generate => {
            generate::run()?;
        }
        cli::Command::Read => {
            read::run()?;
        }
    }

    Ok(())
}
