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

| Code | Rule | Source | Level | Ready | Status | Fixability | Description |
|------|------|--------|-------|-------|--------|------------|-------------|
| `LC001` | `await-holding-lock` | Clippy | DIR | ✗ | 🔶 | None | Disallow holding a mutex lock across an await point |
| `LC002` | `for-direction` | ESLint | AST | ✓ | ✅ | None | Enforce for loop update clause moving in the correct direction |
| `LC003` | `no-approx-constant` | Destack | AST | ✓ | ✅ | Safe | Disallow approximate representations of mathematical constants |
| `LC004` | `no-arguments-order-mismatch` | SonarQube | DIR | ✓ | 🔶 | Suggestion | Disallow arguments that appear swapped based on parameter names |
| `LC005` | `no-array-constructor` | ESLint | DIR | ✓ | ✅ | Safe | Disallow `new Array()` (confusing behavior) |
| `LC006` | `no-array-delete` | TS-ESLint | DIR | ✓ | ✅ | None | Disallow `delete` on arrays (creates holes) |
| `LC007` | `no-async-promise-executor` | ESLint | DIR | ✓ | ✅ | Unsafe | Disallow async functions as Promise executor |
| `LC008` | `no-base-to-string` | TS-ESLint | DIR | ✓ | 🔶 | Suggestion | Disallow `.toString()` on objects without useful representation |
| `LC009` | `no-borrow-across-await` | Destack | DIR | ✓ | 🔶 | None | Disallow holding borrows across await points |
| `LC010` | `no-class-assign` | ESLint | DIR | ✓ | 🔶 | None | Disallow reassigning class/struct declarations |
| `LC011` | `no-compare-neg-zero` | ESLint | AST | ✓ | ✅ | Safe | Disallow comparing against negative zero |
| `LC012` | `no-const-assign` | ESLint | DIR | ✓ | 🔶 | None | Disallow reassigning const variables |
| `LC013` | `no-constant-binary-expression` | ESLint | AST | ✓ | ✅ | Unsafe | Disallow expressions where the operation doesn't affect the value |
| `LC014` | `no-constant-condition` | ESLint | AST | ✓ | ✅ | None | Disallow constant expressions in conditions |
| `LC015` | `no-control-regex` | ESLint | AST | ✓ | ✅ | None | Disallow control characters in regular expressions |
| `LC016` | `no-deprecated` | TS-ESLint | DIR | ✓ | 🔶 | Suggestion | Disallow use of `@deprecated` APIs |
| `LC017` | `no-division-by-zero` | Destack | DIR | ✓ | 🔶 | None | Disallow division where denominator is not proven non-zero |
| `LC018` | `no-duplicate-case` | ESLint | AST | ✓ | ✅ | None | Disallow duplicate case labels |
| `LC019` | `no-floating-point-equality` | Clippy | DIR | ✓ | 🔶 | Suggestion | Disallow direct `==` comparison of floats |
| `LC020` | `no-empty-range` | Destack | AST | ✓ | ✅ | None | Disallow empty ranges where start > end |
| `LC021` | `no-fallthrough` | ESLint | AST | ✓ | ✅ | Suggestion | Disallow fallthrough of case statements |
| `LC022` | `no-floating-promises` | TS-ESLint | DIR | ✓ | 🔶 | Suggestion | Require Promises to be awaited or returned |
| `LC023` | `no-for-in-array` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Disallow iterating over arrays with for-in |
| `LC024` | `no-func-assign` | ESLint | DIR | ✓ | 🔶 | None | Disallow reassigning function declarations |
| `LC025` | `no-implicit-any-in-export` | Destack | DIR | ✓ | 🔶 | Suggestion | Disallow implicit `any` in public APIs |
| `LC026` | `no-index-out-of-bounds` | Destack | DIR | ✗ | 🔶 | None | Disallow array access where index is not proven in bounds |
| `LC027` | `no-infinite-iterator` | Destack | DIR | ✗ | 🔶 | None | Disallow using methods that produce infinite iterators |
| `LC028` | `no-infinite-recursion` | ErrorProne | DIR | ✓ | 🔶 | None | Disallow functions that unconditionally call themselves |
| `LC029` | `no-invalid-regexp` | ESLint | AST | ✓ | ✅ | None | Disallow invalid regular expression strings |
| `LC030` | `no-iterator-invalidation` | Destack | DIR | ✗ | 🔶 | None | Disallow modifying a collection while iterating over it |
| `LC031` | `no-loop-single-iteration` | SonarQube | AST | ✓ | ✅ | Suggestion | Disallow loops that execute at most once |
| `LC032` | `no-misused-promises` | TS-ESLint | DIR | ✓ | 🔶 | None | Disallow Promises in places not designed to handle them |
| `LC033` | `no-misused-spread` | TS-ESLint | DIR | ✓ | 🔶 | None | Disallow spread syntax in contexts where it's incorrect |
| `LC034` | `no-new-native-nonconstructor` | ESLint | DIR | ✓ | 🔶 | Safe | Disallow `new` on Symbol and BigInt |
| `LC035` | `no-obj-calls` | ESLint | DIR | ✓ | 🔶 | None | Disallow calling global objects as functions |
| `LC036` | `no-overlapping-match-arms` | Destack | DIR | ✓ | 🔶 | Safe | Disallow match patterns that subsume later arms |
| `LC037` | `no-promise-executor-return` | ESLint | DIR | ✓ | ✅ | Safe | Disallow returning values from Promise executor |
| `LC038` | `no-self-compare` | ESLint | AST | ✓ | ✅ | None | Disallow comparisons where both sides are exactly the same |
| `LC039` | `no-sparse-arrays` | ESLint | AST | ✓ | ✅ | None | Disallow sparse arrays with holes |
| `LC040` | `no-struct-identity-compare` | Destack | DIR | ✓ | 🔶 | Safe | Disallow identity comparison on value types |
| `LC041` | `no-this-before-super` | ESLint | DIR | ✓ | 🔶 | None | Disallow `this` before calling `super()` in constructors |
| `LC042` | `no-throw-in-result-function` | Destack | DIR | ✓ | 🔶 | None | Disallow `throw` in functions returning `Result` |
| `LC043` | `no-unchecked-overflow` | Destack | DIR | ✗ | 🔶 | Suggestion | Disallow arithmetic that may overflow without explicit handling |
| `LC044` | `no-unchecked-pointer-deref` | Destack | DIR | ✗ | 🔶 | None | Disallow dereferencing pointers without a proven non-null guard |
| `LC045` | `no-unchecked-type-assertion` | Destack | DIR | ✓ | 🔶 | Suggestion | Disallow type assertions without validation |
| `LC046` | `no-unhandled-result` | Destack | DIR | ✓ | 🔶 | Suggestion | Require Result values to be handled |
| `LC047` | `no-unsafe-finally` | ESLint | AST | ✓ | ✅ | Safe | Disallow control flow statements in finally blocks |
| `LC048` | `no-unsafe-negation` | ESLint | AST | ✓ | ✅ | Safe | Disallow negating the left operand of relational operators |
| `LC049` | `no-unknown-rule-decorator` | Destack | AST | ✓ | ✅ | None | Disallow unknown rule decorators |
| `LC050` | `no-unsafe-optional-chaining` | ESLint | DIR | ✓ | 🔶 | None | Disallow optional chaining in contexts where undefined is not allowed |
| `LC051` | `no-useless-assignment` | ESLint | DIR | ✓ | 🔶 | Safe | Disallow assignments that are immediately overwritten |
| `LC052` | `no-useless-increment` | SonarQube | DIR | ✓ | 🔶 | Safe | Disallow incrementing a value that is never used afterward |
| `LC053` | `require-array-sort-compare` | TS-ESLint | DIR | ✓ | 🔶 | Suggestion | Require comparison function for `.sort()` |
| `LC054` | `switch-exhaustiveness-check` | TS-ESLint | DIR | ✓ | 🔶 | Suggestion | Require switch statements to be exhaustive |
| `LC055` | `unbound-method` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Disallow unbound methods as callbacks |
| `LC056` | `unused-must-use` | Destack | DIR | ✓ | 🔶 | Suggestion | Disallow ignoring return values of `@mustUse` functions |
| `LC057` | `use-isnan` | ESLint | AST | ✓ | ✅ | Safe | Require `Number.isNaN()` instead of comparisons with `NaN` |

## Suspicious (U)

Code that is likely unintentional but may occasionally be intentional.

[`src/rules/suspicious/`](src/rules/suspicious/)

| Code | Rule | Source | Level | Ready | Status | Fixability | Description |
|------|------|--------|-------|-------|--------|------------|-------------|
| `LU001` | `guard-for-in` | ESLint | AST | ✓ | ✅ | Suggestion | Require `hasOwnProperty` guard in for-in loops |
| `LU002` | `no-async-foreach` | Destack | DIR | ✓ | 🔶 | Unsafe | Disallow `forEach` with async callback (doesn't await) |
| `LU003` | `no-async-map-without-await` | Destack | DIR | ✓ | 🔶 | Suggestion | Disallow async callbacks in `.map()` without awaiting results |
| `LU004` | `no-cond-assign` | ESLint | AST | ✓ | ✅ | Suggestion | Disallow assignment operators in conditional expressions |
| `LU005` | `no-conflicting-decorators` | Destack | DIR | ✓ | 🔶 | None | Disallow decorator combinations that conflict |
| `LU006` | `no-confusing-assignment` | Destack | AST | ✓ | ✅ | Suggestion | Warn on assignments that look like comparisons |
| `LU007` | `no-confusing-non-null-assertion` | TS-ESLint | AST | ✓ | ✅ | Safe | Disallow non-null assertions after optional chain expressions |
| `LU008` | `no-constant-assertion` | Destack | AST | ✓ | ✅ | Safe | Disallow assertions on constant values |
| `LU009` | `no-constructor-return` | ESLint | AST | ✓ | ✅ | Safe | Disallow returning values from constructors |
| `LU010` | `no-debugger` | ESLint | AST | ✓ | ✅ | Safe | Disallow debugger statements |
| `LU011` | `no-dupe-else-if` | ESLint | AST | ✓ | ✅ | None | Disallow duplicate conditions in if-else-if chains |
| `LU012` | `no-duplicate-decorators` | Destack | AST | ✓ | 🔶 | Safe | Disallow duplicate decorators on the same target |
| `LU013` | `no-duplicate-match-arms` | Destack | AST | ✓ | ✅ | None | Warn on match arms with identical bodies |
| `LU014` | `no-empty` | ESLint | AST | ✓ | ✅ | Suggestion | Disallow empty block statements |
| `LU015` | `no-empty-function` | ESLint | AST | ✓ | ✅ | Suggestion | Disallow empty functions |
| `LU016` | `no-empty-pattern` | ESLint | AST | ✓ | ✅ | None | Disallow empty destructuring patterns |
| `LU017` | `no-empty-static-block` | ESLint | AST | ✓ | ✅ | Safe | Disallow empty static initialization blocks in classes |
| `LU018` | `no-ex-assign` | ESLint | DIR | ✓ | 🔶 | None | Disallow reassigning exceptions in catch clauses |
| `LU019` | `no-extra-non-null-assertion` | TS-ESLint | AST | ✓ | ✅ | Safe | Disallow extra non-null assertions |
| `LU020` | `no-global-assign` | ESLint | DIR | ✓ | 🔶 | None | Disallow assignments to native objects or read-only globals |
| `LU021` | `no-identical-branches` | SonarQube | AST | ✓ | ✅ | Safe | Warn when all branches of if/switch have identical bodies |
| `LU022` | `no-identical-functions` | SonarQube | DIR | ✓ | 🔶 | Suggestion | Warn on functions with identical implementations |
| `LU023` | `no-implicit-void-expression` | Destack | DIR | ✓ | 🔶 | Safe | Disallow implicit void returns from expression blocks |
| `LU024` | `no-incomplete-range` | Destack | AST | ✓ | ✅ | Safe | Warn on exclusive ranges that are likely meant to be inclusive |
| `LU025` | `no-inner-declarations` | ESLint | AST | ✓ | ✅ | None | Disallow variable or function declarations in nested blocks |
| `LU026` | `no-large-try-block` | DeepSource | AST | ✓ | ✅ | None | Warn when try block contains much more than throwing code |
| `LU027` | `no-loop-func` | ESLint | DIR | ✓ | 🔶 | None | Disallow functions that capture loop variables |
| `LU028` | `no-method-shadowing` | Destack | DIR | ✓ | 🔶 | None | Warn when a method shadows an inherited method |
| `LU029` | `no-misleading-character-class` | ESLint | AST | ✓ | ✅ | Suggestion | Disallow characters that behave unexpectedly in regex |
| `LU030` | `no-missing-override` | ErrorProne | DIR | ✓ | 🔶 | Safe | Warn when method overrides parent without `override` keyword |
| `LU031` | `no-negation-in-equality-check` | Unicorn | AST | ✓ | ✅ | Safe | Disallow negation in the left operand of equality tests |
| `LU032` | `no-prototype-builtins` | ESLint | DIR | ✓ | 🔶 | Safe | Disallow calling Object.prototype methods directly on objects |
| `LU033` | `no-pointer-arithmetic` | Destack | DIR | ✓ | 🔶 | Suggestion | Warn on pointer arithmetic without explicit offset helpers |
| `LU034` | `no-pointer-comparison` | Destack | DIR | ✓ | 🔶 | Suggestion | Warn on pointer comparisons outside explicit address checks |
| `LU035` | `no-redundant-await` | Destack | DIR | ✓ | 🔶 | Safe | Disallow redundant `await` expressions |
| `LU036` | `no-redundant-match-guard` | Destack | AST | ✓ | ✅ | Safe | Disallow match guards that are always true or false |
| `LU037` | `no-redundant-pattern` | Destack | AST | ✓ | ✅ | Safe | Disallow patterns that bind nothing useful |
| `LU038` | `no-return-assign` | ESLint | AST | ✓ | ✅ | None | Disallow assignment operators in return statements |
| `LU039` | `no-self-assign` | ESLint | AST | ✓ | ✅ | Safe | Disallow assignments where both sides are exactly the same |
| `LU040` | `no-shadow-restricted-names` | ESLint | AST | ✓ | ✅ | None | Disallow shadowing of restricted or builtin names |
| `LU041` | `no-single-element-tuple` | Destack | AST | ✓ | ✅ | Safe | Warn on single-element tuples that may be accidental |
| `LU042` | `no-template-curly-in-string` | ESLint | AST | ✓ | ✅ | Safe | Disallow template literal placeholder syntax in regular strings |
| `LU043` | `no-throw-literal` | ESLint | DIR | ✓ | 🔶 | Suggestion | Disallow throwing literals instead of Error objects |
| `LU044` | `no-unnecessary-clone` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Warn on cloning values that are not used afterward |
| `LU045` | `no-unnecessary-type-assertion` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Disallow type assertions that do not change the type |
| `LU046` | `no-unsafe-declaration-merging` | TS-ESLint | DIR | ✓ | 🔶 | None | Disallow unsafe declaration merging |
| `LU047` | `no-unused-except-recursion` | Destack | DIR | ✓ | 🔶 | Suggestion | Warn on function arguments only used for recursion |
| `LU048` | `no-useless-backreference` | ESLint | AST | ✓ | ✅ | Safe | Disallow useless backreferences in regular expressions |
| `LU049` | `no-useless-cast` | Clippy | DIR | ✓ | 🔶 | Safe | Disallow casts that do not change the type |
| `LU050` | `no-useless-catch` | ESLint | AST | ✓ | ✅ | Safe | Disallow catch clauses that only rethrow |
| `LU051` | `no-useless-computed-key` | ESLint | AST | ✓ | ✅ | Safe | Disallow unnecessary computed property keys |
| `LU052` | `no-useless-concat` | ESLint | AST | ✓ | ✅ | Safe | Disallow unnecessary concatenation of literals |
| `LU053` | `no-useless-constructor` | ESLint | AST | ✓ | ✅ | Safe | Disallow unnecessary constructors |
| `LU054` | `no-useless-escape` | ESLint | AST | ✓ | ✅ | Safe | Disallow unnecessary escape characters |
| `LU055` | `no-useless-rename` | ESLint | AST | ✓ | ✅ | Safe | Disallow renaming imports/exports to the same name |
| `LU056` | `no-useless-return` | ESLint | AST | ✓ | ✅ | Safe | Disallow redundant return statements |
| `LU057` | `prefer-array-filter` | Unicorn | DIR | ✓ | 🔶 | Unsafe | Suggest `.filter()` over manual filtering loops |
| `LU058` | `prefer-array-find` | Unicorn | DIR | ✓ | 🔶 | Unsafe | Suggest `.find()` over manual search loops |
| `LU059` | `prefer-array-map` | Destack | DIR | ✓ | 🔶 | Unsafe | Suggest `.map()` over manual mapping loops |
| `LU060` | `prefer-flat-map` | Unicorn | DIR | ✓ | 🔶 | Safe | Suggest `.flatMap()` over `.map().flatten()` |
| `LU061` | `prefer-match` | Destack | AST | ✓ | ✅ | Unsafe | Suggest match expressions over complex if-else chains or switch statements |
| `LU062` | `require-await` | TS-ESLint | AST | ✓ | ✅ | Safe | Disallow async functions with no await expressions |
| `LU063` | `require-else-in-if-chain` | Destack | AST | ✓ | ✅ | Suggestion | Require final else in if-else-if chains |
| `LU064` | `require-yield` | ESLint | AST | ✓ | ✅ | None | Require generator functions to contain yield |
| `LU065` | `return-await` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Enforce consistent `return await` usage |

## Security (S)

Patterns that may expose the application to attacks.

[`src/rules/security/`](src/rules/security/)

| Code | Rule | Source | Level | Ready | Status | Fixability | Description |
|------|------|--------|-------|-------|--------|------------|-------------|
| `LS001` | `no-blank-target` | Biome | AST | ✓ | ✅ | Safe | Disallow `target="_blank"` without `rel="noopener"` |
| `LS002` | `no-hardcoded-ip` | SonarQube | AST | ✓ | ✅ | None | Disallow hardcoded IP addresses |
| `LS003` | `no-implied-eval` | ESLint | DIR | ✓ | 🔶 | Safe | Disallow `setTimeout` and `setInterval` with string arguments |
| `LS004` | `no-insecure-random` | Semgrep | DIR | ✓ | 🔶 | None | Disallow insecure random number generators |
| `LS005` | `no-open-redirect` | Semgrep | DIR | ✗ | 🔶 | None | Disallow redirects using user-controlled URLs |
| `LS006` | `no-path-traversal` | Destack | DIR | ✗ | 🔶 | None | Disallow tainted data in file system paths |
| `LS007` | `no-prototype-pollution` | Semgrep | DIR | ✗ | 🔶 | Suggestion | Disallow patterns that may pollute Object.prototype |
| `LS008` | `no-regex-injection` | Destack | DIR | ✗ | 🔶 | None | Disallow tainted data in regular expression patterns |
| `LS009` | `no-script-url` | ESLint | AST | ✓ | ✅ | None | Disallow `javascript:` URLs |
| `LS010` | `no-secrets` | Biome | AST | ✓ | ✅ | None | Disallow hardcoded secrets and credentials |
| `LS011` | `no-sensitive-log` | Bearer | DIR | ✗ | 🔶 | Suggestion | Disallow logging data marked as sensitive |
| `LS012` | `no-sql-injection` | Destack | DIR | ✗ | 🔶 | None | Disallow tainted data in SQL queries |
| `LS013` | `no-unsafe-deserialization` | Semgrep | DIR | ✗ | 🔶 | None | Disallow unsafe deserialization of untrusted input |
| `LS014` | `no-unsafe-templating` | Semgrep | DIR | ✗ | 🔶 | None | Disallow untrusted data in string templating sinks |
| `LS015` | `no-weak-crypto` | Semgrep | DIR | ✓ | 🔶 | Suggestion | Disallow weak cryptographic algorithms |

## Performance (P)

Correct code that could be faster or use less memory.

[`src/rules/performance/`](src/rules/performance/)

| Code | Rule | Source | Level | Ready | Status | Fixability | Description |
|------|------|--------|-------|-------|--------|------------|-------------|
| `LP001` | `no-accumulating-spread` | Biome | DIR | ✓ | 🔶 | Unsafe | Disallow spreading in accumulators (causes O(n²) allocations) |
| `LP002` | `no-alloc-in-loop` | Clippy | MIR | ✗ | 🔶 | None | Disallow heap allocations inside loops |
| `LP003` | `no-array-for-each` | Unicorn | DIR | ✓ | 🔶 | Safe | Prefer for-of over `Array.forEach()` |
| `LP004` | `no-array-unshift-loop` | Destack | DIR | ✓ | 🔶 | Unsafe | Disallow `unshift` in loops (causes O(n²) reallocations) |
| `LP005` | `no-await-in-loop` | ESLint | AST | ✓ | ✅ | Suggestion | Disallow await inside of loops |
| `LP006` | `no-barrel-file` | Biome | AST | ✓ | ✅ | None | Disallow barrel files that re-export everything |
| `LP007` | `no-intermediate-collect` | Clippy | DIR | ✓ | 🔶 | Suggestion | Disallow collecting only to immediately iterate once |
| `LP008` | `no-json-clone` | Destack | AST | ✓ | 🔶 | Unsafe | Disallow `JSON.parse(JSON.stringify())` for cloning |
| `LP009` | `no-nested-array-includes` | Destack | DIR | ✓ | 🔶 | Unsafe | Disallow `includes`/`indexOf` inside loops over another array |
| `LP010` | `no-object-spread-in-reduce` | Destack | DIR | ✓ | 🔶 | Unsafe | Disallow object spread in reduce accumulators |
| `LP011` | `no-regex-in-loop` | Destack | DIR | ✓ | 🔶 | Safe | Disallow `new RegExp()` inside loops |
| `LP012` | `no-sequential-independent-await` | Destack | DIR | ✗ | 🔶 | Safe | Suggest `Promise.all` for independent sequential awaits |
| `LP013` | `no-string-concat-in-loop` | Destack | DIR | ✓ | 🔶 | Unsafe | Disallow `+=` string concatenation in loops |
| `LP014` | `no-super-linear-regex` | Destack | AST | ✓ | ✅ | None | Disallow regular expressions with catastrophic backtracking |
| `LP015` | `prefer-array-every` | Unicorn | DIR | ✓ | 🔶 | Safe | Prefer `.every()` over `.filter().length === .length` |
| `LP016` | `prefer-array-literal` | Destack | DIR | ✓ | 🔶 | Unsafe | Suggest using array literal instead of empty array followed by extend |
| `LP017` | `prefer-array-some` | Unicorn | DIR | ✓ | 🔶 | Safe | Prefer `.some()` over `.find() !== undefined` |
| `LP018` | `prefer-for-of` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Prefer for-of loops over index-based for loops |
| `LP019` | `prefer-includes` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Prefer `.includes()` over `.indexOf() !== -1` |
| `LP020` | `prefer-reserve` | Clippy | DIR | ✗ | 🔶 | Suggestion | Prefer reserving capacity when the size is known |
| `LP021` | `prefer-string-endswith` | Unicorn | DIR | ✓ | 🔶 | Safe | Prefer `.endsWith()` over `.slice(-n) === suffix` |
| `LP022` | `prefer-string-startswith` | Unicorn | DIR | ✓ | 🔶 | Safe | Prefer `.startsWith()` over `.indexOf() === 0` |
| `LP023` | `prefer-typed-array` | Destack | DIR | ✗ | 🔶 | Unsafe | Suggest `TypedArray` for numeric buffers |
| `LP024` | `require-unicode-regexp` | ESLint | AST | ✓ | ✅ | Safe | Require `u` or `v` flag on regular expressions |

## Style (Y)

Subjective preferences for consistent coding style.

[`src/rules/style/`](src/rules/style/)

| Code | Rule | Source | Level | Ready | Status | Fixability | Description |
|------|------|--------|-------|-------|--------|------------|-------------|
| `LY001` | `array-type` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Require consistently using either `T[]` or `Array<T>` |
| `LY002` | `catch-error-name` | Unicorn | AST | ✓ | ✅ | Safe | Enforce a specific name for catch clause error parameters |
| `LY003` | `comment-casing` | Destack | AST | ✓ | ✅ | Safe | Enforce comment / doc casing |
| `LY004` | `comment-layout` | Destack | AST | ✓ | ✅ | Safe | Enforce comment / doc layout |
| `LY005` | `comment-punctuation` | Destack | AST | ✓ | ✅ | Safe | Enforce comment / doc punctuation style |
| `LY006` | `consistent-extension-style` | Destack | AST | ✓ | ✅ | Unsafe | Enforce consistent use of named or anonymous extensions |
| `LY007` | `consistent-type-definitions` | TS-ESLint | AST | ✓ | ✅ | Safe | Enforce type definitions to use either `interface` or `type` |
| `LY008` | `consistent-type-imports` | TS-ESLint | AST | ✓ | ✅ | Safe | Enforce consistent usage of type imports |
| `LY009` | `consistent-visibility` | Destack | AST | ✓ | 🔶 | Suggestion | Enforce consistent visibility modifiers |
| `LY010` | `default-param-last` | ESLint | AST | ✓ | ✅ | Unsafe | Enforce default parameters to be last |
| `LY011` | `dot-notation` | ESLint | AST | ✓ | ✅ | Safe | Enforce dot notation whenever possible |
| `LY012` | `eqeqeq` | ESLint | AST | ✓ | ✅ | Safe | Require `===` and `!==` |
| `LY013` | `explicit-function-return-type` | TS-ESLint | AST | ✓ | ✅ | Suggestion | Require explicit return types on functions |
| `LY014` | `explicit-module-boundary-types` | TS-ESLint | AST | ✓ | 🔶 | Suggestion | Require explicit types on exported APIs |
| `LY015` | `filename-case` | Unicorn | AST | ✓ | ✅ | Unsafe | Enforce a case style for filenames |
| `LY016` | `grouped-accessor-pairs` | ESLint | AST | ✓ | ✅ | None | Require grouped accessor pairs in object literals and classes |
| `LY017` | `no-boolean-literal-compare` | Unicorn | AST | ✓ | ✅ | Safe | Disallow comparing boolean expressions to boolean literals |
| `LY018` | `no-collapsible-if` | Unicorn | AST | ✓ | ✅ | Safe | Suggest merging nested if statements without else |
| `LY019` | `no-duplicate-string` | SonarQube | AST | ✓ | ✅ | None | Disallow the same string literal appearing many times |
| `LY020` | `no-duplicate-type-constituents` | TS-ESLint | AST | ✓ | ✅ | Safe | Disallow duplicate constituents in union/intersection types |
| `LY021` | `no-else-return` | ESLint | AST | ✓ | ✅ | Safe | Disallow else blocks after return statements |
| `LY022` | `no-empty-interface` | TS-ESLint | AST | ✓ | ✅ | Safe | Disallow empty interfaces |
| `LY023` | `no-extra-boolean-cast` | ESLint | DIR | ✓ | 🟡 | Safe | Disallow unnecessary boolean casts |
| `LY024` | `no-implicit-coercion` | ESLint | DIR | ✓ | 🟡 | Safe | Disallow shorthand type conversions |
| `LY025` | `no-lonely-if` | ESLint | AST | ✓ | ✅ | Safe | Disallow if statements as the only statement in else blocks |
| `LY026` | `no-negated-condition` | ESLint | AST | ✓ | ✅ | Safe | Disallow negated conditions with else branches |
| `LY027` | `no-redundant-type-constituents` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Disallow type constituents made redundant by others |
| `LY028` | `no-nested-template-literal` | SonarQube | AST | ✓ | ✅ | None | Disallow template literals nested inside template literals |
| `LY029` | `no-nested-ternary` | ESLint | AST | ✓ | ✅ | Unsafe | Disallow nested ternary expressions |
| `LY030` | `no-switch` | Destack | AST | ✓ | 🔶 | Safe | Disallow switch statements (in favor of match) |
| `LY031` | `no-object-constructor` | ESLint | DIR | ✓ | 🔶 | Safe | Disallow `new Object()` |
| `LY032` | `no-unneeded-ternary` | ESLint | AST | ✓ | ✅ | Safe | Disallow ternary operators when simpler alternatives exist |
| `LY033` | `no-unnecessary-template-expression` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Disallow unnecessary template literal expressions |
| `LY034` | `no-unnecessary-type-arguments` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Disallow type arguments that equal the default |
| `LY035` | `no-unnecessary-lambda` | ErrorProne | AST | ✓ | ✅ | Safe | Disallow lambdas that only wrap a direct function call |
| `LY036` | `no-var` | ESLint | AST | ✓ | ✅ | Safe | Require `let` or `const` instead of `var` |
| `LY037` | `object-shorthand` | ESLint | AST | ✓ | ✅ | Safe | Require or disallow method and property shorthand syntax |
| `LY038` | `operator-assignment` | ESLint | AST | ✓ | ✅ | Safe | Require or disallow assignment operator shorthand |
| `LY039` | `prefer-arrow-callback` | ESLint | AST | ✓ | ✅ | Safe | Require arrow functions as callbacks |
| `LY040` | `prefer-as-const` | TS-ESLint | AST | ✓ | ✅ | Safe | Prefer `as const` over literal type assertions |
| `LY041` | `prefer-const` | ESLint | DIR | ✓ | 🔶 | Safe | Require `const` declarations for never-reassigned variables |
| `LY042` | `prefer-destructuring` | ESLint | DIR | ✓ | 🔶 | Safe | Prefer destructuring from arrays and objects |
| `LY043` | `prefer-exponentiation-operator` | ESLint | DIR | ✓ | 🔶 | Safe | Prefer `**` over `Math.pow()` |
| `LY044` | `prefer-expression` | Destack | AST | ✓ | ✅ | Safe | Prefer expression syntax for assignments |
| `LY045` | `prefer-extension-method` | Destack | DIR | ✗ | 🔶 | Suggestion | Suggest converting functions to extension methods |
| `LY046` | `prefer-fragment-shorthand` | Destack | AST | ✓ | ✅ | Safe | Prefer `<>` shorthand over `<Fragment>` |
| `LY047` | `prefer-if-else-over-match-bool` | Destack | AST | ✓ | ✅ | Safe | Suggest using if/else instead of match on booleans |
| `LY048` | `prefer-implicit-return` | Destack | AST | ✓ | ✅ | Safe | Prefer implicit returns in expression-bodied functions |
| `LY049` | `prefer-inclusive-range` | Destack | AST | ✓ | ✅ | Safe | Prefer inclusive range syntax where applicable |
| `LY050` | `prefer-loop` | Destack | AST | ✓ | ✅ | Safe | Prefer `loop` keyword over `while(true)` or `for(;;)` |
| `LY051` | `prefer-map-or-else` | Destack | DIR | ✓ | 🔶 | Safe | Prefer `mapOrElse()` over `map().unwrap()` |
| `LY052` | `prefer-named-extension` | Destack | AST | ✓ | ✅ | Unsafe | Prefer named extensions for foreign types |
| `LY081` | `prefer-nullish-coalescing` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Prefer `??` over `\|\|` for default values |
| `LY053` | `prefer-numeric-literals` | ESLint | DIR | ✓ | 🔶 | Safe | Prefer numeric literals over `parseInt()` |
| `LY054` | `prefer-object-has-own` | ESLint | DIR | ✓ | 🔶 | Safe | Prefer `Object.hasOwn()` over `Object.prototype.hasOwnProperty` |
| `LY055` | `prefer-object-spread` | ESLint | DIR | ✓ | 🔶 | Safe | Prefer spread operator over `Object.assign()` |
| `LY056` | `prefer-pattern-over-guard` | Destack | AST | ✓ | ✅ | Safe | Suggest moving match guards into the pattern |
| `LY057` | `prefer-precise-numeric` | Destack | AST | ✓ | ✅ | Suggestion | Prefer precise numeric types over `number` |
| `LY058` | `prefer-promise-reject-errors` | TS-ESLint | DIR | ✓ | 🔶 | Suggestion | Require Error objects in Promise rejections |
| `LY059` | `prefer-propagate-operator` | Destack | DIR | ✓ | 🟡 | Safe | Prefer `?` propagation over manual Result matching |
| `LY060` | `prefer-range-contains` | Destack | AST | ✓ | ✅ | Safe | Prefer range contains method over comparison chains |
| `LY061` | `prefer-range-literal` | Destack | AST | ✓ | ✅ | Safe | Prefer range literals over C-style for loops |
| `LY062` | `prefer-readonly` | TS-ESLint | DIR | ✓ | 🔶 | Suggestion | Prefer `readonly` for non-mutated fields |
| `LY063` | `prefer-result-type` | Destack | DIR | ✓ | 🔶 | Suggestion | Prefer `Result<T, E>` return type over throwing |
| `LY064` | `prefer-self-closing-tree` | Destack | AST | ✓ | ✅ | Safe | Prefer self-closing tree elements when possible |
| `LY065` | `prefer-set-over-empty-map` | Destack | DIR | ✓ | 🔶 | Safe | Suggest `Set<K>` over `Map<K, void>` |
| `LY066` | `prefer-struct` | Destack | AST | ✓ | ✅ | Unsafe | Prefer struct for data-only classes |
| `LY067` | `prefer-string-replaceall` | Unicorn | DIR | ✓ | 🔶 | Safe | Prefer `.replaceAll()` over `.replace()` with global regex |
| `LY068` | `prefer-struct-literal` | Destack | AST | ✓ | ✅ | Safe | Prefer struct literal syntax over constructor calls |
| `LY069` | `prefer-template` | ESLint | AST | ✓ | ✅ | Safe | Prefer template literals over string concatenation |
| `LY070` | `prefer-tuple` | Destack | AST | ✓ | ✅ | Safe | Suggest tuple type for fixed-length heterogeneous arrays |
| `LY071` | `prefer-tuple-destructure` | Destack | AST | ✓ | ✅ | Safe | Prefer tuple destructuring over indexed access |
| `LY072` | `prefer-tuple-swap` | Destack | AST | ✓ | ✅ | Safe | Prefer tuple swap syntax over temporary variable |
| `LY073` | `prefer-unary-negation` | Destack | AST | ✓ | ✅ | Safe | Prefer unary negation over multiplying by -1 |
| `LY074` | `promise-function-async` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Require `async` keyword for Promise-returning functions |
| `LY075` | `require-jsdoc` | ESLint | AST | ✓ | ✅ | Suggestion | Require documentation on public items |
| `LY076` | `require-returns-doc` | ESLint | AST | ✓ | ✅ | Suggestion | Require return type documentation |
| `LY077` | `restrict-template-expressions` | TS-ESLint | DIR | ✓ | 🔶 | Suggestion | Require template expressions to be strings or numbers |
| `LY078` | `sort-imports` | ESLint | AST | ✓ | ✅ | Safe | Enforce sorted import declarations |
| `LY079` | `symbol-description` | ESLint | DIR | ✓ | 🔶 | Suggestion | Require symbol descriptions |
| `LY080` | `yoda` | ESLint | AST | ✓ | ✅ | Safe | Disallow Yoda conditions |

## Complexity (X)

Overly complex code that is harder to understand and maintain.

[`src/rules/complexity/`](src/rules/complexity/)

| Code | Rule | Source | Level | Ready | Status | Fixability | Description |
|------|------|--------|-------|-------|--------|------------|-------------|
| `LX001` | `cognitive-complexity` | Biome | AST | ✓ | ✅ | None | Enforce a maximum cognitive complexity |
| `LX002` | `cyclomatic-complexity` | ESLint | AST | ✓ | ✅ | None | Enforce a maximum cyclomatic complexity |
| `LX003` | `max-branching-factor` | Destack | AST | ✓ | 🔶 | None | Enforce a maximum branching factor in conditionals |
| `LX004` | `max-depth` | ESLint | AST | ✓ | ✅ | None | Enforce a maximum depth of nested blocks |
| `LX005` | `max-generic-params` | Destack | AST | ✓ | 🔶 | None | Enforce a maximum number of generic parameters |
| `LX006` | `max-lines` | ESLint | AST | ✓ | ✅ | None | Enforce a maximum number of lines per file |
| `LX007` | `max-lines-per-function` | ESLint | AST | ✓ | ✅ | None | Enforce a maximum number of lines per function |
| `LX008` | `max-nested-callbacks` | ESLint | AST | ✓ | ✅ | None | Enforce a maximum depth of nested callbacks |
| `LX009` | `max-params` | ESLint | AST | ✓ | ✅ | None | Enforce a maximum number of function parameters |
| `LX010` | `max-return-statements` | Code Climate | AST | ✓ | ✅ | None | Enforce a maximum number of return statements per function |
| `LX011` | `max-statements` | ESLint | AST | ✓ | ✅ | None | Enforce a maximum number of statements per function |
| `LX012` | `max-switch-cases` | SonarQube | AST | ✓ | ✅ | None | Enforce a maximum number of cases in a switch statement |
| `LX013` | `max-type-fields` | Destack | AST | ✓ | ✅ | None | Enforce a maximum number of fields in a struct, class, interface, or object type |
| `LX014` | `max-type-nesting` | Destack | AST | ✓ | 🔶 | None | Enforce a maximum nesting depth for types |
| `LX015` | `max-type-variants` | Destack | AST | ✓ | ✅ | None | Enforce a maximum number of variants in a union type or enum |
| `LX016` | `no-complex-boolean-expression` | Destack | AST | ✓ | ✅ | Safe | Suggest simplifying complex boolean expressions |
| `LX017` | `no-complex-type` | Destack | AST | ✓ | ✅ | None | Warn on overly complex types that should be aliased |
| `LX018` | `no-duplicate-code` | Code Climate | DIR | ✗ | 🔶 | Suggestion | Warn on duplicate or near-duplicate code blocks |
| `LX019` | `no-excessive-booleans` | Destack | AST | ✓ | ✅ | Suggestion | Disallow too many boolean parameters or struct fields |
| `LX020` | `no-multi-assign` | ESLint | AST | ✓ | ✅ | Safe | Disallow chained assignment expressions |
| `LX021` | `no-multi-declarators` | ESLint | AST | ✓ | ✅ | Safe | Disallow multiple variable declarations per statement |
| `LX022` | `no-nested-switch` | SonarQube | AST | ✓ | ✅ | None | Disallow switch statements nested inside switch statements (or match) |
| `LX023` | `no-unused-expressions` | ESLint | AST | ✓ | ✅ | Safe | Disallow expressions that have no effect |
| `LX024` | `no-useless-underscore-binding` | Destack | AST | ✓ | ✅ | Safe | Warn on underscore bindings with no side effects |
| `LX025` | `prefer-expression-over-let-if` | Destack | AST | ✓ | ✅ | Safe | Suggest expression syntax over let-if sequences |
| `LX026` | `prefer-if-let` | Destack | AST | ✓ | ✅ | Safe | Suggest if-let over single-arm match |
| `LX027` | `prefer-simplified-comparison` | Destack | AST | ✓ | ✅ | Safe | Suggest simplifying comparisons like `x >= y + 1` |

## Restriction (R)

Opt-in rules that ban certain patterns by project choice. These rules may conflict with each other.

[`src/rules/restriction/`](src/rules/restriction/)

| Code | Rule | Source | Level | Ready | Status | Fixability | Description |
|------|------|--------|-------|-------|--------|------------|-------------|
| `LR001` | `no-alert` | ESLint | DIR | ✓ | 🔶 | None | Disallow the use of `alert`, `confirm`, and `prompt` |
| `LR002` | `no-anonymous-default-export` | Unicorn | AST | ✓ | ✅ | Suggestion | Disallow anonymous default exports |
| `LR003` | `no-arguments` | ESLint | DIR | ✓ | 🔶 | Safe | Disallow use of the `arguments` object |
| `LR004` | `no-banned-import` | ESLint | DIR | ✓ | 🔶 | None | Disallow imports from specified modules |
| `LR005` | `no-bitwise` | ESLint | AST | ✓ | ✅ | None | Disallow bitwise operators |
| `LR006` | `no-circular-dependency` | Import | DIR | ✗ | 🔶 | None | Disallow circular module dependencies |
| `LR007` | `no-class` | Destack | AST | ✓ | ✅ | Unsafe | Disallow class declarations (prefer structs) |
| `LR008` | `no-console` | ESLint | DIR | ✓ | 🔶 | Safe | Disallow the use of `console` |
| `LR009` | `no-continue` | ESLint | AST | ✓ | ✅ | None | Disallow `continue` statements |
| `LR010` | `no-cross-target-import` | Destack | DIR | ✓ | 🔶 | None | Disallow imports across target boundaries |
| `LR011` | `no-default-export` | Import | AST | ✓ | ✅ | Unsafe | Disallow default exports |
| `LR012` | `no-re-export-all` | Biome | AST | ✓ | ✅ | None | Disallow `export * from` (hurts tree-shaking) |
| `LR013` | `no-enum` | Biome | AST | ✓ | ✅ | Unsafe | Disallow TypeScript enums (prefer union types) |
| `LR014` | `no-explicit-any` | TS-ESLint | AST | ✓ | ✅ | Suggestion | Disallow the `any` type |
| `LR015` | `no-implicit-return` | Destack | AST | ✓ | ✅ | Safe | Require explicit `return` statements |
| `LR016` | `no-labels` | ESLint | AST | ✓ | ✅ | None | Disallow labeled statements |
| `LR017` | `no-layer-violation` | SonarQube | DIR | ✗ | 🔶 | None | Disallow imports that cross architectural layer boundaries |
| `LR018` | `no-magic-numbers` | ESLint | AST | ✓ | ✅ | Suggestion | Disallow magic numbers |
| `LR019` | `no-namespace` | TS-ESLint | AST | ✓ | ✅ | None | Disallow TypeScript namespaces |
| `LR020` | `no-non-null-assertion` | TS-ESLint | AST | ✓ | ✅ | Suggestion | Disallow non-null assertions using the `!` postfix |
| `LR021` | `no-null` | Unicorn | AST | ✓ | ✅ | Safe | Disallow `null` (prefer `undefined`) |
| `LR022` | `no-parameter-reassignment` | SonarQube | DIR | ✓ | 🔶 | None | Disallow reassigning function parameters |
| `LR023` | `no-placeholder-implementation` | ESLint | AST | ✓ | ✅ | None | Disallow placeholder implementations (throw "not implemented", etc.) |
| `LR024` | `no-plusplus` | ESLint | AST | ✓ | ✅ | Safe | Disallow `++` and `--` operators |
| `LR025` | `no-process-exit` | Unicorn | DIR | ✓ | 🔶 | None | Disallow `process.exit()` |
| `LR026` | `no-require-imports` | TS-ESLint | DIR | ✓ | 🟡 | Safe | Disallow `require()` imports |
| `LR027` | `no-runtime-reflection` | Destack | DIR | ✗ | 🔶 | None | Disallow runtime reflection (ban RTTI) |
| `LR028` | `no-sequences` | ESLint | AST | ✓ | ✅ | None | Disallow comma operators |
| `LR029` | `no-shadow` | Destack | DIR | ✓ | 🔶 | Suggestion | Disallow shadowing by rebinding a value |
| `LR030` | `no-struct` | Destack | AST | ✓ | ✅ | Unsafe | Disallow struct declarations (prefer classes) |
| `LR031` | `no-ternary` | ESLint | AST | ✓ | ✅ | Unsafe | Disallow ternary operators |
| `LR032` | `no-void` | ESLint | AST | ✓ | 🔶 | Safe | Disallow the `void` operator |
| `LR033` | `no-warning-comments` | ESLint | AST | ✓ | ✅ | None | Disallow specified warning terms in comments (TODO, FIXME, etc.) |
| `LR034` | `no-wildcard-imports` | Destack | AST | ✓ | ✅ | Unsafe | Disallow wildcard imports |
| `LR035` | `strict-boolean-expressions` | TS-ESLint | DIR | ✓ | 🔶 | Unsafe | Disallow truthy/falsy coercion in conditions |
| `LR036` | `no-delete` | Destack | AST | ✓ | ✅ | None | Disallow the `delete` operator |
