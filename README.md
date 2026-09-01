# iris

> Archived: folded into [swoosh](https://github.com/theia-hq/swoosh) as `swoosh beam`. Use that.

Send files to a machine you address by its public key, not its location. Each transfer is hashed on the
way out and re-hashed on the way in, so a truncated or tampered file is rejected rather than saved.

Powered by [bifrost](https://github.com/theia-hq/bifrost) for the keyed connection.

**The name.** Iris is the messenger of Greek myth, who carried word between parties along the
rainbow. This carries a file to whoever holds a given key, and checks it arrived whole.

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
$ iris recv
bf01k2m…  # your address; share it with the sender
```

On the sender, dial that address and send one or more files or directories:

```sh
iris send bf01k2m… report.pdf photos/
```

The address is the receiver's public key, printed by `iris recv`. Received files land in the current
directory by default; `iris recv --out <dir>` chooses another.

## Things to know

- Integrity is checked end to end with BLAKE3: the receiver saves a file only if its hash matches what
  the sender computed, so tampering or truncation in flight is rejected.
- Your address is an ed25519 public key, persisted at `IRIS_KEY` or `~/.config/iris/identity.key`, so it
  stays the same across runs.
- Files stream in fixed-size chunks; a large transfer is never held whole in memory.
- The transport today is iroh, which finds a peer and traverses NATs on its own. iris does not depend on
  it directly and runs over any [bifrost](https://github.com/theia-hq/bifrost) transport.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
