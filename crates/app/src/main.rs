//! ShareClick — a low-latency, open-source software KVM.
//!
//! Move one keyboard & mouse (and the clipboard, and files) between machines
//! over the LAN with the lowest possible input lag.

mod bench;
mod transport;

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
        Command::Serve { bind } => {
            tracing::info!(%bind, "serve mode not yet implemented (transport ready)");
            Ok(())
        }
        Command::Connect { server } => {
            tracing::info!(%server, "connect mode not yet implemented (transport ready)");
            Ok(())
        }
    }
}
