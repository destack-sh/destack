## Code Style

### READMEs

We have README.md for every substantial crate/package and even many modules/folders within those projects.
We don't like writing information that is redundant and easily out of date into the READMEs or specifications (so, avoid folder structures, paths, or "current status").

### Comments

Inline comments should be short and begin with a lowercase letter.
 - (This extends to comments in *any* code file, even scripts. I just like lowercase better.)
 - Place comments above a related code block (usually 2-10 lines).
 - Most comments are <1 sentence and should not include a period at the end (again, lowercase).
 - Avoid using hyphens inside comments, instead prefer colons or commas (except for proper compound words)

Inline comments may also just be single words or sequences of words if the "scoping" is clear; i.e., not every inline comment needs to be a sentence.
Comments serve to organize the reader's mental model of the code, so they can be just anything from a one-word summary, a three word phrase, or a short explanatory note.

Trivial functions (<3-4 lines) do not _need_ comments / blank lines, especially when the comments just repeat the documentation above.
Also, tests don't need quite the same level of comments, especially within obvious test cases.

Documentation comments for functions/types/etc. should be proper sentences with punctuation.
 - Files should NOT have a top-level documentation comments. They always get stale.
 - Go multiline if there is more than one sentence. Only one sentence should begin per line.
 - For methods, documentation should be imperative, usually starting with a verb (e.g., "Send a message").

*All* functions, types, variants/fields, etc. should have documentation (one line is fine).
Documentation comments do not need to start with a verb, they should just plainly state what the thing is (e.g., for a field, "The blocks built so far." is better than "Represents the blocks built up to this point."; more succint is better).

When documenting if/else-if/else-_like_ logic, the comments should go *before* each case like so:
```text
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
```

For ===-like separators for large comment blocks, you may use upper case sentences:
```text
// ================================================================================
// Binary operator precedence
// ================================================================================
```
Though try to minimize the number of these, they're quite noisy.

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
- `#Architecture`: larger design issue to reconsider

### Naming

- Names should be obvious, clear, and idiomatic to the language and topic.
- Where relevant prior art exists, we should follow existing modern terminology.
- Shorter, stronger nouns and verbs are almost always better
- Prefer writing out most names and words (even in variable names, `extension` > `ext`, `directory` > `dir`).
- As with logic, symmetry in naming across related logic is simpler 
- Avoid single-letter variables unless obvious (`i`, `x`, `Vector.x` are fine).
- Booleans should start with `is_` unless already clear (or otherwise required by context).
- Abstraction sludge names like "seam", "lane", "info", "factory", .. and friends are to be treated with high suspicion and almost certainly wrong.
- The same logic applies for module and file names too (single part file names are clearer, "support" is sludge, etc.)

### Logic

- Less is more, every line of code is a liability
- Fewer overloads are better, fewer fields are better, ...
- Long methods are allowed if the logic isn't meaningfully extractable / resuable.
- Prefer pure(ish) functions, pass in context explicitly when needed (usually as the last argument).
- Break larger code blocks into logical chunks with whitespace and/or preamble comments.
- All logic in functions and outside should be broken into small-ish coherent blocks (2-7 lines or so) with a preceding comment.
- Logic blocks are always separated by blank lines (except the first).
- Usually you want the comment before the if clause / loop / whatever, not inside.
- Every logic block should have a comment (returns may omit the comment), and every logic block (except the first) should have a blank line before it.
- The return value implicit or explicit should also have a blank line before it, even if it's uncommented (which is, again, fine).

Use temporary variables for non-trivial operations (yes, it's deliberately verbose):
```rust
let first_digit = (dt_bytes[0] - b'0') as i64;
let second_digit = (dt_bytes[1] - b'0') as i64;
let number = 10 * first_digit + second_digit;
```

- Try to make logic "incrementally granular" (as per Casey Muratori), i.e., ideally we should be able to reuse logic at various pieces of granularity.
- Conceptually, this means not hiding details too much, and assuming (especially internally) that the caller is a consenting adult.
- More specifically, as a trivial example, when a function takes an array of something, try to make it work on a single "element" instead and just loop in the caller.
- Prefer parameteric mutability. etc. etc. that sort of thing.

### Refactoring

- We should always strive to refactor and "clean" as we go, continuously re-audit and semantically compress where the opportunity presents itself
- Relatedly, as we go, we must never assume that what is already there is good just because it exists, even if it's in use
- Every noun, verb, type, variant, field, line, .. must be earned. 
- Bloat is deadly, and often we only realise something was bloated as we get further along and the true shape of the problem reveals itself (hence, refactor as we go)
- Never introduce "transitional" or "for now" logic, we always want the final ideal shape, nothing in between
- It is often better to break / change the source directly and then let the compiler guide

### Failures

- Always prefer explicit, loud errors through conventional channels.
- Outside of tests, errors should almost never be suppressed or somehow fall back to "default values" (especially evil are things like defaulting `unwrap_or(0)`, or other special values like `-1`, `MAX`).
- On the flipside, in general, and especially internally, we should assume that both sides of an API are consenting adults and we should _not_ check every conceivable failure state in every location - this is usually more noise than it's worth.

### Boundaries

- Prefer loud failures even and especially for invariants coming from other subsystems, and _especially_ for subsystems we control.
- For example, if some upstream shape or contract implies a certain field in some state should be there at some point, but it's not, we MUST treat that as an error instead of working around it in any capacity.
- Attempting to work around issues in upstream / other dependencies is always dangerous, but doing it for dependencies _we control_ is just a recipe for maintenance disaster.
- Invariants should be clear and crisp, and if they're not, that is a design issue to be surfaced and discussed.

### Dependencies

- Fewer dependencies is better, but sometimes it's worth it.
- When simple logic is needed, we just implement it ourselves.
- Moderately complex logic is sometimes vendored.
- Complex or dev-only dependencies are sometimes okay.
- When adding a dependency, we should use the latest _stable_ version.

### Testing

Tests should start with `test_` (or equivalent) and state their content as a verb.
Example: `test_roundtrip_duration`, `test_send_receive_message`.

The first line or docstring should describe desired behavior (don't mention "test").
Prefer property-based testing and roundtrip testing where possible.

If there is an opportunity to test "the entire thing" vs "part of it", prefer complete asserts.
(For example, if we're generating string output, compare the entire output, not just "contains").
More generally, we should always test *specific outcomes* like "these two errors with that message" rather than "expect failed" or "any two errors".
Even better, where possible, we should assert the entire expected output (snapshot style) rather than just "contains" or "doesn't contain".
For any non-trivial assertions you should comment the logic block like we do with any other logic block, though you don't need to comment *every* logic block as with regular/main logic.

### Formatting

You should always format code before you're "done" with a change.
Ideally, you should format code *before* running it (via tests or otherwise), so we don't compile twice.
(Most directories have a `just fmt` or equivalent command, see the context.)

## Rust

### Development

- If you encounter an ICE, just do `cargo clean` (same if you run out of disk space)
- Comments/documentation goes before *all* attributes (like `#[inline]`, `#[derive]`, etc.)
- No `crate::X` within functions, prefer relative references (again, imports at the top)
- Place imports at the top, prefer `use std::time::Instant` patterns
- Just use `pub use submodule::*` for public exports, we use `pub` properly
- Relatedly, we like to just use `use crate::x` directly at the top level (when possible)
- `mod.rs` and `main.rs` are intended strictly for re-exports
- Modules should either be `module.rs` or have `module/mod.rs` + real `module/whatever.rs`, never both
- Avoid `include!` or convoluted `#[path]` to bypass
- Avoid nesting `mod x { }` inside a file (except for `tests`)

### Logic

- Put constants at the top of the file (no magic numbers/values)
- Avoid `unwrap`/`expect`/`panic` etc. outside tests; fail explicitly, use proper Result handling
- Tests go in a trailing `mod tests` or in standalone test modules/crates (contextual)
- Inline variables in format macros if possible: `format!("name is {name}")`
- Prefer multiline raw strings for longer strings
- Prefer re-defining variables if we're just transforming them about
  (e.g., `let module = modules.get(); let module = module.read();` is fine)
- Avoid nesting items inside of functions (like other functions, lambdas, types, etc.)

### Lints and warnings

- Fix all the lints from `cargo check -p <crate>` and `cargo clippy -p <crate>`
- Most clippy allow stuff should go on top of the `impl`, not individual functions (like too many arguments is almost always fine at a broad scope)
- In general, ignore too many arguments and type complexity warnings
- Put lint suppression at the top of the impl block, not individual functions

## Markdown

- One sentence per line. Always (in prose, tables and such are different).
- Use proper rich formatting: sections, sub-sections, highlighting, code examples, tables, etc.
- Non-prose items (lists, code blocks, tables) in a subsection should be preceded by a prose line

## Commands

We use `justfile`s for commands. See `just --list` for all commands:
```sh
just check
just fmt
just build
just test
```

## Commits

- Typically, agents aren't supposed to commit or merge directly without being explicitly instructed to.
- For commit message format, follow `CONTRIBUTING.md#commit-style`.
- We typically work with branches and worktrees off a main branch.
- We try to frequently rebase of main and merge back into main.
- When merging into main, try to fast-forward or cherry-pick to retain the commit history (except when there are a _lot_ of small commits, feel free to squash then).
