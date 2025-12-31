# Stress Tests

Stress tests verify the toolchain handles extreme scale without crashing, hanging, or running out of memory.

Unlike fuzz tests (which explore random/malformed inputs), stress tests use **valid but extreme** inputs.

## Structure

```
stress/
├── generate.sh              # top-level script
├── parser/                  # parser throughput: deep nesting, long lines, wide files
├── resolver/                # module resolution: many modules, deep import chains
└── checker/                 # type system: many types, deep inheritance, large unions
```

## Usage

```bash
# generate all fixtures (not checked into git)
just generate-stress

# run all stress tests
just test-stress
```
