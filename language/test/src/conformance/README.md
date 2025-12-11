# Conformance Test Status

> See [TESTING.md](../../../../TESTING.md) for the overall testing philosophy and strategy.

**Blended Conformance**: 80.5% (5961/7405 tests passing)

| Suite    | Passed | Failed | Total |  Rate   |
|:---------|-------:|-------:|------:|--------:|
| test262  |  4519  |   844  |  5363 |  84.3%  |
| babel    |   511  |   209  |   720 |  71.0%  |
| swc      |   522  |   163  |   685 |  76.2%  |
| biome    |   409  |   228  |   637 |  64.2%  |

**Note:** Flow support is intentionally excluded. We support TypeScript only.

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

---

## Key Missing Features & Misbehaviors

### 1. Static Initialization Blocks (~25 tests) - **HIGH PRIORITY**

Class static blocks (ES2022) are not supported. This is the single highest-impact fix.

```typescript
class Foo {
  static x: number;
  static {
    this.x = computeValue();  // static initialization block
  }
  static {}  // multiple blocks allowed
}
```

**Affected suites:** babel (~13), swc (~8), biome (~4)
**Files:** `class.rs`, AST definitions

### 2. Yield/Generator Issues (~75 tests) - **HIGH PRIORITY**

JavaScript generators have complex `yield` semantics we don't fully handle.

**yield without value (ASI):**
```javascript
function* a() { yield }      // valid - yields undefined
function* a() { yield *b }   // yield* delegation
function *a() { yield *yield } // yield* with yield as operand
```

**yield as identifier in generators (should reject):**
```javascript
function *g() { var yield; }        // INVALID - we accept
function *g(){ (yield) => 42 }      // INVALID - we accept
```

**Files:** `expression.rs` (yield parsing), `function.rs` (generator context)

### 3. TypeScript Import Equals (~18 tests) - **MEDIUM PRIORITY**

TypeScript-specific import syntax not supported:

```typescript
import A = B.C;              // import equals
import a = require("a");     // import require
export import A = B.C;       // export import equals
import type A = B.C;         // type import equals
```

**Files:** `import.rs`, AST definitions

### 4. Invalid JS We Accept (~383 fail tests)

The parser accepts many invalid JavaScript programs. Key categories:

| Pattern | Count | Example | Fix Type |
|---------|-------|---------|----------|
| **Yield semantic** | ~26 | `function *g() { var yield; }` | Parser context |
| **Octal strict mode** | ~11 | `'use strict'; 08` | Semantic pass |
| **Regex edge cases** | ~9 | `/42` | Lexer fix |
| **Reserved fn names** | ~4 | `function null() {}` | Parser validation |
| **Invalid LHS** | ~4 | `3 = 4`, `(1+1) = 10` | Parser validation |
| **Return outside fn** | ~3 | `{ return; }` | Parser context |
| **new.prop invalid** | ~6 | `new.prop` (only `new.target` valid) | Parser fix |
| **Rest not last** | ~20 | `[...x, y] = 0` | Parser validation |

### 5. TypeScript Class Features (~30 tests)

**accessor keyword (ES2022 auto-accessors):**
```typescript
class Foo {
  accessor prop: number = 1;
  static accessor prop2: number;
  abstract accessor prop3: number;
}
```

**Modifier ordering validation:**
```typescript
class Foo {
  override private foo: string;     // should error: wrong order
  readonly private foo2: string;    // should error: wrong order
}
```

### 6. TypeScript Type Assertions (~21 tests)

```typescript
<T>() => {};           // type assertion in arrow (TSX ambiguity)
(a as T) => {};        // as in arrow param (invalid)
<number> 1;            // type assertion expression
(<number>x) = null     // type assertion + assignment
```

### 7. JSX Edge Cases (~24 tests)

```jsx
<a:b />               // namespaced tags (not supported)
<!-- comment -->      // HTML comments in JSX
&entity;              // HTML entity handling
```

---

## Test262 Failures (844 tests)

| Category | Description | Count |
|----------|-------------|-------|
| pass | Valid JS we reject | 211 |
| pass-explicit | Valid modules we reject | 156 |
| fail | Invalid JS we accept | 383 |
| early | Semantic errors we miss | 94 |

### Pass Failures (367 tests) - Valid JS We Reject

| Pattern | Count | Example |
|---------|-------|---------|
| yield/yield* | ~49 | `yield *a`, ASI with yield |
| for-in/of destructuring | ~17 | `for([a,b[a]] in 3);` |
| Spread/rest patterns | ~30 | `(...[]) => 1` |
| class extends | ~14 | `class extends b {}` |

### Fail Failures (383 tests) - Invalid JS We Accept

Most require parser-level validation or semantic analysis:
- Yield context validation (~26)
- Strict mode restrictions (~40)
- LHS validation (~50)
- Control flow scope (~15)

### Early Failures (94 tests) - Semantic Errors

These parse correctly but should fail semantic analysis:
- yield in non-generator (~20)
- Duplicate bindings (~15)
- Labeled break/continue scope (~10)

---

## Babel Failures (209 tests)

| Category | Count | Priority |
|----------|-------|----------|
| static-blocks | 13 | **High** |
| import equals | 18 | **High** |
| class features | 30 | Medium |
| cast/assertions | 21 | Medium |
| type-arguments | 13 | Medium |
| arrow-function | 10 | Medium |
| JSX | 24 | Low |

---

## Biome Failures (228 tests)

| Category | Count | Description |
|----------|-------|-------------|
| error/ | 147 | Invalid code we accept |
| ok/ | 81 | Valid code we reject |

Key error/ patterns we should reject:
- `abstract class` in JS files
- `await` outside async function
- `break`/`continue` in nested functions
- TypeScript modifiers in wrong context

---

## Roadmap

### Phase 1: High-Impact Parsing Fixes

| Feature | Tests | Complexity |
|---------|-------|------------|
| **Static blocks** | ~25 | Medium |
| **yield/yield*** | ~49 | Medium |
| **Import equals** | ~18 | Low |

### Phase 2: Validation & TypeScript

| Feature | Tests | Notes |
|---------|-------|-------|
| accessor keyword | ~15 | ES2022 auto-accessors |
| Type assertions | ~25 | `<Type>expr` edge cases |
| Invalid LHS | ~50 | `3 = 4` validation |
| Reserved words | ~30 | Function names, imports |

### Phase 3: Semantic Analysis Pass

Adding a compile/semantic step to conformance tests would catch:
- yield/await context errors (~50)
- Strict mode violations (~40)
- Duplicate binding detection (~30)
- Control flow scope validation (~20)

```rust
// Current: parse only
let outcome = parse_file(&path, &content, file_type, options);

// Future: parse + semantic analysis
let module = parse_file(&path, &content, file_type, options)?;
let outcome = analyze_semantics(&module, analysis_options);
```

---

