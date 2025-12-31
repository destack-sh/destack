# Ecosystem Tests

Test the Destack toolchain against real-world TypeScript/JavaScript packages.

## Structure

```
ecosystem/
├── packages/           # package manifests (*.toml)
├── patches/            # patches to apply to packages (dsconfig.json, fixes, etc.)
│   └── <package>/      # patches for a specific package
├── cache/              # cloned packages (gitignored)
└── baselines/          # known failure lists
```

## Running

```bash
# fetch all packages
just language/ecosystem-fetch

# run ecosystem tests
cargo test --release --test ecosystem

# run specific package
cargo test --release --test ecosystem -- zod
```

## Adding a Package

1. Create `packages/<name>.toml` with package metadata
2. Optionally add patches in `patches/<name>/`
3. Run `just language/ecosystem-fetch`
4. Run tests with `--update-known-failures` to capture baseline

## Tiers

| Tier | Description |
|------|-------------|
| 1: Parse | Can parse all files without errors |
| 2: Check | Type checking passes |
| 3: Compile JS | JS codegen produces valid output |
| 4: Compile Native | Works with `strict_portable` |
