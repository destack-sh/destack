# destack (Rust)

Rust client for Destack.
This crate is published to crates.io as `destack`.

## Installation

```toml
[dependencies]
destack = "0.55.2"
```

## API

```rust
let client = destack::Client::new();
assert_eq!(client.backend(), "rust");
assert_eq!(destack::version(), "0.55.2");
```
