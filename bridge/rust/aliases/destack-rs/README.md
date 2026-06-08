# destack-rs

Destack legacy alias crate for Rust.
This crate re-exports the canonical `destack` crate.

## Installation

```toml
[dependencies]
destack-rs = "0.55.4"
```

## API

```rust
let session = destack_rs::Session::open_path(".")?;
```
