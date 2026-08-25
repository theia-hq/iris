use std::path::PathBuf;

use bifrost::{Discovery, Node, NodeId, Session, Transport};
use bifrost_wire::{Blob, Transfer};
use clap::Args;
use eyre::WrapErr as _;

/// Send one or more files to a peer, addressed by their node id.
#[derive(Debug, Args)]
pub struct SendCmd {
    /// The recipient's node id, as printed by `iris recv`.
    peer: NodeId,
    /// The files to send.
    #[arg(required = true)]
    paths: Vec<PathBuf>,
}

impl SendCmd {
    pub async fn run<T: Transport, D: Discovery>(self, node: &Node<T, D>) -> eyre::Result<()> {
        println!("connecting to {}...", self.peer.short());
        let session = node
            .connect(self.peer)
            .await
            .wrap_err_with(|| format!("could not reach {}", self.peer.short()))?;

        // One stream per file; files are the app's concern, the wire only sees bytes.
        for path in &self.paths {
            let name = path
                .file_name()
                .and_then(|component| component.to_str())
                .ok_or_else(|| eyre::eyre!("path has no file name: {}", path.display()))?;

            let blob = {
                let mut file = tokio::fs::File::open(path)
                    .await
                    .wrap_err_with(|| format!("open {}", path.display()))?;
                Blob::hash(&mut file).await?
            };

            let (send, recv) = session.open_bi().await?;
            let mut source = tokio::fs::File::open(path)
                .await
                .wrap_err_with(|| format!("open {}", path.display()))?;
            Transfer::new(send, recv)
                .send(name.as_bytes(), &blob, &mut source)
                .await?;

            println!("sent {name} ({} bytes)", blob.len());
        }

        node.close().await;
        Ok(())
    }
}
