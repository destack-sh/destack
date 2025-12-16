# Stress Tests

Stress tests verify the toolchain handles extreme scale without crashing, hanging, or running out of memory.

Unlike fuzz tests (which explore random/malformed inputs), stress tests use **valid but extreme** inputs.

## Structure

```
stress/
├── generate.sh              # top-level script
├── large_files/             # parser throughput: 100k lines, deep nesting, long lines
├── large_projects/          # module resolution: many modules, deep import chains
├── memory/                  # type system: many types, deep inheritance, large unions
├── edge_cases/              # parser robustness: empty files, BOM, mixed line endings
└── pathological/            # adversarial: unclosed brackets, malformed input, error recovery
```

## Usage

```bash
# generate all fixtures (not checked into git)
just generate-stress

# run all stress tests
just test-stress
```
