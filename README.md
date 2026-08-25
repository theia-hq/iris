# iris

A verifiable file courier over the bifrost overlay. Send a file to a peer addressed by their public
key, not their location; every transfer is BLAKE3-verified end to end. Built on `bifrost` (reach) and
`bifrost-wire` (verified transfer).

> Experimental. The CLI and wire format will change; not ready for production use.

## Installation

Not yet published. Build from a checkout:

```sh
cargo install --path .
```

or run without installing with `cargo run --`.

## Usage

On the receiver, print your address and wait:

```sh
iris recv
```

On the sender, dial that address and stream a file:

```sh
iris send <path> <address>
```

The address is the receiver's `NodeId`, printed by `iris recv`.

## Things to know

- The transport is iroh today (self-discovering via n0 relays and DNS); the courier itself is
  transport-blind and rides any bifrost transport.
- Transfers are content-addressed and BLAKE3-verified; tampering or truncation is rejected.
- Identity is ephemeral per process for now — no persisted keypair yet, so the address changes each run.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
