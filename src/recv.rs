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
    pub out: PathBuf,
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
            let spinner = crate::progress::spinner();
            let received = {
                let file = tokio::fs::File::create(&temp).await?;
                let mut sink = crate::progress::ProgressWriter::new(file, spinner.clone());
                match Transfer::new(send, recv).recv(&mut sink).await {
                    Ok(received) => {
                        sink.flush().await?;
                        spinner.finish_and_clear();
                        received
                    }
                    Err(err) => {
                        spinner.finish_and_clear();
                        drop(sink);
                        let _ = tokio::fs::remove_file(&temp).await;
                        return Err(err.into());
                    }
                }
            };

            let relative = safe_relative_path(&received.header);
            let final_path = self.out.join(&relative);
            if let Some(parent) = final_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::rename(&temp, &final_path)
                .await
                .wrap_err_with(|| format!("save to {}", final_path.display()))?;

            println!(
                "received {} ({} bytes) from {}",
                relative.display(),
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

/// Reduce a peer-supplied header to a safe relative path under `out`: keep only normal components,
/// dropping roots, prefixes, and `..`, so a peer cannot write outside the output directory.
fn safe_relative_path(header: &[u8]) -> PathBuf {
    let raw = String::from_utf8_lossy(header);
    let mut safe = PathBuf::new();
    for component in Path::new(raw.as_ref()).components() {
        if let std::path::Component::Normal(part) = component {
            safe.push(part);
        }
    }
    if safe.as_os_str().is_empty() {
        safe.push("download");
    }
    safe
}
