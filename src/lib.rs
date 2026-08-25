//! iris: a verifiable file courier over the bifrost overlay.
//!
//! The command logic (`send`, `recv`) is exposed as a library so it can be driven over any bifrost
//! transport in tests; the binary wires it to iroh. See `main.rs` for the CLI.

pub mod identity;
pub mod recv;
pub mod send;

mod progress;
