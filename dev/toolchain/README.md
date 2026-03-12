# Toolchain

Host setup and target tooling for Destack runtime, bridge, and release workflows.

## Testing

Run these from the repository root.

```sh
# focused local loop
just language/doctor-toolchain
just bridge/doctor-toolchain
just install-hygiene-toolchain

# clean gate
just check-hygiene

# exhaustive gate
just full
```
