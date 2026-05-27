# capi

C ABI bridge layer for Destack clients.
This crate exposes a minimal stable surface for FFI-based language clients.

## Header

The public C header lives at `include/destack.h`.

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
