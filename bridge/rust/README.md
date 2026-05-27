# destack (Rust)

Rust client for Destack.
This crate is published to crates.io as `destack`.
Legacy alias crate `destack-rs` is in [`aliases/destack-rs`](aliases/destack-rs/README.md).

## Installation

```toml
[dependencies]
destack = "0.55.4"
```

## API

```rust
let client = destack::Client::new();
assert_eq!(client.backend(), "rust");
assert!(client.capi_abi_version() > 0);
assert!(client.capi_is_available());
assert_eq!(destack::version(), "0.55.4");
```

## Testing

Run these from the repository root.

```sh
# focused local loop
just bridge/test

# clean check
just bridge/check-quick

# exhaustive check
just bridge/check-full
```
