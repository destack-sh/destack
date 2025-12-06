# Destack Guidelines

Destack is a full-stack software stack powered by our custom "TypeScript++" language (`.ds`).
The Destack language, library, and platform ecosystem are fully integrated for fantastic software development.
We aim to be a first-class citizen in the web and specifically the TypeScript ecosystem (with full bi-directional interoperability).

## Code Style

### READMEs

We have README.md for every substantial crate/package and even many modules/folders within those crates.
That's where significant documentation and context should be written.
We don't like writing information that is redundant and easily out of date (like folder structures or paths).

### Comments

Inline comments should be short and begin with a lowercase letter.
 - (This extends to comments in *any* code file, even scripts. I just like lowercase better.)
 - Place comments above a related code block (usually 2-10 lines).
 - Most comments are <1 sentence and should not include a period at the end (again, lowercase).
 - Avoid using hyphens inside comments, instead prefer colons or commas
Documentation comments for functions/types/etc. should be proper sentences with punctuation.
 - Files should NOT have a top-level documentation comments. They always get stale.
 - Go multiline if there is more than one sentence. Only one sentence should begin per line.
 - For methods, documentation should be imperative, usually starting with a verb (e.g., "Send a message").

Comments may start with keywords (without the spaces):
- `N O T E`: call out something important
- `T O D O`: something to address eventually
- `n o c h e c k i n`: temporary, should not be committed

Keywords should include tags:
- `#Performance`: could be faster or more efficient
- `#Robustness`: might be flaky in some cases
- `#Broken`: doesn't work in likely cases
- `#Cleanup`: could be simpler or better structured
- `#Incomplete`: obvious feature is missing
- `#Security`: may allow more access than intended
- `#Architecture`: larger design to reconsider

### Naming

Names should be obvious, clear, and idiomatic to the language.
Prefer writing out most names and words (even in variable names, `extension` > `ext`, `directory` > `dir`).
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

### Dependencies

Fewer dependencies is better.
When simple logic is needed, we just implement it ourselves.
Moderately complex logic is sometimes vendored.
Complex or dev-only dependencies are sometimes okay.
When adding a dependency, we should try go for the latest stable version.

### Testing

Tests should start with `test_` and state their content as a verb.
Example: `test_roundtrip_duration`, `test_send_recv_message`.

The first line or docstring should describe desired behavior (don't mention "test").
Prefer property-based testing and roundtrip testing where possible.

If there is an opportunity to test "the entire thing" vs "part of it", prefer complete asserts.
(For example, if we're generating string output, compare the entire output, not just "contains").

### Formatting

You should always format code before you're "done" with a change.
Ideally, you should format code *before* running it (via tests or otherwise), so we don't compile twice.
(Most directories have a `just fmt` or equivalent command, see the context.).

## Rust

Toolchain: `nightly-2025-11-27` (see `rust-toolchain.toml`)

- Place imports at the top, prefer `use std::time::Instant` patterns
- No `crate::X` within functions, use relative references (again, imports at the top)
- Avoid `unwrap`/`expect` outside tests; fail explicitly
- Tests go in a trailing `mod tests` or in standalone test modules/crates (contextual)
- Inline variables in format macros if possible: `format!("name is {name}")`
- Prefer multiline raw strings for longer strings
- Prefer using `--release` for build, test, check, etc. (it's faster)

## Commands

We use `justfile`s for commands. See `just --list` for all commands:
```sh
just check
just fmt
just build
just test
```

## Commits

Typically, agents aren't supposed to commit code directly, but for reference:
 - Follow `type(scope): summary` (≤100 chars, imperative).
 - Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`
 - Example: `feat(language): improve error span precision (to sub-token granularity)`