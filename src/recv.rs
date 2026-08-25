use std::path::{Path, PathBuf};

use bifrost::{Discovery, Node, Session, Transport};
use bifrost_wire::Transfer;
use clap::Args;
use eyre::WrapErr as _;
use tokio::io::AsyncWriteExt as _;

/// Wait to receive a single file, then exit.
#[derive(Debug, Args)]
pub struct RecvCmd {
    /// Directory to write the received file into.
    #[arg(short, long, default_value = ".")]
    out: PathBuf,
}

impl RecvCmd {
    pub async fn run<T: Transport, D: Discovery>(self, node: &Node<T, D>) -> eyre::Result<()> {
        println!("iris ready. share this address with the sender:\n");
        println!("    {}\n", node.node_id());
        println!("waiting for a sender...");

        let session = node.accept().await?;
        let peer = session.peer();
        let (send, recv) = session.accept_bi().await?;

        // The wire streams verified bytes into a sink; choosing the sink (a temp file) and naming the
        // final file are the app's concerns, not the wire's.
        let temp = self.out.join(format!(".iris-{}.part", std::process::id()));
        let received = {
            let mut sink = tokio::fs::File::create(&temp).await?;
            match Transfer::new(send, recv).recv(&mut sink).await {
                Ok(received) => {
                    sink.flush().await?;
                    received
                }
                Err(err) => {
                    drop(sink);
                    let _ = tokio::fs::remove_file(&temp).await;
                    return Err(err.into());
                }
            }
        };

        let name = safe_file_name(&received.header);
        let final_path = self.out.join(&name);
        tokio::fs::rename(&temp, &final_path)
            .await
            .wrap_err_with(|| format!("save to {}", final_path.display()))?;
        session.wait_closed().await;

        println!(
            "received {name} ({} bytes) from {}",
            received.blob.len(),
            peer.short()
        );
        println!("saved to {}", final_path.display());
        Ok(())
    }
}

/// Reduce a peer-supplied header to a safe bare file name, so a peer cannot write outside `out`.
fn safe_file_name(header: &[u8]) -> String {
    let raw = String::from_utf8_lossy(header);
    Path::new(raw.as_ref())
        .file_name()
        .and_then(|component| component.to_str())
        .filter(|component| !component.is_empty())
        .unwrap_or("download")
        .to_owned()
}
