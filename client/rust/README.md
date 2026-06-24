# destack (Rust)

Rust language client for Destack.
This crate is published to crates.io as `destack`.

## Installation

```toml
[dependencies]
destack = "0.55.4"
```

## API

```rust
let server = destack::workspace::Server::open(".")?;

drop(server);
```

## Testing

Run these from the repository root.

```sh
# focused local loop
just client/test

# clean check
just client/check-quick

# exhaustive check
just client/check-full
```
