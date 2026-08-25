use std::path::{Path, PathBuf};

use bifrost::{Discovery, Node, Session, Transport};
use bifrost_wire::Transfer;
use clap::Args;
use eyre::WrapErr as _;
use tokio::io::AsyncWriteExt as _;

/// Wait to receive files into a directory, saving each until the sender is done.
#[derive(Debug, Args)]
pub struct RecvCmd {
    /// Directory to write received files into.
    #[arg(short, long, default_value = ".")]
    out: PathBuf,
}

impl RecvCmd {
    pub async fn run<T: Transport, D: Discovery>(self, node: &Node<T, D>) -> eyre::Result<()> {
        println!("iris ready. share this address with the sender:\n");
        println!("    {}\n", node.node_id());
        println!("waiting for a sender...");

        let session = node.accept().await.wrap_err("waiting for a sender")?;
        let peer = session.peer();

        // One stream per file; the loop ends when the sender closes the session.
        let mut count = 0;
        while let Ok((send, recv)) = session.accept_bi().await {
            let temp = self
                .out
                .join(format!(".iris-{}-{count}.part", std::process::id()));
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

            println!(
                "received {name} ({} bytes) from {}",
                received.blob.len(),
                peer.short()
            );
            count += 1;
        }

        if count == 0 {
            eyre::bail!("sender closed without sending a file");
        }
        println!("received {count} file(s), saved to {}", self.out.display());
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
