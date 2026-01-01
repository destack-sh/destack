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
 - Avoid using hyphens inside comments, instead prefer colons or commas (except for proper compound words)
Inline comments may also just be single words or sequences of words if the "scoping" is clear; i.e., not every inline comment needs to be a sentence.
Comments serve to organize the reader's mental model of the code, so they can be just anything from a one-word summary, a three word phrase, or a short explanatory note.

Documentation comments for functions/types/etc. should be proper sentences with punctuation.
 - Files should NOT have a top-level documentation comments. They always get stale.
 - Go multiline if there is more than one sentence. Only one sentence should begin per line.
 - For methods, documentation should be imperative, usually starting with a verb (e.g., "Send a message").
*All* functions, types, variants/fields, etc. should have documentation (one line is fine).
Documentation comments do not need to start with a verb, they should just plainly state what the thing is (e.g., for a field, "The blocks built so far." is better than "Represents the blocks built up to this point."; more succint is better).

When documenting if/else-if/else logic, the comments should go *before* each case like so:
```
// do this
if (...) {
  ...
} 
// otherwise do this
else if (...) {
  ...
} 
// fall back to this
else {
  ...
}

Comments MAY start with keywords:
- `NOTE`: call out something important
- `TODO`: something to address eventually
- `FUGU`: temporary, f-ed up, should be addressed before going upstream

Keywords should include tags (like "NOTE #Suspicious: allocating in runtime seems wrong?"):
- `#Performance`: could be faster or more efficient
- `#Robustness`: might be flaky in some cases
- `#Broken`: doesn't work in likely cases
- `#Cleanup`: could be simpler or better structured
- `#Incomplete`: obvious feature is missing
- `#Suspicious`: something that looks wrong or weird
- `#Security`: may allow more access than intended
- `#Architecture`: larger design to reconsider

### Naming

Names should be obvious, clear, and idiomatic to the language.
Prefer writing out most names and words (even in variable names, `extension` > `ext`, `directory` > `dir`).
Avoid single-letter variables unless obvious (`i`, `x`, `Vector.x` are fine).
Booleans should start with `is_` unless already clear.

### Logic

Long methods are allowed if the logic isn't meaningfully extractable.
Prefer pure(ish) functions, pass in context explicitly when needed (usually as the last argument).

Break larger code blocks into logical chunks with whitespace and/or preamble comments.
All logic in functions and outside should be broken into small-ish coherent blocks (2-8 lines or so) with a preceding comment.
Logic blocks are always separated by blank lines (except the first).
Usually you want the comment before the if clause / loop / whatever, not inside.
Every logic block should have a comment (returns may omit the comment), and every logic block (except the first) should have a blank line before it.
The return value implicit or explicit should also have a blank line before it, even if it's uncommented (which is, again, fine).

Use temporary variables for non-trivial operations (yes, it's deliberately verbose):
```rust
let first_digit = (dt_bytes[0] - b'0') as i64;
let second_digit = (dt_bytes[1] - b'0') as i64;
let number = 10 * first_digit + second_digit;
```

Try to make logic "incrementally granular" (as per Casey Muratori), i.e., ideally we should be able to reuse logic at various pieces of logic.
Conceptually, this means not hiding details too much, and assuming (especially internally) that the caller is a consenting adult.
More specifically, for example, when a function takes an array of something, try to make it work on a single "element" instead and just loop in the caller.

### Errors

Always prefer explicit, loud errors through conventional channels. 
Outside of tests, errors should almost never be suppressed or somehow default to "default values".

### Dependencies

Fewer dependencies is better, but sometimes it's worth it.
When simple logic is needed, we just implement it ourselves.
Moderately complex logic is sometimes vendored.
Complex or dev-only dependencies are sometimes okay.
When adding a dependency, we should try go for the latest stable version.

### Testing

Tests should start with `test_` (or equivalent) and state their content as a verb.
Example: `test_roundtrip_duration`, `test_send_receive_message`.

The first line or docstring should describe desired behavior (don't mention "test").
Prefer property-based testing and roundtrip testing where possible.

If there is an opportunity to test "the entire thing" vs "part of it", prefer complete asserts.
(For example, if we're generating string output, compare the entire output, not just "contains").

### Formatting

You should always format code before you're "done" with a change.
Ideally, you should format code *before* running it (via tests or otherwise), so we don't compile twice.
(Most directories have a `just fmt` or equivalent command, see the context.)

## Rust

Toolchain: `nightly-2025-11-27` (see `rust-toolchain.toml`)

- Place imports at the top, prefer `use std::time::Instant` patterns
- Comments/documentation goes before *all* attributes (like `#[inline]`, `#[derive]`, etc.)
- No `crate::X` within functions, prefer relative references (again, imports at the top)
- Avoid `unwrap`/`expect` outside tests; fail explicitly
- Tests go in a trailing `mod tests` or in standalone test modules/crates (contextual)
- Inline variables in format macros if possible: `format!("name is {name}")`
- Prefer multiline raw strings for longer strings
- Prefer using `--release` for build, test, check, etc. (it's faster)
- Just use `pub use submodule::*` for public exports, we use `pub` properly
- Relatedly, we like to just use `use crate::x` directly (when possible)
- Prefer re-defining variables if we're just transforming them about
  (e.g., `let module = modules.get(); let module = module.read();` is fine)
- Fix all the lints from `cargo check --release -p <crate>` and `cargo clippy --release -p <crate>`


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