# Destack Guidelines

Destack is a full-stack software stack powered by our custom language (`.ds`).
A TypeScript superset with integrated language, library, and platform ecosystem.

## Code Style

### Comments

Inline comments should be short and begin with a lowercase letter.
Place them above a related code block (usually 2-10 lines).
Most comments are <1 sentence and should not include a period at the end.

Function, module, and class documentation must be proper sentences with punctuation.
Go multiline if there is more than one sentence.
For methods, documentation should be imperative, starting with a verb ("Send a message to XYZ").

Comments may start with keywords:
- `NOTE`: call out something important
- `TODO`: something to address eventually
- `nocheckin`: temporary, should not be committed

Keywords should include tags:
- `#Performance`: could be faster or more efficient
- `#Robustness`: might be flaky in some cases
- `#Broken`: doesn't work in likely cases
- `#Cleanup`: could be simpler or better structured
- `#Incomplete`: obvious feature is missing
- `#Security`: may allow more access than intended
- `#Architecture`: larger design to reconsider

Example: `NOTE #Performance: avoid cloning string in parser`

### Naming

Names should be obvious, clear, and idiomatic to the language.
Avoid single-letter variables unless obvious (`i`, `x`, `Vector.x` are fine).
Booleans should start with `is_` unless already clear.

### Logic

Long methods are allowed if the logic isn't extractable.
Prefer pure(ish) functions.
Break larger code blocks into logical chunks with whitespace and/or preamble comments.

For exhaustive matching, prefer if/else over match.
Use `assert_never` in else branches for exhaustive checks.

Use temporary variables for non-trivial operations:

```rust
let first_digit = (dt_bytes[0] - b'0') as i64;
let second_digit = (dt_bytes[1] - b'0') as i64;
let number = 10 * first_digit + second_digit;
```

### Testing

Tests should start with `test_` and state their content as a verb.
Example: `test_roundtrip_duration`, `test_send_recv_message`.

The first line or docstring should describe desired behavior (don't mention "test").
Prefer property-based testing and roundtrip testing where possible.

## Rust

Toolchain: `nightly-2025-11-27` (see `rust-toolchain.toml`)

- Place imports at the top, prefer `use std::time::Instant` patterns
- No `crate::X` within functions, use relative references
- Avoid `unwrap`/`expect` outside tests; fail explicitly
- Tests go in a trailing `mod tests`
- Inline variables in format macros: `format!("name is {name}")`
- Public and complex function docs should list arguments and return values
- Prefer multiline raw strings for longer strings

### Commands

```sh
just language/check    # cargo check --workspace
just language/build    # cargo build --workspace --release
just language/test     # cargo test --workspace --all-targets
just language/lint     # cargo clippy --workspace --all-targets --all-features
just language/fmt      # cargo fmt --all
```

## TypeScript / Destack

- Always type everything properly
- Avoid `as any` or similar casts
- Use Bun as the runtime

### Commands

```sh
just install           # bun install
just library/napi      # build napi bindings
bun test               # run tests
```

## Commits

Follow `type(scope): summary` (≤100 chars, imperative).
Example: `feat(language): add error spans`
Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`
