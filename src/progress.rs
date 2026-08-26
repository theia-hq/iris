//! Terminal progress reporting that stays out of the way of scripts.
//!
//! Bars render only when stderr is a terminal, and always draw to stderr, so piped or scripted use
//! sees nothing here and the `sent`/`received` lines on stdout stay clean and parseable.

use core::pin::Pin;
use core::task::{Context, Poll};
use std::io::IsTerminal as _;

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use tokio::io::{self, AsyncRead, AsyncWrite, ReadBuf};

/// A container for the concurrent per-file bars, hidden when stderr is not a terminal so scripted and
/// piped use stays clean. Bars added to it inherit its visibility.
pub fn multi() -> MultiProgress {
    let multi = MultiProgress::new();
    if !std::io::stderr().is_terminal() {
        multi.set_draw_target(ProgressDrawTarget::hidden());
    }
    multi
}

/// A progress bar for a transfer of known length. Add it to a [`multi`] to control visibility.
pub fn bar(len: u64, name: &str) -> ProgressBar {
    let bar = ProgressBar::new(len);
    // The template is a compile-fixed literal, so parsing it can only fail if this source is edited
    // wrong; the panic then fires in the first test run, never on user input.
    #[allow(clippy::expect_used)]
    bar.set_style(
        ProgressStyle::with_template(
            "{msg:.bold}  {bar:28.cyan/blue} {bytes:>10}/{total_bytes:<10} {binary_bytes_per_sec:>12}",
        )
        .expect("static progress template is valid")
        .progress_chars("=> "),
    );
    bar.set_message(name.to_owned());
    bar
}

/// A byte spinner for a transfer of unknown length. Add it to a [`multi`] to control visibility.
pub fn spinner() -> ProgressBar {
    let bar = ProgressBar::new_spinner();
    // Static template literal; see the note in `bar` for why the expect is unreachable on real input.
    #[allow(clippy::expect_used)]
    bar.set_style(
        ProgressStyle::with_template(
            "{spinner:.cyan} receiving  {bytes:>10} {binary_bytes_per_sec:>12}",
        )
        .expect("static progress template is valid"),
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
