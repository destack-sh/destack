# Parser Fuzzing

> See [TESTING.md](../../../TESTING.md) for the overall testing philosophy and strategy.

Fuzz testing for the Destack parser using libfuzzer.

## Quick Start

```bash
# run the tokenizer fuzzer
cargo +nightly fuzz run destack_parser_fuzz

# run for a specific duration (5 minutes)
cargo +nightly fuzz run destack_parser_fuzz -- -max_total_time=300

# minimize the corpus
cargo +nightly fuzz cmin destack_parser_fuzz
```

## Fuzz Targets

| Target | Description | Status |
|--------|-------------|--------|
| `destack_parser_fuzz` | Tokenizer roundtrip | Active |

### destack_parser_fuzz (Tokenizer)

Tests the tokenizer roundtrip invariant:
1. Tokenize input string
2. Render tokens back to string
3. Tokenize again
4. Assert token sequences match

This catches:
- Tokenizer crashes on malformed input
- Tokenizer producing different tokens on re-lex
- Token rendering that loses information

## Corpus

The `corpus/` directory contains inputs discovered by fuzzing that exercise interesting code paths.
Currently ~2,100 entries.

## Artifacts

The `artifacts/` directory contains crash-inducing inputs.
When a crash is found, minimize it and create a regression test.

## Adding New Fuzz Targets

1. Create a new file in `fuzz_targets/`
2. Add a `[[bin]]` entry in `Cargo.toml`
3. Run the fuzzer to build initial corpus

Example target:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        // your fuzzing logic here
    }
});
```

## Future Targets

See [TESTING.md](../../../TESTING.md#fuzzing-expansion) for planned expansion.

Proposed additional targets:
- **parser**: Full AST parsing (beyond tokenization)
- **compiler**: Type checking on valid ASTs
- **formatter**: Format roundtrip stability
- **codegen_js**: JS output validity
