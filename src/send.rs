use std::path::PathBuf;

use bifrost::{Discovery, Node, NodeId, Session, Transport};
use bifrost_wire::{Blob, Transfer};
use clap::Args;

/// Send a file to a peer, addressed by their node id.
#[derive(Debug, Args)]
pub struct SendCmd {
    /// The file to send.
    path: PathBuf,
    /// The recipient's node id, as printed by `iris recv`.
    peer: NodeId,
}

impl SendCmd {
    pub async fn run<T: Transport, D: Discovery>(self, node: &Node<T, D>) -> eyre::Result<()> {
        let Self { path, peer } = self;
        let name = path
            .file_name()
            .and_then(|component| component.to_str())
            .ok_or_else(|| eyre::eyre!("path has no file name"))?;

        // Files are the app's concern: hash it, then stream it. The wire only sees bytes.
        let blob = {
            let mut file = tokio::fs::File::open(&path).await?;
            Blob::hash(&mut file).await?
        };

        println!("connecting to {}...", peer.short());
        let session = node.connect(peer).await?;
        let (send, recv) = session.open_bi().await?;

        let mut source = tokio::fs::File::open(&path).await?;
        Transfer::new(send, recv)
            .send(name.as_bytes(), &blob, &mut source)
            .await?;
        node.close().await;

        println!(
            "sent {name} ({} bytes), verified by {}",
            blob.len(),
            peer.short()
        );
        Ok(())
    }
}
