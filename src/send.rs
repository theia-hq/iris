use std::path::{Path, PathBuf};

use bifrost::{Discovery, Node, NodeId, Session, Transport};
use bifrost_wire::{Blob, Transfer};
use clap::Args;
use eyre::WrapErr as _;
use futures::StreamExt as _;
use futures::stream::FuturesUnordered;
use indicatif::MultiProgress;

use crate::progress::{self, ProgressReader};

/// Files send concurrently over separate streams, capped so one connection is not flooded.
const MAX_INFLIGHT: usize = 16;

/// Send files or directories to a peer, addressed by their node id.
#[derive(Debug, Args)]
pub struct SendCmd {
    /// The recipient's node id, as printed by `iris recv`.
    pub peer: NodeId,
    /// The files or directories to send.
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,
}

impl SendCmd {
    pub async fn run<T: Transport, D: Discovery>(self, node: &Node<T, D>) -> eyre::Result<()> {
        println!("connecting to {}...", self.peer.short());
        let session = node
            .connect(self.peer)
            .await
            .wrap_err_with(|| format!("could not reach {}", self.peer.short()))?;

        // Expand directories, then pipeline up to MAX_INFLIGHT files over concurrent streams.
        let mut files = Vec::new();
        for path in &self.paths {
            files.extend(collect_files(path).await?);
        }

        let multi = progress::multi();
        let mut pending = files.into_iter();
        let mut sending = FuturesUnordered::new();
        for _ in 0..MAX_INFLIGHT {
            match pending.next() {
                Some((name, path)) => sending.push(send_one(&session, &multi, name, path)),
                None => break,
            }
        }
        while let Some(result) = sending.next().await {
            result?;
            if let Some((name, path)) = pending.next() {
                sending.push(send_one(&session, &multi, name, path));
            }
        }

        node.close().await;
        Ok(())
    }
}

/// Send one file over its own stream, showing a progress bar.
async fn send_one<S: Session>(
    session: &S,
    multi: &MultiProgress,
    name: String,
    path: PathBuf,
) -> eyre::Result<()> {
    let blob = {
        let mut file = tokio::fs::File::open(&path)
            .await
            .wrap_err_with(|| format!("open {}", path.display()))?;
        Blob::hash(&mut file).await?
    };

    let (send, recv) = session.open_bi().await?;
    let file = tokio::fs::File::open(&path)
        .await
        .wrap_err_with(|| format!("open {}", path.display()))?;
    let bar = multi.add(progress::bar(blob.len(), &name));
    let mut source = ProgressReader::new(file, bar.clone());
    Transfer::new(send, recv)
        .send(name.as_bytes(), &blob, &mut source)
        .await?;
    bar.finish_and_clear();

    println!("sent {name} ({} bytes)", blob.len());
    Ok(())
}

/// Collect `(relative name, path)` pairs to send: a file yields itself; a directory yields every file
/// under it, named by its path relative to the directory's parent (so the directory name is kept).
async fn collect_files(root: &Path) -> eyre::Result<Vec<(String, PathBuf)>> {
    let meta = tokio::fs::metadata(root)
        .await
        .wrap_err_with(|| format!("stat {}", root.display()))?;

    if meta.is_file() {
        let name = file_name(root)?;
        return Ok(vec![(name, root.to_path_buf())]);
    }

    let base = root.parent().unwrap_or(root);
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries = tokio::fs::read_dir(&dir)
            .await
            .wrap_err_with(|| format!("read {}", dir.display()))?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_type = entry.file_type().await?;
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                let relative = path
                    .strip_prefix(base)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((relative, path));
            }
        }
    }
    Ok(files)
}

fn file_name(path: &Path) -> eyre::Result<String> {
    path.file_name()
        .and_then(|component| component.to_str())
        .map(str::to_owned)
        .ok_or_else(|| eyre::eyre!("path has no file name: {}", path.display()))
}
