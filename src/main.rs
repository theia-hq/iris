//! iris: send files to a machine by its public key, verified end to end.
//!
//! `iris recv` prints this machine's address and waits; `iris send <address> <path>...` dials that
//! address and sends the given files or directories. You address a peer by who it is (an ed25519 public
//! key), not where it is, and iris reaches it wherever it is on the internet, across NATs.
//!
//! Integrity is checked end to end: the sender hashes each file with BLAKE3 and the receiver re-hashes
//! as bytes arrive, saving a file only if the hashes match, so a truncated or tampered transfer is
//! rejected rather than written. Files stream in fixed-size chunks; a large transfer is never held whole
//! in memory. This machine's address is a persisted key (`--key` / `IRIS_KEY` or
//! `~/.config/iris/identity.key`), so it stays the same across runs.

use std::path::PathBuf;

use bifrost::{NoDiscovery, Node};
use clap::{Parser, Subcommand};
use iris::identity;
use iris::recv::RecvCmd;
use iris::send::SendCmd;

/// Send files to a machine by its public key, verified end to end.
#[derive(Debug, Parser)]
#[command(name = "iris", version, about)]
struct Cli {
    /// pin a persisted identity file [env: IRIS_KEY]
    #[arg(long = "key", env = "IRIS_KEY", global = true)]
    key: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Wait to receive files. Prints this node's address to share with a sender.
    Recv(RecvCmd),
    /// Send files to a peer's address.
    Send(SendCmd),
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // The one and only place a concrete transport is named. Everything downstream speaks `bifrost`.
    // iroh self-discovers (n0 pkarr/DNS + relays), so it composes with NoDiscovery. The identity is
    // persisted so this node keeps the same address across runs.
    let secret = identity::load_or_create(cli.key.as_deref()).await?;
    let node = Node::new(
        bifrost_iroh::Endpoint::bind_with_secret(secret).await?,
        NoDiscovery,
    );

    match cli.command {
        Command::Recv(cmd) => cmd.run(&node).await,
        Command::Send(cmd) => cmd.run(&node).await,
    }
}
