# nocheckin: remove this plan

# Conformance Test Failure Analysis

## Current Status: 77% (4130/5363 test262 tests)

Last updated: 2024-12-10

## Category Breakdown (1233 failures)

| Category | Count | Issue |
|----------|-------|-------|
| **pass/** | 382 | Valid JS we're rejecting |
| **fail/** | 376 | Invalid JS we're accepting |
| **pass-explicit/** | 327 | Valid modules we're rejecting |
| **early/** | 148 | Valid syntax with semantic errors we're rejecting |

## Root Causes Identified from Diagnostics

### PASS/ failures (things we reject but shouldn't)

| Error Pattern | Example | Root Cause | Est. Impact |
|---------------|---------|------------|-------------|
| `unexpected :` | `a: while (true) { break a; }` | **Labeled statements not parsed** | ~49 tests |
| `unexpected Identifier` (for of) | `for (var a of b) c(a);` | **`of` keyword not recognized in for loops** | ~25-30 tests |
| `unexpected ;` (for in destructure) | `for (var {a, b} in c);` | **Destructuring in for-in not supported** | ~50 tests |
| `unexpected End` (do-while) | `do continue; while (true)` | **do-while parsing issue** | ~8 tests |
| `unexpected End` (new) | `new a` | **new without parens not handled** | ~17 tests |
| `unexpected <` | `<!--\n;` | **HTML comments not supported** | ~14 tests |
| `unexpected Unknown` | `var A\u{42}C;` | **Unicode escapes in identifiers** | ~15 tests |

### FAIL/ failures (things we accept but shouldn't)

| Pattern | Example | Missing Check |
|---------|---------|---------------|
| No error | `3 = 4` | **Invalid LHS in assignment** |
| No error | `/*` | **Unterminated block comment** |
| No error | `{ return; }` | **return outside function** (semantic) |

## Priority Order for Fixes

1. **for...of support** (~25-30 tests) - `of` keyword handling in for loops
2. **Labeled statements** (~49 tests) - `label:` prefix syntax
3. **for-in destructuring** (~50 tests) - `for (var {a,b} in c)`
4. **new without parens** (~17 tests) - `new Foo` vs `new Foo()`
5. **do-while edge cases** (~8 tests)
6. **Unicode escapes** (~15 tests) - `\u{42}` in identifiers
7. **HTML comments** (~14 tests) - `<!--` and `-->`

For **fail/** tests:
1. Invalid LHS validation (`3 = 4`)
2. Unterminated comment detection

## Progress Tracking

- [ ] HTML comments (`<!--`, `-->`) - lexer
- [ ] Unicode escapes in identifiers (`\u{42}`) - lexer
- [ ] for...of loops
- [ ] Labeled statements (`label:`)
- [ ] for-in destructuring
- [ ] new without parens
- [ ] do-while edge cases
- [ ] Invalid LHS validation
- [ ] Unterminated comment detection

## Notes

### Labeled Statements
Destack uses `:label` syntax (Rust-style) in `block.rs:186-247`.
JavaScript uses `label:` prefix. Need to support both for JS target compatibility.

### HTML Comments
Legacy web compat feature. `<!--` starts single-line comment, `-->` also acts as comment.
Only valid in script mode (not modules).

### Unicode Escapes
ECMAScript allows `\uXXXX` and `\u{X...}` in identifiers.
Example: `var A\u{42}C;` is equivalent to `var ABC;` (42 hex = 66 = 'B')
