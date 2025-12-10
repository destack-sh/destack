# Conformance Test Analysis

**Current Status**: 74.5% (6074/8158 tests passing)

| Suite | Passed | Failed | Total | Rate |
|-------|--------|--------|-------|------|
| test262 | 4150 | 1213 | 5363 | 77.4% |
| babel | 1011 | 462 | 1473 | 68.6% |
| swc | 518 | 167 | 685 | 75.6% |
| biome | 395 | 242 | 637 | 62.0% |

## Test262 Failures (1213 tests)

Test262 is the official JavaScript conformance suite. Failures are categorized by type:

| Category | Description | Count |
|----------|-------------|-------|
| pass | Valid JS we're rejecting | 367 |
| pass-explicit | Valid modules we're rejecting | 327 |
| fail | Invalid JS we're accepting | 379 |
| early | Valid JS with semantic errors we're rejecting | 140 |

### Pattern Analysis

**PASS/PASS-EXPLICIT failures** (valid JS we reject, ~694 tests):

| Pattern | Count | Example | Fix Difficulty |
|---------|-------|---------|----------------|
| Labeled statements | ~100 | `a: while (true) { break a }` | Medium |
| Generator/yield | ~70 | `function* a() { yield }` | Medium |
| for-of/for-in | ~100 | `for (var {a, b} in c);` | Medium |
| for destructuring | ~40 | `for ([a,b] in c);` | Medium |
| do-while edge cases | ~30 | `do continue; while (true)` | Easy |
| new without parens | ~30 | `new Foo` (vs `new Foo()`) | Easy |
| Spread/rest patterns | ~30 | `(a, ...[]) => 1` | Medium |
| Arrow functions | ~15 | Edge cases | Easy |

**FAIL failures** (invalid JS we accept, ~379 tests):

| Pattern | Count | Example | Fix Difficulty |
|---------|-------|---------|----------------|
| Unterminated comment | 10 | `/*` without `*/` | Easy |
| Invalid LHS | ~50 | `3 = 4` | Medium |
| Return outside function | ~30 | `{ return; }` | Easy |
| Reserved word misuse | ~30 | `var enum = 1` | Medium |
| Regex edge cases | ~20 | `/42` | Medium |
| Invalid spread | ~40 | `[{a=0},...0]` | Medium |

**EARLY failures** (semantic errors we miss, ~140 tests):

| Pattern | Count | Example |
|---------|-------|---------|
| yield in non-generator | ~30 | `function a() { yield 1 }` |
| for-in/of duplicates | ~30 | `for (const {a, a} of 1);` |
| Labeled break/continue | ~15 | `a: continue a;` |

## Babel Failures (462 tests)

Babel tests focus on TypeScript, Flow, and JSX:

| Category | Count | Notes |
|----------|-------|-------|
| Flow | 252 | Flow type system features |
| TypeScript | 190 | TS-specific syntax |
| JSX | 20 | JSX edge cases |

### TypeScript Issues (190 tests)

| Subcategory | Count | Priority |
|-------------|-------|----------|
| class | 30 | High - modifiers, abstract, etc. |
| types | 24 | Medium - complex type syntax |
| cast | 21 | High - type assertions |
| import | 18 | High - import equals, type imports |
| static-blocks | 13 | Medium |
| type-arguments | 13 | High - generics parsing |
| arrow-function | 10 | High - generic arrows |
| declare | 10 | Medium |

### Flow Issues (252 tests)

Flow is lower priority but represents significant failures:
- Type annotations: 29
- Enum declarations: 20
- Typeapp-call: 16
- Typecasts: 16
- Iterator types: 15

## SWC Failures (167 tests)

SWC tests overlap significantly with Babel. Key areas:
- TypeScript features: ~100
- JSX: ~40
- JS edge cases: ~27

## Biome Failures (242 tests)

| Category | Count | Description |
|----------|-------|-------------|
| ok/ | 102 | Valid code we reject |
| error/ | 140 | Invalid code we accept |

### Semantic Validation Missing (error/ tests):

- `abstract_class_in_js.js` - Abstract classes in JS
- `await_in_non_async_function.js` - Await outside async
- `break_in_nested_function.js` - Break scope validation
- `return_stmt_err.js` - Return validation
- `class_constructor_parameter.js` - Constructor parameters
- Many TypeScript-specific semantic checks

---

## Priority Fixes

### Tier 1: High Impact, Medium Difficulty (~400 tests)

1. **Labeled Statements** (~100 tests)
   - Support `label:` prefix syntax (currently only `:label` Rust-style)
   - Fix break/continue with labels
   - Files: `block.rs`, `expression.rs`

2. **for-of/for-in improvements** (~100 tests)
   - Destructuring in for-in/of
   - Better `of` keyword recognition
   - Files: `block.rs`

3. **Generator/yield** (~70 tests)
   - Yield expression parsing
   - Generator function edge cases
   - Files: `function.rs`, `expression.rs`

4. **Type assertions (TypeScript)** (~50 tests)
   - `<Type>expr` syntax
   - `expr as Type` edge cases
   - Files: `type.rs`, `expression.rs`

### Tier 2: Medium Impact, Easy Difficulty (~150 tests)

5. **new without parens** (~30 tests)
   - `new Foo` should be valid
   - Files: `expression.rs`

6. **do-while edge cases** (~30 tests)
   - Single statement bodies
   - ASI handling
   - Files: `block.rs`

7. **Unterminated comment detection** (~10 tests)
   - Simple lexer fix
   - Files: `lex.rs`

8. **Return outside function** (~30 tests)
   - Semantic validation
   - Files: `block.rs` or semantic pass

### Tier 3: Invalid JS Detection (~200 tests)

9. **Invalid LHS validation** (~50 tests)
   - `3 = 4` should error
   - Complex assignment targets

10. **Reserved word validation** (~30 tests)
    - `var enum = 1` should error in strict mode

11. **Spread/rest validation** (~40 tests)
    - Invalid spread patterns

### Tier 4: Flow Support (Lower Priority, 252 tests)

Flow support is extensive work. Consider:
- Deferring until core JS/TS is solid
- Or implementing key features incrementally

---

## Quick Wins

| Fix | Tests | Effort | Files |
|-----|-------|--------|-------|
| Unterminated comment | 10 | 1h | lex.rs |
| new without parens | 30 | 2h | expression.rs |
| do-while ASI | 30 | 2h | block.rs |
| Return outside function | 30 | 2h | block.rs |
| **Total** | **100** | **~7h** | |

---

## Recommended Roadmap

### Phase 1: Core JS (Target: 85% test262)
1. Labeled statements
2. for-of/for-in improvements
3. Generator/yield fixes
4. Quick wins (above)

### Phase 2: TypeScript (Target: 80% babel TS)
1. Type assertions
2. Generic arrow functions
3. Import equals
4. Class modifiers

### Phase 3: Validation (Target: 90% fail tests)
1. Invalid LHS
2. Reserved words
3. Semantic checks

### Phase 4: Flow (Optional)
- Evaluate if Flow support is needed
- Consider plugin architecture
