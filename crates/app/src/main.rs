//! ShareClick — a low-latency, open-source software KVM.
//!
//! Move one keyboard & mouse (and the clipboard, and files) between machines
//! over the LAN with the lowest possible input lag.

mod bench;
mod transport;

#[cfg(feature = "native")]
mod capture;
#[cfg(feature = "native")]
mod emit;
#[cfg(feature = "native")]
mod keymap;
#[cfg(feature = "native")]
mod run;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "shareclick", version, about = "Low-latency open-source software KVM")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Measure input-channel round-trip latency over loopback.
    Bench {
        /// Number of ping/pong round trips to measure.
        #[arg(short, long, default_value_t = 20_000)]
        count: usize,
    },
    /// Run as the server (the machine whose keyboard & mouse are shared).
    Serve {
        /// Address to bind the input channel to.
        #[arg(long, default_value = "0.0.0.0:24800")]
        bind: String,
    },
    /// Connect to a server as a client (receives input, injects it locally).
    Connect {
        /// Server address, e.g. 192.168.1.20:24800
        server: String,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Bench { count } => bench::run(count),
        #[cfg(feature = "native")]
        Command::Serve { bind } => run::serve(&bind),
        #[cfg(feature = "native")]
        Command::Connect { server } => run::connect(&server),
        #[cfg(not(feature = "native"))]
        Command::Serve { .. } | Command::Connect { .. } => {
            anyhow::bail!("serve/connect require the `native` feature (build without --no-default-features)")
        }
    }
}
