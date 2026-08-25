//! Iris: a verifiable file courier over the Bifrost overlay.
//!
//! `iris recv` prints this node's address and waits; `iris send <path> <address>` dials that identity
//! and streams a file, verified end to end. You send to who someone is, not where they are.

use bifrost::{NoDiscovery, Node};
use clap::{Parser, Subcommand};

mod recv;
mod send;

use recv::RecvCmd;
use send::SendCmd;

/// Send files to a peer, addressed by their public key, verified end to end.
#[derive(Debug, Parser)]
#[command(name = "iris", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Wait to receive a file. Prints this node's address to share with a sender.
    Recv(RecvCmd),
    /// Send a file to a peer's address.
    Send(SendCmd),
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // The one and only place a concrete transport is named. Everything downstream speaks `bifrost`.
    // iroh self-discovers (n0 pkarr/DNS + relays), so it composes with NoDiscovery.
    let node = Node::new(bifrost_iroh::Endpoint::bind().await?, NoDiscovery);

    match cli.command {
        Command::Recv(cmd) => cmd.run(&node).await,
        Command::Send(cmd) => cmd.run(&node).await,
    }
}
