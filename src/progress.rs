//! Terminal progress reporting that stays out of the way of scripts.
//!
//! Bars render only when stderr is a terminal, and always draw to stderr, so piped or scripted use
//! sees nothing here and the `sent`/`received` lines on stdout stay clean and parseable.

use std::io::IsTerminal as _;
use std::pin::Pin;
use std::task::{Context, Poll};

use indicatif::{ProgressBar, ProgressStyle};
use tokio::io::{self, AsyncRead, AsyncWrite, ReadBuf};

/// A progress bar for a transfer of known length, or a hidden no-op bar off a terminal.
pub fn bar(len: u64, name: &str) -> ProgressBar {
    if !std::io::stderr().is_terminal() {
        return ProgressBar::hidden();
    }
    let bar = ProgressBar::new(len);
    bar.set_style(
        ProgressStyle::with_template(
            "{msg:.bold}  {bar:28.cyan/blue} {bytes:>10}/{total_bytes:<10} {binary_bytes_per_sec:>12}",
        )
        .unwrap()
        .progress_chars("=> "),
    );
    bar.set_message(name.to_owned());
    bar
}

/// A byte spinner for a transfer of unknown length, or a hidden no-op bar off a terminal.
pub fn spinner() -> ProgressBar {
    if !std::io::stderr().is_terminal() {
        return ProgressBar::hidden();
    }
    let bar = ProgressBar::new_spinner();
    bar.set_style(
        ProgressStyle::with_template(
            "{spinner:.cyan} receiving  {bytes:>10} {binary_bytes_per_sec:>12}",
        )
        .unwrap(),
    );
    bar
}

/// Wraps a reader, advancing a progress bar by the bytes read.
pub struct ProgressReader<R> {
    inner: R,
    bar: ProgressBar,
}

impl<R> ProgressReader<R> {
    pub fn new(inner: R, bar: ProgressBar) -> Self {
        Self { inner, bar }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for ProgressReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        let poll = Pin::new(&mut this.inner).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = &poll {
            this.bar.inc((buf.filled().len() - before) as u64);
        }
        poll
    }
}

/// Wraps a writer, advancing a progress bar by the bytes written.
pub struct ProgressWriter<W> {
    inner: W,
    bar: ProgressBar,
}

impl<W> ProgressWriter<W> {
    pub fn new(inner: W, bar: ProgressBar) -> Self {
        Self { inner, bar }
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for ProgressWriter<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let poll = Pin::new(&mut this.inner).poll_write(cx, buf);
        if let Poll::Ready(Ok(written)) = &poll {
            this.bar.inc(*written as u64);
        }
        poll
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}
