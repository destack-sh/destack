# linter

Static analysis rules for Destack, TypeScript, and JavaScript.
Lints run at different IR levels (AST, DIR, MIR), usually per module.

## Overview

The linter is a separate crate from the compiler, but deeply integrated.
It runs directly on the same IRs and takes advantage of the same task parallelization.
Rules can operate on AST (syntax patterns), DIR (typed IR), or MIR (low-level IR).

We draw inspiration from linters across the ecosystem: ESLint, TypeScript-ESLint, Biome, Clippy, Ruff, SonarQube, Semgrep.
Each rule notes its source if following an established rule (we also try to keep the name, severity, and fixability the same).

## Architecture

Rules implement the `LintRule` trait:

```ds
interface LintRule {
    meta(): LintMeta
    checkModuleAst(severity: LintSeverity, ctx: LintModuleAstContext): void
    checkModuleDir(severity: LintSeverity, ctx: LintModuleDirContext): void
    // ...
}
```

The `LintRunner` orchestrates execution, filtering rules by their level (AST/DIR/MIR) and checking configuration.
Rules can provide automatic fixes, which the runner collects alongside diagnostics.

Rules are declared using the `declare_lint!` macro, which generates the boilerplate:

```ds
@lint({
    category: Correctness,
    level: Dir,
    requiresAll: [],
    requiresAny: [],
    fixable: No,
})
/// Disallow `delete` on arrays (creates holes).
class NoArrayDelete implements LintRule { ... }
```

## Categories

Rules are organized into categories, each with a letter code for diagnostic IDs:

| Category | Code | Default | Description |
|----------|------|---------|-------------|
| [Complexity](#complexity-x) | `X` | Warning | Overly complex code |
| [Correctness](#correctness-c) | `C` | Error | Likely bugs and logic errors |
| [Performance](#performance-p) | `P` | Warning | Inefficient patterns |
| [Restriction](#restriction-r) | `R` | Off | Project-specific restrictions (opt-in) |
| [Security](#security-s) | `S` | Error | Potential vulnerabilities |
| [Style](#style-y) | `Y` | Warning | Consistent coding style |
| [Suspicious](#suspicious-u) | `U` | Warning | Code that is likely unintentional |

Lint codes are stable identifiers, and the tables are sorted alphabetically by rule name.
Gaps are expected, and new rules should use the next available code within their category.

---

## Compiler vs Linter

We have a full compiler with its own analysis, verification, and diagnostics.
Checks that are fundamental to correct compilation are handled by the **compiler** rather than the linter:

| Check | Owner | Reasoning |
|-------|-------|-----------|
| Type mismatches | Compiler | Fundamental type system |
| Unbound symbols | Compiler | Required for compilation |
| Conflicting symbols | Compiler | Required for compilation |
| Unreachable code | Compiler | CFG analysis for codegen |
| Precision loss | Compiler | Numeric type semantics |
| Pattern exhaustiveness | Compiler | Required for correctness |
| Ownership violations | Compiler | Enforced in strict borrow mode (`borrowMode: "strict"`) |

---

## Correctness (C)

High-confidence issues that are almost always wrong.

[`src/rules/correctness/`](src/rules/correctness/)

| Code | Rule | Source | Level | Status | Fixability | Description |
|------|------|--------|-------|--------|------------|-------------|
| `LC045` | `await-holding-lock` | Clippy | DIR |  | None | Disallow holding a mutex lock across an await point |
| `LC001` | `for-direction` | ESLint | AST | ✓ | Unsafe | Enforce for loop update clause moving in the correct direction |
| `LC060` | `improper-ctypes` | Rust | DIR |  | None | Disallow FFI signatures with ABI-unsafe types |
| `LC002` | `no-approx-constant` | Destack | AST | ✓ | Safe | Disallow approximate representations of mathematical constants |
| `LC046` | `no-arguments-order-mismatch` | SonarQube | DIR |  | Suggestion | Disallow arguments that appear swapped based on parameter names |
| `LC003` | `no-array-constructor` | ESLint | DIR | ✓ | Safe | Disallow `new Array()` (confusing behavior) |
| `LC004` | `no-array-delete` | TS-ESLint | DIR | ✓ | Unsafe | Disallow `delete` on arrays (creates holes) |
| `LC005` | `no-async-promise-executor` | ESLint | DIR | ✓ | None | Disallow async functions as Promise executor |
| `LC006` | `no-base-to-string` | TS-ESLint | DIR | ✓ | None | Disallow `.toString()` on objects without useful representation |
| `LC007` | `no-compare-neg-zero` | ESLint | AST | ✓ | Safe | Disallow comparing against negative zero |
| `LC068` | `no-confusing-void-expression` | TS-ESLint | DIR |  | Suggestion | Disallow `void` expressions in positions where values are expected |
| `LC008` | `no-constant-binary-expression` | ESLint | AST | ✓ | None | Disallow expressions where the operation doesn't affect the value |
| `LC009` | `no-constant-condition` | ESLint | AST | ✓ | None | Disallow constant expressions in conditions |
| `LC010` | `no-control-regex` | ESLint | AST | ✓ | None | Disallow control characters in regular expressions |
| `LC011` | `no-deprecated` | TS-ESLint | DIR | ✓ | None | Disallow use of `@deprecated` APIs |
| `LC012` | `no-duplicate-case` | ESLint | AST | ✓ | None | Disallow duplicate case labels |
| `LC015` | `no-floating-point-equality` | Clippy | DIR | ✓ | None | Disallow direct `==` comparison of floats |
| `LC013` | `no-empty-range` | Destack | AST | ✓ | Unsafe | Disallow empty ranges where start > end |
| `LC014` | `no-fallthrough` | ESLint | AST | ✓ | Suggestion | Disallow fallthrough of case statements |
| `LC058` | `no-fast-math-sensitive-ops` | Destack | DIR |  | Suggestion | Warn when float math semantics require NaN, inf, or signed zero |
| `LC016` | `no-floating-promises` | TS-ESLint | DIR | ✓ | Suggestion | Require Promises to be awaited or returned |
| `LC017` | `no-for-in-array` | TS-ESLint | DIR | ✓ | None | Disallow iterating over arrays with for-in |
| `LC018` | `no-implicit-any` | TypeScript | DIR | ✓ | Suggestion | Disallow implicit `any` types |
| `LC047` | `no-infinite-iterator` | Destack | DIR |  | None | Disallow using methods that produce infinite iterators |
| `LC019` | `no-infinite-recursion` | ErrorProne | DIR | ✓ | None | Disallow functions that unconditionally call themselves |
| `LC020` | `no-invalid-regexp` | ESLint | AST | ✓ | None | Disallow invalid regular expression strings |
| `LC021` | `no-iterator-invalidation` | Destack | DIR | ✓ | None | Disallow modifying a collection while iterating over it |
| `LC022` | `no-loop-single-iteration` | SonarQube | AST | ✓ | None | Disallow loops that execute at most once |
| `LC023` | `no-misused-promises` | TS-ESLint | DIR | ✓ | None | Disallow Promises in places not designed to handle them |
| `LC048` | `no-overlapping-match-arms` | Destack | DIR |  | Safe | Disallow match patterns that subsume later arms |
| `LC024` | `no-promise-executor-return` | ESLint | DIR | ✓ | Safe | Disallow returning values from Promise executor |
| `LC025` | `no-self-compare` | ESLint | DIR | ✓ | None | Disallow comparisons where both sides are exactly the same |
| `LC026` | `no-sparse-arrays` | ESLint | AST | ✓ | Safe | Disallow sparse arrays with holes |
| `LC027` | `no-struct-identity-compare` | Destack | DIR | ✓ | None | Disallow identity comparison on value types |
| `LC028` | `no-throw-in-result-function` | Destack | DIR | ✓ | None | Disallow `throw` in functions returning `Result` |
| `LC031` | `no-unnecessary-type-arguments` | TS-ESLint | DIR | ✓ | Safe | Disallow type arguments that equal the default |
| `LC032` | `no-unnecessary-type-assertion` | TS-ESLint | DIR | ✓ | Safe | Disallow type assertions that do not change the type |
| `LC033` | `no-unsafe-finally` | ESLint | AST | ✓ | None | Disallow control flow statements in finally blocks |
| `LC034` | `no-unsafe-negation` | ESLint | AST | ✓ | Safe | Disallow negating the left operand of relational operators |
| `LC029` | `no-unknown-rule-decorator` | Destack | AST | ✓ | None | Disallow unknown rule decorators |
| `LC030` | `no-unnecessary-condition` | TS-ESLint | DIR | ✓ | None | Disallow conditions that are always truthy, always falsy, or never nullish |
| `LC035` | `no-unused-imports` | Destack | DIR | ✓ | Safe | Disallow unused import bindings |
| `LC036` | `no-unused-parameters` | Destack | DIR | ✓ | Safe | Disallow unused function and method parameters |
| `LC037` | `no-unused-private-class-members` | Destack | DIR | ✓ | Unsafe | Disallow unused private class members |
| `LC038` | `no-useless-assignment` | ESLint | DIR | ✓ | None | Disallow assignments that are immediately overwritten |
| `LC039` | `no-useless-increment` | SonarQube | DIR | ✓ | Safe | Disallow incrementing a value that is never used afterward |
| `LC040` | `require-array-sort-compare` | TS-ESLint | DIR | ✓ | None | Require comparison function for `.sort()` |
| `LC041` | `unbound-method` | TS-ESLint | DIR | ✓ | None | Disallow unbound methods as callbacks |
| `LC042` | `unused-must-use` | Destack | DIR | ✓ | Suggestion | Disallow ignoring return values of `@mustUse` functions |
| `LC043` | `use-isnan` | ESLint | AST | ✓ | Safe | Require `Number.isNaN()` instead of comparisons with `NaN` |
| `LC044` | `use-unknown-in-catch-callback-variable` | TS-ESLint | DIR | ✓ | Safe | Require `unknown` for catch callback variables instead of `any` |

## Suspicious (U)

Code that is likely unintentional but may occasionally be intentional.

[`src/rules/suspicious/`](src/rules/suspicious/)

| Code | Rule | Source | Level | Status | Fixability | Description |
|------|------|--------|-------|--------|------------|-------------|
| `LU001` | `guard-for-in` | ESLint | AST | ✓ | None | Require `hasOwnProperty` guard in for-in loops |
| `LU002` | `no-async-foreach` | Destack | DIR | ✓ | None | Disallow `forEach` with async callback (doesn't await) |
| `LU003` | `no-cond-assign` | ESLint | AST | ✓ | Suggestion | Disallow assignment operators in conditional expressions |
| `LU004` | `no-confusing-assignment` | Destack | AST | ✓ | Suggestion | Warn on assignments that look like comparisons |
| `LU005` | `no-confusing-non-null-assertion` | TS-ESLint | AST | ✓ | Safe | Disallow non-null assertions after optional chain expressions |
| `LU006` | `no-constant-assertion` | Destack | AST | ✓ | None | Disallow assertions on constant values |
| `LU007` | `no-constructor-return` | ESLint | AST | ✓ | Safe | Disallow returning values from constructors |
| `LU008` | `no-debugger` | ESLint | AST | ✓ | Safe | Disallow debugger statements |
| `LU010` | `no-duplicate-else-if` | ESLint | AST | ✓ | None | Disallow duplicate conditions in if-else-if chains |
| `LU009` | `no-duplicate-decorators` | Destack | AST | ✓ | Always | Disallow identical decorators (same name and arguments) |
| `LU011` | `no-duplicate-match-arms` | Destack | AST | ✓ | None | Warn on match arms with identical bodies |
| `LU012` | `no-empty` | ESLint | AST | ✓ | Suggestion | Disallow empty block statements |
| `LU013` | `no-empty-function` | ESLint | AST | ✓ | Suggestion | Disallow empty functions |
| `LU014` | `no-empty-pattern` | ESLint | AST | ✓ | None | Disallow empty destructuring patterns |
| `LU015` | `no-empty-static-block` | ESLint | AST | ✓ | Safe | Disallow empty static initialization blocks in classes |
| `LU016` | `no-ex-assign` | ESLint | AST | ✓ | None | Disallow reassigning exceptions in catch clauses |
| `LU017` | `no-extra-non-null-assertion` | TS-ESLint | AST | ✓ | Safe | Disallow extra non-null assertions |
| `LU018` | `no-identical-branches` | SonarQube | AST | ✓ | None | Warn when all branches of if/switch have identical bodies |
| `LU019` | `no-incomplete-range` | Destack | AST | ✓ | None | Warn on exclusive ranges that are likely meant to be inclusive |
| `LU020` | `no-inner-declarations` | ESLint | AST | ✓ | None | Disallow variable or function declarations in nested blocks |
| `LU021` | `no-large-try-block` | DeepSource | AST | ✓ | None | Warn when try block contains much more than throwing code |
| `LU046` | `no-loop-func` | ESLint | DIR |  | None | Disallow functions that capture loop variables |
| `LU047` | `no-method-shadowing` | Destack | DIR |  | None | Warn when a method shadows an inherited method |
| `LU023` | `no-mixed-key-types` | Destack | DIR | ✓ | None | Warn on objects that mix string, symbol, and numeric keys |
| `LU022` | `no-misleading-character-class` | ESLint | AST | ✓ | None | Disallow characters that behave unexpectedly in regex |
| `LU048` | `no-missing-override` | ErrorProne | DIR |  | Safe | Warn when method overrides parent without `override` keyword |
| `LU024` | `no-negation-in-equality-check` | Unicorn | AST | ✓ | Safe | Disallow negation in the left operand of equality tests |
| `LU049` | `no-pointer-arithmetic` | Destack | DIR |  | Suggestion | Warn on pointer arithmetic without explicit offset helpers |
| `LU050` | `no-pointer-comparison` | Destack | DIR |  | Suggestion | Warn on pointer comparisons outside explicit address checks |
| `LU025` | `no-redundant-match-guard` | Destack | AST | ✓ | Safe | Disallow match guards that are always true or false |
| `LU026` | `no-redundant-pattern` | Destack | AST | ✓ | None | Disallow patterns that bind nothing useful |
| `LU027` | `no-return-assign` | ESLint | AST | ✓ | Unsafe | Disallow assignment operators in return statements |
| `LU028` | `no-self-assign` | ESLint | AST | ✓ | Safe | Disallow assignments where both sides are exactly the same |
| `LU029` | `no-shadow-restricted-names` | ESLint | AST | ✓ | None | Disallow shadowing of restricted or builtin names |
| `LU067` | `no-shadowed-mutable` | Clippy | DIR |  | Suggestion | Warn on mutable shadowing that obscures prior bindings |
| `LU030` | `no-single-element-tuple` | Destack | AST | ✓ | Safe | Warn on single-element tuples that may be accidental |
| `LU031` | `no-template-curly-in-string` | ESLint | AST | ✓ | Safe | Disallow template literal placeholder syntax in regular strings |
| `LU032` | `no-throw-literal` | ESLint | DIR | ✓ | Suggestion | Disallow throwing literals instead of Error objects |
| `LU051` | `no-unnecessary-clone` | TS-ESLint | DIR |  | Safe | Warn on cloning values that are not used afterward |
| `LU068` | `no-unstable-iteration-order` | Go | DIR |  | Suggestion | Warn on map iteration order where determinism is required |
| `LU033` | `no-unused-except-recursion` | Destack | DIR | ✓ | None | Warn on function arguments only used for recursion |
| `LU034` | `no-useless-backreference` | ESLint | AST | ✓ | None | Disallow useless backreferences in regular expressions |
| `LU052` | `no-useless-cast` | Clippy | DIR |  | Safe | Disallow casts that do not change the type |
| `LU035` | `no-useless-catch` | ESLint | AST | ✓ | Safe | Disallow catch clauses that only rethrow |
| `LU036` | `no-useless-computed-key` | ESLint | AST | ✓ | Safe | Disallow unnecessary computed property keys |
| `LU037` | `no-useless-concat` | ESLint | AST | ✓ | Safe | Disallow unnecessary concatenation of literals |
| `LU038` | `no-useless-constructor` | ESLint | AST | ✓ | Safe | Disallow unnecessary constructors |
| `LU039` | `no-useless-escape` | ESLint | AST | ✓ | Safe | Disallow unnecessary escape characters |
| `LU040` | `no-useless-rename` | ESLint | AST | ✓ | Safe | Disallow renaming imports/exports to the same name |
| `LU041` | `no-useless-return` | ESLint | AST | ✓ | Safe | Disallow redundant return statements |
| `LU042` | `require-await` | TS-ESLint | AST | ✓ | Safe | Disallow async functions with no await expressions |
| `LU043` | `require-else-in-if-chain` | Destack | AST | ✓ | None | Require final else in if-else-if chains |
| `LU044` | `require-yield` | ESLint | AST | ✓ | None | Require generator functions to contain yield |
| `LU045` | `return-await` | TS-ESLint | DIR | ✓ | Safe | Enforce consistent `return await` usage |

## Security (S)

Patterns that may expose the application to attacks.

[`src/rules/security/`](src/rules/security/)

| Code | Rule | Source | Level | Status | Fixability | Description |
|------|------|--------|-------|--------|------------|-------------|
| `LS001` | `no-blank-target` | Biome | AST | ✓ | None | Disallow `target="_blank"` without `rel="noopener"` |
| `LS018` | `no-command-injection` | Semgrep | DIR |  | None | Disallow command execution using untrusted input |
| `LS016` | `no-ffi-abi-mismatch` | Rust | DIR |  | None | Disallow FFI calls with ABI-unsafe layouts |
| `LS002` | `no-hardcoded-ip` | SonarQube | AST | ✓ | None | Disallow hardcoded IP addresses |
| `LS003` | `no-implied-eval` | ESLint | DIR | ✓ | None | Disallow `setTimeout` and `setInterval` with string arguments |
| `LS019` | `no-insecure-deserialization` | Semgrep | DIR |  | None | Disallow deserialization of untrusted data without validation |
| `LS004` | `no-insecure-random` | Semgrep | DIR | ✓ | None | Disallow insecure random number generators |
| `LS005` | `no-open-redirect` | Semgrep | DIR | ✓ | None | Disallow tainted values in browser redirect APIs |
| `LS020` | `no-path-traversal` | Semgrep | DIR |  | None | Disallow file system path construction from untrusted input |
| `LS006` | `no-prototype-pollution` | Semgrep | DIR | ✓ | None | Disallow patterns that may pollute Object.prototype |
| `LS007` | `no-regex-injection` | Destack | DIR | ✓ | None | Disallow tainted values in dynamic regular expression patterns |
| `LS008` | `no-script-url` | ESLint | AST | ✓ | None | Disallow `javascript:` URLs |
| `LS009` | `no-secrets` | Biome | AST | ✓ | None | Disallow hardcoded secrets and credentials |
| `LS021` | `no-sql-injection` | Semgrep | DIR |  | None | Disallow query construction that interpolates untrusted input |
| `LS022` | `no-ssrf` | Semgrep | DIR |  | None | Disallow network requests to attacker controlled destinations |
| `LS010` | `no-tainted-sink` | Destack | DIR | ✓ | None | Disallow passing tainted values into security sinks |
| `LS023` | `no-template-injection` | Semgrep | DIR |  | None | Disallow rendering templates with untrusted template text |
| `LS017` | `no-unsafe-decorator` | Destack | DIR |  | Suggestion | Disallow decorators with unsafe side effects |
| `LS011` | `no-weak-crypto` | Semgrep | DIR | ✓ | None | Disallow weak cryptographic algorithms |

## Performance (P)

Correct code that could be faster or use less memory.

[`src/rules/performance/`](src/rules/performance/)

| Code | Rule | Source | Level | Status | Fixability | Description |
|------|------|--------|-------|--------|------------|-------------|
| `LP027` | `large-stack-arrays` | Clippy | MIR |  | Suggestion | Warn on large stack allocations that should be heap allocated |
| `LP028` | `large-types-passed-by-value` | Clippy | DIR |  | Suggestion | Warn on passing large structs or arrays by value |
| `LP001` | `no-accumulating-spread` | Biome | DIR | ✓ | None | Disallow spreading in accumulators (causes O(n²) allocations) |
| `LP033` | `no-ambiguous-type` | Destack | DIR |  | Suggestion | Warn on types that force dynamic dispatch unnecessarily |
| `LP019` | `no-alloc-in-loop` | Clippy | MIR |  | None | Disallow heap allocations inside loops |
| `LP002` | `no-array-for-each` | Unicorn | DIR | ✓ | None | Prefer for-of over `Array.forEach()` |
| `LP003` | `no-array-unshift-loop` | Destack | DIR | ✓ | None | Disallow `unshift` in loops (causes O(n²) reallocations) |
| `LP004` | `no-await-in-loop` | ESLint | AST | ✓ | None | Disallow await inside of loops |
| `LP005` | `no-barrel-file` | Biome | AST | ✓ | None | Disallow barrel files that re-export everything |
| `LP029` | `no-blocking-in-async` | Destack | DIR |  | Suggestion | Disallow blocking calls inside async functions |
| `LP025` | `no-clone-in-loop` | Clippy | DIR |  | Suggestion | Warn on cloning inside loops |
| `LP030` | `no-devirtualization-blockers` | Destack | MIR |  | Suggestion | Warn when dynamic dispatch blocks devirtualization in hot paths |
| `LP031` | `no-escape-to-heap` | Destack | MIR |  | Suggestion | Warn when values escape and force heap allocation |
| `LP034` | `no-excessive-reflection` | Destack | DIR |  | Suggestion | Warn on heavy runtime reflection in hot paths |
| `LP035` | `no-implicit-boxing` | Destack | DIR |  | Suggestion | Warn on implicit boxing or interface erasure allocations |
| `LP006` | `no-json-clone` | Destack | DIR | ✓ | Unsafe | Disallow `JSON.parse(JSON.stringify())` for cloning |
| `LP007` | `no-nested-array-includes` | Destack | DIR | ✓ | None | Disallow `includes`/`indexOf` inside loops over another array |
| `LP008` | `no-object-spread-in-reduce` | Destack | DIR | ✓ | None | Disallow object spread in reduce accumulators |
| `LP009` | `no-regex-in-loop` | Destack | DIR | ✓ | None | Disallow `new RegExp()` inside loops |
| `LP020` | `no-sequential-independent-await` | Destack | DIR |  | Safe | Suggest `Promise.all` for independent sequential awaits |
| `LP010` | `no-string-concat-in-loop` | Destack | DIR | ✓ | None | Disallow `+=` and `x = x + y` string concatenation in loops |
| `LP011` | `no-super-linear-regex` | Destack | AST | ✓ | None | Disallow regular expressions with catastrophic backtracking |
| `LP036` | `no-vectorization-blocker` | Destack | MIR |  | Suggestion | Warn on aliasing patterns that block vectorization |
| `LP026` | `no-virtual-call-in-loop` | Destack | DIR |  | Suggestion | Warn on dynamic dispatch inside loops |
| `LP032` | `non-canonical-loop` | Destack | MIR |  | Suggestion | Warn on loop forms that block canonical loop optimizations |
| `LP012` | `prefer-array-every` | Unicorn | DIR | ✓ | Safe | Prefer `.every()` over `.filter().length === .length` |
| `LP013` | `prefer-array-literal` | Destack | DIR | ✓ | None | Suggest using array literal instead of empty array followed by extend |
| `LP014` | `prefer-for-of` | TS-ESLint | DIR | ✓ | None | Prefer for-of loops over index-based for loops |
| `LP015` | `prefer-includes` | TS-ESLint | DIR | ✓ | Safe | Prefer `.includes()` over `.indexOf() !== -1` |
| `LP021` | `prefer-reserve` | Clippy | DIR |  | Suggestion | Prefer reserving capacity when the size is known |
| `LP016` | `prefer-string-endswith` | Unicorn | DIR | ✓ | Safe | Prefer `.endsWith()` over `.slice(-n) === suffix` |
| `LP017` | `prefer-string-startswith` | Unicorn | DIR | ✓ | Safe | Prefer `.startsWith()` over `.indexOf() === 0` |
| `LP018` | `require-unicode-regexp` | ESLint | AST | ✓ | None | Require `u` or `v` flag on regular expressions |

## Style (Y)

Subjective preferences for consistent coding style.

[`src/rules/style/`](src/rules/style/)

| Code | Rule | Source | Level | Status | Fixability | Description |
|------|------|--------|-------|--------|------------|-------------|
| `LY070` | `array-type` | TS-ESLint | DIR |  | Safe | Require consistently using either `T[]` or `Array<T>` |
| `LY001` | `catch-error-name` | Unicorn | AST | ✓ | Safe | Enforce a specific name for catch clause error parameters |
| `LY002` | `comment-casing` | Destack | AST | ✓ | None | Enforce comment / doc casing |
| `LY003` | `comment-layout` | Destack | AST | ✓ | None | Enforce comment / doc layout |
| `LY004` | `comment-punctuation` | Destack | AST | ✓ | None | Enforce comment / doc punctuation style |
| `LY005` | `consistent-extension-style` | Destack | AST | ✓ | None | Enforce consistent use of named or anonymous extensions |
| `LY006` | `consistent-type-definitions` | TS-ESLint | AST | ✓ | None | Enforce type definitions to use either `interface` or `type` |
| `LY007` | `consistent-type-imports` | TS-ESLint | AST | ✓ | Safe | Enforce consistent usage of type imports |
| `LY008` | `default-param-last` | ESLint | AST | ✓ | None | Enforce default parameters to be last |
| `LY009` | `dot-notation` | ESLint | AST | ✓ | Safe | Enforce dot notation whenever possible |
| `LY010` | `eqeqeq` | ESLint | AST | ✓ | Safe | Require `===` and `!==` |
| `LY011` | `explicit-function-return-type` | TS-ESLint | AST | ✓ | None | Require explicit return types on functions |
| `LY071` | `explicit-module-boundary-types` | TS-ESLint | AST |  | Suggestion | Require explicit types on exported APIs |
| `LY012` | `filename-case` | Unicorn | AST | ✓ | None | Enforce a case style for filenames |
| `LY013` | `grouped-accessor-pairs` | ESLint | AST | ✓ | None | Require grouped accessor pairs in object literals and classes |
| `LY088` | `missing-docs` | Rust | AST |  | Suggestion | Require documentation comments on public items |
| `LY014` | `no-boolean-literal-compare` | Unicorn | AST | ✓ | Safe | Disallow comparing boolean expressions to boolean literals |
| `LY015` | `no-collapsible-if` | Unicorn | AST | ✓ | Safe | Suggest merging nested if statements without else |
| `LY016` | `no-duplicate-string` | SonarQube | AST | ✓ | None | Disallow the same string literal appearing many times |
| `LY072` | `no-duplicate-type-constituents` | TS-ESLint | AST |  | Safe | Disallow duplicate constituents in union/intersection types |
| `LY017` | `no-else-return` | ESLint | AST | ✓ | Safe | Disallow else blocks after return statements |
| `LY018` | `no-empty-interface` | TS-ESLint | AST | ✓ | Safe | Disallow empty interfaces |
| `LY067` | `no-extra-boolean-cast` | ESLint | DIR | ✓ | Safe | Disallow unnecessary boolean casts |
| `LY073` | `no-implicit-coercion` | ESLint | DIR |  | Safe | Disallow shorthand type conversions |
| `LY019` | `no-lonely-if` | ESLint | AST | ✓ | Safe | Disallow if statements as the only statement in else blocks |
| `LY020` | `no-negated-condition` | ESLint | AST | ✓ | Safe | Disallow negated conditions with else branches |
| `LY074` | `no-redundant-type-constituents` | TS-ESLint | DIR |  | Safe | Disallow type constituents made redundant by others |
| `LY021` | `no-nested-template-literal` | SonarQube | AST | ✓ | None | Disallow template literals nested inside template literals |
| `LY022` | `no-nested-ternary` | ESLint | AST | ✓ | None | Disallow nested ternary expressions |
| `LY023` | `no-object-constructor` | ESLint | DIR | ✓ | Safe | Disallow `new Object()` |
| `LY025` | `no-unneeded-ternary` | ESLint | AST | ✓ | Safe | Disallow ternary operators when simpler alternatives exist |
| `LY068` | `no-unnecessary-template-expression` | TS-ESLint | DIR | ✓ | Safe | Disallow unnecessary template literal expressions |
| `LY024` | `no-unnecessary-lambda` | ErrorProne | AST | ✓ | Safe | Disallow lambdas that only wrap a direct function call |
| `LY026` | `no-var` | ESLint | AST | ✓ | Safe | Require `let` or `const` instead of `var` |
| `LY027` | `object-shorthand` | ESLint | AST | ✓ | Safe | Require or disallow method and property shorthand syntax |
| `LY028` | `operator-assignment` | ESLint | AST | ✓ | Safe | Require or disallow assignment operator shorthand |
| `LY033` | `prefer-arrow-callback` | ESLint | AST | ✓ | Safe | Require arrow functions as callbacks |
| `LY034` | `prefer-as-const` | TS-ESLint | AST | ✓ | Safe | Prefer `as const` over literal type assertions |
| `LY035` | `prefer-const` | ESLint | DIR | ✓ | Safe | Require `const` declarations for never-reassigned variables |
| `LY075` | `prefer-destructuring` | ESLint | DIR |  | Safe | Prefer destructuring from arrays and objects |
| `LY036` | `prefer-exponentiation-operator` | ESLint | DIR | ✓ | Safe | Prefer `**` over `Math.pow()` |
| `LY037` | `prefer-expression` | Destack | AST | ✓ | None | Prefer expression syntax for assignments |
| `LY039` | `prefer-fragment-shorthand` | Destack | AST | ✓ | Safe | Prefer `<>` shorthand over `<Fragment>` |
| `LY040` | `prefer-if-else-over-match-bool` | Destack | AST | ✓ | None | Suggest using if/else instead of match on booleans |
| `LY041` | `prefer-implicit-return` | Destack | AST | ✓ | Safe | Prefer implicit returns in expression-bodied functions |
| `LY042` | `prefer-inclusive-range` | Destack | AST | ✓ | Safe | Prefer inclusive range syntax where applicable |
| `LY043` | `prefer-loop` | Destack | AST | ✓ | Safe | Prefer `loop` keyword over `while(true)` or `for(;;)` |
| `LY045` | `prefer-named-extension` | Destack | AST | ✓ | None | Prefer named extensions for foreign types |
| `LY046` | `prefer-nullish-coalescing` | TS-ESLint | DIR | ✓ | Safe | Prefer `??` over `\ |
| `LY047` | `prefer-numeric-literals` | ESLint | DIR | ✓ | Safe | Prefer numeric literals over `parseInt()` |
| `LY069` | `prefer-object-has-own` | ESLint | DIR | ✓ | Safe | Prefer `Object.hasOwn()` over `Object.prototype.hasOwnProperty` |
| `LY048` | `prefer-object-spread` | ESLint | DIR | ✓ | Safe | Prefer spread operator over `Object.assign()` |
| `LY049` | `prefer-pattern-over-guard` | Destack | AST | ✓ | None | Suggest moving match guards into the pattern |
| `LY050` | `prefer-precise-numeric` | Destack | AST | ✓ | Suggestion | Prefer precise numeric types over `number` |
| `LY076` | `prefer-promise-reject-errors` | TS-ESLint | DIR |  | Suggestion | Require Error objects in Promise rejections |
| `LY077` | `prefer-propagate-operator` | Destack | DIR |  | Safe | Prefer `?` propagation over manual Result matching |
| `LY051` | `prefer-range-contains` | Destack | AST | ✓ | Safe | Prefer range contains method over comparison chains |
| `LY052` | `prefer-range-literal` | Destack | AST | ✓ | None | Prefer range literals over C-style for loops |
| `LY078` | `prefer-readonly` | TS-ESLint | DIR |  | Suggestion | Prefer `readonly` for non-mutated fields |
| `LY053` | `prefer-self-closing-tree` | Destack | AST | ✓ | Safe | Prefer self-closing tree elements when possible |
| `LY079` | `prefer-set-over-empty-map` | Destack | DIR |  | Safe | Suggest `Set<K>` over `Map<K, void>` |
| `LY055` | `prefer-struct` | Destack | AST | ✓ | None | Prefer struct for data-only classes |
| `LY054` | `prefer-string-replaceall` | Unicorn | DIR | ✓ | Safe | Prefer `.replaceAll()` over `.replace()` with global regex |
| `LY056` | `prefer-struct-literal` | Destack | DIR | ✓ | Safe | Prefer struct literal syntax over constructor calls |
| `LY057` | `prefer-template` | ESLint | AST | ✓ | Safe | Prefer template literals over string concatenation |
| `LY058` | `prefer-tuple` | Destack | AST | ✓ | Safe | Suggest tuple type for fixed-length heterogeneous arrays |
| `LY059` | `prefer-tuple-destructure` | Destack | AST | ✓ | None | Prefer tuple destructuring over indexed access |
| `LY060` | `prefer-tuple-swap` | Destack | AST | ✓ | Safe | Prefer tuple swap syntax over temporary variable |
| `LY061` | `prefer-unary-negation` | Destack | AST | ✓ | Safe | Prefer unary negation over multiplying by -1 |
| `LY080` | `promise-function-async` | TS-ESLint | DIR |  | Safe | Require `async` keyword for Promise-returning functions |
| `LY062` | `require-jsdoc` | ESLint | AST | ✓ | None | Require documentation on public items |
| `LY063` | `require-returns-doc` | ESLint | AST | ✓ | None | Require return type documentation |
| `LY081` | `restrict-template-expressions` | TS-ESLint | DIR |  | Suggestion | Require template expressions to be strings or numbers |
| `LY064` | `sort-imports` | ESLint | AST | ✓ | None | Enforce sorted import declarations |
| `LY065` | `symbol-description` | ESLint | DIR | ✓ | Suggestion | Require symbol descriptions |
| `LY066` | `yoda` | ESLint | AST | ✓ | Safe | Disallow Yoda conditions |
| `LY032` | `prefer-array-some` | Unicorn | DIR | ✓ | Safe | Prefer `some()` over `filter().length` or `findIndex()` comparisons |
| `LY029` | `prefer-array-filter` | Destack | DIR | ✓ | None | Suggest `.filter()` over `forEach` with conditional push |
| `LY030` | `prefer-array-find` | Unicorn | DIR | ✓ | None | Suggest `.find()` over `.filter()[0]` |
| `LY031` | `prefer-array-map` | Destack | DIR | ✓ | None | Suggest `.map()` over `forEach` with push |
| `LY038` | `prefer-flat-map` | Unicorn | DIR | ✓ | Safe | Suggest `.flatMap()` over `.map().flat()` |
| `LY044` | `prefer-match` | Destack | AST | ✓ | None | Suggest match expressions over complex if-else chains |

## Complexity (X)

Overly complex code that is harder to understand and maintain.

[`src/rules/complexity/`](src/rules/complexity/)

| Code | Rule | Source | Level | Status | Fixability | Description |
|------|------|--------|-------|--------|------------|-------------|
| `LX001` | `cognitive-complexity` | Biome | AST | ✓ | None | Enforce a maximum cognitive complexity |
| `LX002` | `cyclomatic-complexity` | ESLint | AST | ✓ | None | Enforce a maximum cyclomatic complexity |
| `LX003` | `max-branching-factor` | Destack | AST | ✓ | None | Enforce a maximum branching factor in conditionals |
| `LX004` | `max-depth` | ESLint | AST | ✓ | None | Enforce a maximum depth of nested blocks |
| `LX011` | `max-static-params` | Destack | AST | ✓ | None | Enforce a maximum number of static parameters |
| `LX005` | `max-lines` | ESLint | AST | ✓ | None | Enforce a maximum number of lines per file |
| `LX006` | `max-lines-per-function` | ESLint | AST | ✓ | None | Enforce a maximum number of lines per function |
| `LX007` | `max-nested-callbacks` | ESLint | AST | ✓ | None | Enforce a maximum depth of nested callbacks |
| `LX008` | `max-params` | ESLint | AST | ✓ | None | Enforce a maximum number of function parameters |
| `LX009` | `max-return-statements` | Code Climate | AST | ✓ | None | Enforce a maximum number of return statements per function |
| `LX010` | `max-statements` | ESLint | AST | ✓ | None | Enforce a maximum number of statements per function |
| `LX012` | `max-switch-cases` | SonarQube | AST | ✓ | None | Enforce a maximum number of cases in a switch statement |
| `LX013` | `max-type-fields` | Destack | AST | ✓ | None | Enforce a maximum number of fields in a struct, class, interface, or object type |
| `LX014` | `max-type-variants` | Destack | AST | ✓ | None | Enforce a maximum number of variants in a union type or enum |
| `LX015` | `no-complex-boolean-expression` | Destack | AST | ✓ | None | Suggest simplifying complex boolean expressions |
| `LX016` | `no-complex-type` | Destack | AST | ✓ | None | Warn on overly complex types that should be aliased |
| `LX017` | `no-duplicate-code` | Code Climate | AST | ✓ | None | Warn on duplicate or near-duplicate code blocks |
| `LX018` | `no-excessive-booleans` | Destack | AST | ✓ | None | Disallow too many boolean parameters or struct fields |
| `LX019` | `no-multi-assign` | ESLint | AST | ✓ | Safe | Disallow chained assignment expressions |
| `LX020` | `no-multi-declarators` | ESLint | AST | ✓ | Safe | Disallow multiple variable declarations per statement |
| `LX021` | `no-nested-switch` | SonarQube | AST | ✓ | None | Disallow switch statements nested inside switch statements (or match) |
| `LX022` | `no-unused-expressions` | ESLint | AST | ✓ | None | Disallow expressions that have no effect |
| `LX023` | `no-useless-underscore-binding` | Destack | AST | ✓ | None | Warn on underscore bindings with no side effects |
| `LX024` | `prefer-expression-over-let-if` | Destack | AST | ✓ | None | Suggest expression syntax over let-if sequences |
| `LX025` | `prefer-if-let` | Destack | AST | ✓ | None | Suggest if-let over single-arm match |
| `LX026` | `prefer-simplified-comparison` | Destack | AST | ✓ | Safe | Suggest simplifying comparisons like `x >= y + 1` |

## Restriction (R)

Opt-in rules that ban certain patterns by project choice. These rules may conflict with each other.

[`src/rules/restriction/`](src/rules/restriction/)

| Code | Rule | Source | Level | Status | Fixability | Description |
|------|------|--------|-------|--------|------------|-------------|
| `LR001` | `no-alert` | ESLint | DIR | ✓ | Unsafe | Disallow the use of `alert`, `confirm`, and `prompt` |
| `LR002` | `no-anonymous-default-export` | Unicorn | AST | ✓ | Suggestion | Disallow anonymous default exports |
| `LR003` | `no-banned-import` | ESLint | DIR | ✓ | None | Disallow imports from specified modules |
| `LR004` | `no-bitwise` | ESLint | AST | ✓ | None | Disallow bitwise operators |
| `LR005` | `no-circular-dependency` | Import | DIR | ✓ | None | Disallow circular module dependencies |
| `LR006` | `no-class` | Destack | AST | ✓ | None | Disallow class declarations (prefer structs) |
| `LR007` | `no-console` | ESLint | DIR | ✓ | Safe | Disallow the use of `console` |
| `LR008` | `no-continue` | ESLint | AST | ✓ | None | Disallow `continue` statements |
| `LR035` | `no-cross-target-import` | Destack | DIR |  | None | Disallow imports across target boundaries |
| `LR009` | `no-default-export` | Import | DIR | ✓ | None | Disallow default exports |
| `LR024` | `no-re-export-all` | Biome | AST | ✓ | None | Disallow `export * from` (hurts tree-shaking) |
| `LR011` | `no-enum` | Biome | AST | ✓ | None | Disallow TypeScript enums (prefer union types) |
| `LR013` | `no-explicit-any` | TS-ESLint | AST | ✓ | Suggestion | Disallow the `any` type |
| `LR038` | `no-extraneous-dependencies` | Import | DIR |  | None | Disallow imports from dependencies not declared for the current package |
| `LR014` | `no-implicit-return` | Destack | AST | ✓ | Safe | Require explicit `return` statements |
| `LR039` | `no-internal-modules` | Import | DIR |  | None | Disallow importing deep internal module paths |
| `LR015` | `no-labels` | ESLint | AST | ✓ | None | Disallow labeled statements |
| `LR036` | `no-layer-violation` | SonarQube | DIR |  | None | Disallow imports that cross architectural layer boundaries |
| `LR016` | `no-magic-numbers` | ESLint | AST | ✓ | None | Disallow magic numbers |
| `LR017` | `no-namespace` | TS-ESLint | AST | ✓ | None | Disallow TypeScript namespaces |
| `LR018` | `no-non-null-assertion` | TS-ESLint | AST | ✓ | Suggestion | Disallow non-null assertions using the `!` postfix |
| `LR019` | `no-null` | Unicorn | AST | ✓ | Safe | Disallow `null` (prefer `undefined`) |
| `LR040` | `no-orphans` | dependency-cruiser | DIR |  | None | Disallow modules that are not reachable from configured entry points |
| `LR020` | `no-parameter-reassignment` | SonarQube | DIR | ✓ | None | Disallow reassigning function parameters |
| `LR021` | `no-placeholder-implementation` | ESLint | AST | ✓ | None | Disallow placeholder implementations (throw "not implemented", etc.) |
| `LR022` | `no-plusplus` | ESLint | AST | ✓ | Safe | Disallow `++` and `--` operators |
| `LR041` | `no-private-api-import` | Destack | DIR |  | None | Disallow importing package private internal APIs from outside their scope |
| `LR023` | `no-process-exit` | Unicorn | DIR | ✓ | Unsafe | Disallow `process.exit()` |
| `LR042` | `no-profile-incompatible-import` | Destack | DIR |  | None | Disallow imports that are not available in the active profile libraries |
| `LR034` | `no-relative-parent-imports` | Import | DIR | ✓ | None | Disallow relative parent path imports |
| `LR033` | `no-require-imports` | TS-ESLint | DIR | ✓ | Unsafe | Disallow `require()` imports |
| `LR037` | `no-runtime-reflection` | Destack | DIR |  | None | Disallow runtime reflection (ban RTTI) |
| `LR025` | `no-sequences` | ESLint | AST | ✓ | None | Disallow comma operators |
| `LR026` | `no-shadow` | ESLint | DIR | ✓ | None | Disallow shadowing by rebinding a value |
| `LR027` | `no-struct` | Destack | AST | ✓ | None | Disallow struct declarations (prefer classes) |
| `LR028` | `no-ternary` | ESLint | AST | ✓ | None | Disallow ternary operators |
| `LR029` | `no-unused-modules` | Import | DIR | ✓ | None | Disallow exports that are never imported by any module |
| `LR030` | `no-warning-comments` | ESLint | AST | ✓ | None | Disallow specified warning terms in comments (TODO, FIXME, etc.) |
| `LR031` | `no-wildcard-imports` | Destack | AST | ✓ | None | Disallow wildcard imports |
| `LR032` | `strict-boolean-expressions` | TS-ESLint | DIR | ✓ | None | Disallow truthy/falsy coercion in conditions |
| `LR010` | `no-delete` | Destack | AST | ✓ | Unsafe | Disallow the `delete` operator |
| `LR012` | `no-exceptions` | Destack | DIR | ✓ | None | Disallow `throw` and `try/catch` (use Result types) |
