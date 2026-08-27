use std::path::{Path, PathBuf};

use bifrost::{Discovery, Node, NodeId, Session, Transport};
use bifrost_wire::Transfer;
use clap::Args;
use eyre::WrapErr as _;
use futures::StreamExt as _;
use futures::stream::FuturesUnordered;
use indicatif::{MultiProgress, ProgressBar};
use tokio::io::{self, AsyncWriteExt as _};

use crate::progress::{self, ProgressWriter};

/// Wait to receive files into a directory, saving each until the sender is done.
#[derive(Debug, Args)]
pub struct RecvCmd {
    /// Directory to write received files into.
    #[arg(short, long, value_name = "dir", default_value = ".")]
    pub out: PathBuf,
}

impl RecvCmd {
    pub async fn run<T: Transport, D: Discovery>(self, node: &Node<T, D>) -> eyre::Result<()> {
        println!("iris ready. share this address with the sender:\n");
        println!("    {}\n", node.node_id());
        println!("waiting for a sender. press ctrl-c to stop.");

        let session = node.accept().await.wrap_err("waiting for a sender")?;
        let peer = session.peer();
        let multi = progress::multi();

        // Receive streams concurrently as the sender opens them; the sender's close ends the loop.
        let mut receiving = FuturesUnordered::new();
        let mut opened = 0usize;
        let mut count = 0usize;
        loop {
            tokio::select! {
                accepted = session.accept_bi() => {
                    match accepted {
                        Ok((send, recv)) => {
                            let index = opened;
                            opened += 1;
                            receiving.push(receive_one(send, recv, PathBuf::clone(&self.out), MultiProgress::clone(&multi), peer, index));
                        }
                        Err(_) => break,
                    }
                }
                Some(result) = receiving.next(), if !receiving.is_empty() => {
                    result?;
                    count += 1;
                }
            }
        }
        while let Some(result) = receiving.next().await {
            result?;
            count += 1;
        }

        if count == 0 {
            eyre::bail!("sender closed without sending a file");
        }
        println!("received {count} file(s), saved to {}", self.out.display());
        Ok(())
    }
}

/// Receive one stream into a temp file, verify, then move it into place under `out`.
async fn receive_one<W, R>(
    writer: W,
    reader: R,
    out: PathBuf,
    multi: MultiProgress,
    peer: NodeId,
    index: usize,
) -> eyre::Result<()>
where
    W: io::AsyncWrite + Unpin,
    R: io::AsyncRead + Unpin,
{
    let temp = out.join(format!(".iris-{}-{index}.part", std::process::id()));
    let spinner = multi.add(progress::spinner());
    let received = {
        let file = tokio::fs::File::create(&temp).await?;
        let mut sink = ProgressWriter::new(file, ProgressBar::clone(&spinner));
        match Transfer::new(writer, reader).recv(&mut sink).await {
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
    let final_path = out.join(&relative);
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
    Ok(())
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
