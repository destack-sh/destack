# Repository Guidelines

Destack is a development platform powered by Dyst, our custom programming language (`.ds`).
Dyst modules are mostly backed by Rust currently, but can have bindings to and from other languages.

## Project Structure & Module Organization
Destack is primarily a Rust & Dyst workspace (see `Cargo.toml`) with these top-level areas:
- `client/` - Client SDKs (TypeScript, Python, ...)
- `demo/` - Example applications
- `development/` - Development and deployment utilities
- `extension/` - Various bridges and integration (LSP server + VS Code extension)
- `language/` - Compiler front- and back-ends for Dyst
- `library/` - Standard library
- `platform/` - Platform targets
- `test/` - Full-stack simulation tests

## Comments
Inline comments SHOULD be short and begin with a lowercase letter.
Inline comments SHOULD be above a related code block (these blocks are usually 2-10 lines).
Most comments are <1 sentence and SHOULD not include a . at the end.
Function, module, class, .. documentation must be proper sentences with punctuation. 

Comments MAY start with certain keywords:
 - NOTE: call out something important that should not be missed when reading this code.
 - TODO: something is missing / slow / imperfect about this code, should be addressed eventually.
 - n o c h e c k i n (without the spaces): this is temporary and should not be committed / checked in.
 (Do not use lowercase Note:, always NOTE)
Comments like that SHOULD also include one or more tags like:
 - NOTE @Performance: this clones the string, use custom alloc (?)
 - TODO @Cleanup: this seems unnecessarily confusing
 The allowable tags are:
 - @Performance: could be faster or more efficient
 - @Robustness: this might be flaky in some cases
 - @Broken: doesn't do what it should in likely cases
 - @Cleanup: could be simpler or better structured
 - @Incomplete: obvious/implicit feature is missing
 - @Security: should be hardened / may allow more access then intended
 - @Architecture: something larger than per-class/file code design to reconsider 

NOTE ALL "Note" comments MUST begin on a new line with uppercase NOTE and SHOULD have an @tag.
Sentences SHOULD start on their own line (except *very* short ones).
Prefer more specific verbs ("gets" or "computes" over plain "returns").

Custom types SHOULD be referred by their properly capiztalized names (like `Parse a Token.`).

## Documentation
Non-trivial or non-private methods SHOULD be documented, but we don't document every parameter/error/....
Go multiline if there is more than one sentence to say.
For methods, documentation should be imperative, starting with a verb ("Send a message to XYZ").

## Logic
Long methods are allowed and encouraged if the logic isn't extractable.
Otherwise, smaller functions are great especially in compiled languages.
Avoid nesting function/class definitions (though it's fine if needed).

Most methods/functions should have at least a one-line documentation.

Prefer pure(ish) functions.

For exhaustive matching, prefer if/else over match.
If we're checking anything that should cover all cases use some variant of assert_never in an else branch.

Larger code blocks - regardless of branching - should be broken up into logical chunks
 (demarcate with whitespace and/or preamble comments like `# parse HH:MM remainder`).

When calling functions or returning results any non-trivial operation gets a temporary variable.
Variables should be full words wherever possible and no obvious abbreviation exists.
Like:
```
let first_digit = (dt_bytes[0] - b'0') as i64;
let second_digit = (dt_bytes[1] - b'0') as i64;
let number = 10 * second_digit + first_digit;
Ok(number);
```

Prefer multiline strings for longer strings (raw strings in Rust, """\ in Python.)

## Naming
Naming should be obvious and clear (and idiomatic to the language), though not overly verbose.
Avoid single or few letter variables, method and function names unlress absolutely clear.
 (`i`, `x` and `Vector.x` are fine.)
Booleans should start with `is_` unless they're obvious.

## Performance
Performance is critical across the board.
Don't over-optimize when it makes code harder to read, but consideration up-front is important.

## Testing
Tests should start with `test_` and state their content as a sentence/verb.
(e.g., test_roundtrip_duration, test_send_recv_message).
Tests should elaborate desired behavior in the first line or a 1-2 line docstring.
(Do not mention "test" in the comment, just say what we're doing / what should happen in present tense.)
Wherever possible we like property-based testing, roundtrip testing and such. 

## Commits & PRs
Follow `type(scope): summary` (≤72 chars, imperative) such as `feat(language): add error spans`. PRs should explain motivation, list touched crates/workspaces, attach key `cargo`/`bun` outputs, link issues, and include UI evidence when demos or editor UX change.

## Rust
- Toolchain: `nightly-2025-08-14`.
- Place imports up top and prefer direct `use std::time::Instant` patterns.
- Avoid `unwrap`/`expect` outside tests; fail explicitly instead.
- Keep tests in a trailing `mod tests` and rely on inline capture formatting (`format!("tick {tick}")`).

### Commands
- `cargo check --workspace` quickly validates all Rust crates.
- `cargo build --workspace --release` produces optimized artifacts (web builds reside in `platform/web/target`).
- `cargo test --workspace --all-targets` runs unit and integration suites; add `--features ...` for feature-specific coverage.
- `cargo clippy --workspace --all-targets --all-features` must pass lint gates.
- `cargo fmt --all` applies the repo-level `rustfmt.toml`.

## TypeScript
Always type everything properly.
Avoid using as any or similar casts.

### Commands
- `bun install` then `bun run build` inside `client/destack_ts` compiles the TypeScript SDK; use `bun run test` for JS tests.
- `bun run vscode:compile` from the repo root produces the VS Code extension bundle.

## Python
Always type everything properly.
Use modern lowercase type annotations like `list[str] | None`.
Prefer Sequence/Mapping/.. and such as return annotation.
Avoid getattr/hasattr.
Imports should go to the top of the file.
Don't import anything from __future__.
When documenting with a multi-line string the first line should start on a newline:
"""
This is a long documentation.
More explanation here.
"""
Asserts should have format strings:
```
assert a == b, f"a != b: {a!r} != {b!r}"
```
Match enum-like things exhaustively with assert_never on the else.

