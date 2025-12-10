# Conformance Test Status

**Blended Conformance**: 79.9% (5918/7405 tests passing)

| Suite    | Passed | Failed | Total |  Rate   |
|:---------|-------:|-------:|------:|--------:|
| test262  |  4480  |   883  |  5363 |  83.5%  |
| babel    |   516  |   204  |   720 |  71.7%  |
| swc      |   523  |   162  |   685 |  76.4%  |
| biome    |   399  |   238  |   637 |  62.6%  |

## Recent Progress

### Completed Fixes (2025-12)

| Fix                          | Tests Fixed | Files Changed                    |
|------------------------------|:-----------:|----------------------------------|
| do-while single statements   |     ~30     | `block.rs`, `loop.rs`            |
| new without parentheses      |     ~30     | `call.rs`                        |
| Unterminated block comment   |     ~10     | `lex.rs`                         |
| for/while single statements  |     ~47     | `loop.rs`                        |
| Empty statement in loops     |    ~157     | `block.rs`                       |
| **Labeled statements**       |     ~83     | AST, Parser, DIR, Compiler       |
| **break/continue labels**    |      ~3     | Biome tests fixed                |
| **Invalid escape sequences** |     ~10     | `lex.rs`, `token.rs`             |
| **Total**                    |  **~297**   |                                  |

**Details:**
- `do stmt; while(cond)` now works without requiring braces
- `new Foo` is now valid (parentheses optional)
- `/*` without `*/` now correctly produces an error token
- `for (x in y) stmt;` and `while (x) stmt;` now work without braces
- `for (x of y);` with empty statement body now works
- `label: stmt` labeled statements now work
- `break label` and `continue label` now work (JS-style without colon)
- `PatternField::Elision` added for array elision patterns
- `'\8'`, `'\9'` now correctly rejected as invalid escapes
- Octal escapes in template literals (`` `\07` ``) now rejected

**Note:** Some semantic restrictions (lexical declarations in single-statement context, `this` in for-of, etc.) are deferred to the semantic analysis pass.

### Next Up

The following fixes are prioritized by impact and difficulty. Work in this order:

#### 1. ~~Invalid Escape Sequences~~ ✅ Done

Basic validation implemented: `\8`, `\9`, and octal escapes in template literals now rejected.
Remaining: strict mode octal validation, regex unicode escape validation (deferred to semantic pass).

#### 2. Generator/Yield Issues (~60 tests across suites)

**Problem**: JavaScript generators have complex `yield` semantics we don't fully handle.

**yield without value** (ASI complexity):
```javascript
function* a() { yield }      // valid - yields undefined
function* a() {
    yield      // yields undefined (ASI inserts semicolon)
    foo()      // separate statement
}
function* a() { yield foo() }  // yields result of foo()
```

**yield* delegation**:
```javascript
function* a() { yield* b }   // delegates to another iterator
function* a() { yield *b }   // same thing (space before *)
```

**yield in nested contexts**:
```javascript
function* a() { ({get b(){ yield }}) }  // yield inside getter inside generator
function* a() { (class extends (yield) {}) }  // yield as superclass expression
```

**Files**: `expression.rs` (yield parsing), `function.rs` (generator context tracking)

#### 3. TypeScript Class Features (~50 tests)

**Problem**: TypeScript classes have many modifiers and features beyond standard JS.

**Modifiers and ordering**:
```typescript
class Foo {
    public static readonly x: number;     // multiple modifiers
    private abstract foo(): void;         // abstract methods
    protected override bar() {}           // override keyword
    accessor myProp: string;              // auto-accessor (ES2022+)
}
```

**Static blocks** (ES2022+):
```typescript
class Foo {
    static x: number;
    static {
        this.x = computeValue();  // static initialization block
    }
}
```

**Parameter properties**:
```typescript
class Foo {
    constructor(public x: number, private y: string) {}
}
```

**Files**: `class.rs`, `function.rs` (for constructor parameter properties)

#### Future Work

| Feature | Tests | Required Changes |
|---------|-------|------------------|
| Reserved words as keys | ~20 | Allow keywords in property names |
| Spread/rest patterns | ~30 | Parser updates |
| TypeScript imports | ~25 | `import =` syntax, `import type` |
| Type assertions | ~25 | `<Type>expr`, `as` edge cases |

## Test262 Failures (883 tests)

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

## Babel Failures (204 tests)

Babel tests focus on TypeScript and JSX (Flow is intentionally excluded):

| Category | Count | Notes |
|----------|-------|-------|
| TypeScript | ~180 | TS-specific syntax |
| JSX | ~24 | JSX edge cases |

### TypeScript Issues (~180 tests)

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

## SWC Failures (162 tests)

SWC tests overlap significantly with Babel. Key areas:
- TypeScript features: ~100
- JSX: ~40
- JS edge cases: ~27

## Biome Failures (238 tests)

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

## Completed Fixes

| Fix | Tests | Effort | Files |
|-----|-------|--------|-------|
| Unterminated comment | 10 | 1h | lex.rs |
| new without parens | 30 | 2h | call.rs |
| do-while single stmt | 30 | 2h | block.rs, loop.rs |
| for/while single stmt | ~47 | 1h | loop.rs |
| Empty statement loops | ~157 | 0.5h | block.rs |
| Labeled statements | ~83 | 4h | AST, Parser, DIR, Compiler |
| **Total** | **~287** | | |

---

## Recommended Roadmap

Work in this order (see "Next Up" section above for details):

### Phase 1: Validation & Core JS (Current)
1. ✅ Labeled statements - Done
2. ✅ Loop single statements - Done
3. **Invalid escape sequences** (~39 tests) - lexer validation
4. **Generator/yield** (~60 tests) - ASI handling, yield* delegation

### Phase 2: TypeScript Features
1. **TypeScript class features** (~50 tests) - modifiers, static blocks
2. TypeScript imports (~25 tests) - `import =` syntax
3. Type assertions (~25 tests) - `<Type>expr`, `as` edge cases
4. Generic arrow functions (~10 tests)

### Phase 3: Remaining Validation
1. Invalid LHS (`3 = 4`) - ~50 tests
2. Reserved word validation - ~30 tests
3. Spread/rest validation - ~30 tests
4. Return outside function - semantic pass

**Note:** Flow support is intentionally excluded. We support TypeScript only.
