# linter

Static analysis rules for Destack (`.ds`) and TypeScript (`.ts`, `.tsx`).
Lints run at different IR levels (AST, DIR, MIR), usually per module.

## Overview

The linter is a separate crate from the compiler, but deeply integrated with the same IRs.
It runs directly in process the same IRs (zero copy) and takes advantage of the same task parallelization.
Rules can operate on all the main IRs: AST (syntax patterns), DIR (typed IR), or MIR (low-level IR).

We draw inspiration from linters across the ecosystem: ESLint, TypeScript-ESLint, Biome, Clippy, Ruff, SonarQube, Semgrep.
Each rule notes its main inspiration when following an established rule, and we also try to keep the name, severity, and fixability to a level the ecosystem is used to.
However, the Destack linter is _not_ intended as a complete replacement or even substitute for contemporary linters like ESLint.

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

## Correctness (C)

High-confidence issues that are almost always wrong.

[`src/rules/correctness/`](src/rules/correctness/)

| Code | Rule | Source | Level | Status | Autofix Support | Fixability | Description |
|------|------|--------|-------|--------|------------------|------------|-------------|
| `LC001` | `for-direction` | ESLint | AST | ✓ | Sometimes | Unsafe | Enforce for loop update clause moving in the correct direction |
| `LC002` | `no-approx-constant` | Destack | AST | ✓ | Sometimes | Safe | Disallow approximate representations of mathematical constants |
| `LC046` | `no-arguments-order-mismatch` | SonarQube | DIR | ✓ | No | None | Disallow arguments that appear swapped based on parameter names |
| `LC005` | `no-async-promise-executor` | ESLint | DIR | ✓ | Sometimes | Unsafe | Disallow async functions as Promise executor |
| `LC006` | `no-base-to-string` | TS-ESLint | DIR | ✓ | No | None | Disallow `.toString()` on objects without useful representation |
| `LC007` | `no-compare-neg-zero` | ESLint | AST | ✓ | Sometimes | Safe | Disallow comparing against negative zero |
| `LC068` | `no-confusing-void-expression` | TS-ESLint | DIR | ✓ | No | None | Disallow `void` expressions in positions where values are expected |
| `LC008` | `no-constant-binary-expression` | ESLint | AST | ✓ | No | None | Disallow expressions where the operation doesn't affect the value |
| `LC009` | `no-constant-condition` | ESLint | AST | ✓ | Sometimes | Safe | Disallow constant expressions in conditions |
| `LC010` | `no-control-regex` | ESLint | AST | ✓ | No | None | Disallow control characters in regular expressions |
| `LC011` | `no-deprecated` | TS-ESLint | DIR | ✓ | No | None | Disallow use of `@deprecated` APIs |
| `LC012` | `no-duplicate-case` | ESLint | AST | ✓ | Always | Unsafe | Disallow duplicate case labels |
| `LC015` | `no-floating-point-equality` | Clippy | DIR | ✓ | No | None | Disallow direct `==` comparison of floats |
| `LC014` | `no-fallthrough` | ESLint | AST | ✓ | Always | Suggestion | Disallow fallthrough of case statements |
| `LC016` | `no-floating-promises` | TS-ESLint | DIR | ✓ | Sometimes | Suggestion | Require Promises to be awaited or returned |
| `LC017` | `no-for-in-array` | TS-ESLint | DIR | ✓ | No | None | Disallow iterating over arrays with for-in |
| `LC019` | `no-infinite-recursion` | ErrorProne | DIR | ✓ | No | None | Disallow functions that unconditionally call themselves |
| `LC020` | `no-invalid-regexp` | ESLint | AST | ✓ | No | None | Disallow invalid regular expression strings |
| `LC021` | `no-iterator-invalidation` | Destack | DIR | ✓ | No | None | Disallow modifying a collection while iterating over it |
| `LC022` | `no-loop-single-iteration` | SonarQube | AST | ✓ | Sometimes | Unsafe | Disallow loops that execute at most once |
| `LC023` | `no-misused-promises` | TS-ESLint | DIR | ✓ | No | None | Disallow Promises in places not designed to handle them |
| `LC048` | `no-overlapping-match-arms` | Destack | DIR | ✓ | Always | Unsafe | Disallow match patterns that subsume later arms |
| `LC024` | `no-promise-executor-return` | ESLint | DIR | ✓ | Sometimes | Safe | Disallow returning values from Promise executor |
| `LC025` | `no-self-compare` | ESLint | DIR | ✓ | No | None | Disallow comparisons where both sides are exactly the same |
| `LC027` | `no-struct-identity-compare` | Destack | DIR | ✓ | No | None | Disallow identity comparison on value types |
| `LC028` | `no-throw-in-result-function` | Destack | DIR | ✓ | No | None | Disallow `throw` in functions returning `Result` |
| `LC031` | `no-unnecessary-type-arguments` | TS-ESLint | DIR | ✓ | Sometimes | Safe | Disallow type arguments that equal the default |
| `LC032` | `no-unnecessary-type-assertion` | TS-ESLint | DIR | ✓ | Always | Safe | Disallow type assertions that do not change the type |
| `LC033` | `no-unsafe-finally` | ESLint | AST | ✓ | No | None | Disallow control flow statements in finally blocks |
| `LC034` | `no-unsafe-negation` | ESLint | AST | ✓ | Always | Safe | Disallow negating the left operand of relational operators |
| `LC029` | `no-unknown-rule-decorator` | Destack | AST | ✓ | No | None | Disallow unknown rule decorators |
| `LC030` | `no-unnecessary-condition` | TS-ESLint | DIR | ✓ | No | None | Disallow conditions that are always truthy, always falsy, or never nullish |
| `LC035` | `no-unused-imports` | Destack | DIR | ✓ | Sometimes | Safe | Disallow unused import bindings |
| `LC036` | `no-unused-parameters` | Destack | DIR | ✓ | Sometimes | Unsafe | Disallow unused function and method parameters |
| `LC037` | `no-unused-private-class-members` | Destack | DIR | ✓ | Sometimes | Unsafe | Disallow unused private class members |
| `LC038` | `no-useless-assignment` | ESLint | DIR | ✓ | No | None | Disallow assignments that are immediately overwritten |
| `LC039` | `no-useless-increment` | SonarQube | DIR | ✓ | No | None | Disallow incrementing a value that is never used afterward |
| `LC040` | `require-array-sort-compare` | TS-ESLint | DIR | ✓ | No | None | Require comparison function for `.sort()` |
| `LC041` | `unbound-method` | TS-ESLint | DIR | ✓ | Sometimes | Unsafe | Disallow unbound methods as callbacks |
| `LC042` | `unused-must-use` | Destack | DIR | ✓ | Always | Suggestion | Disallow ignoring return values of `@mustUse` functions |
| `LC043` | `use-isnan` | ESLint | AST | ✓ | Sometimes | Safe | Require `Number.isNaN()` instead of comparisons with `NaN` |

## Suspicious (U)

Code that is likely unintentional but may occasionally be intentional.

[`src/rules/suspicious/`](src/rules/suspicious/)

| Code | Rule | Source | Level | Status | Autofix Support | Fixability | Description |
|------|------|--------|-------|--------|------------------|------------|-------------|
| `LU002` | `no-async-foreach` | Destack | DIR | ✓ | Sometimes | Unsafe | Disallow `forEach` with async callback (doesn't await) |
| `LU003` | `no-cond-assign` | ESLint | AST | ✓ | Sometimes | Safe | Disallow assignment operators in conditional expressions |
| `LU004` | `no-confusing-assignment` | Destack | AST | ✓ | Always | Safe | Warn on assignments that look like comparisons |
| `LU005` | `no-confusing-non-null-assertion` | TS-ESLint | AST | ✓ | Always | Suggestion | Disallow confusing non-null assertions around operators and optional chains |
| `LU006` | `no-constant-assertion` | Destack | AST | ✓ | Sometimes | Suggestion | Disallow assertions on constant values |
| `LU007` | `no-constructor-return` | ESLint | DIR | ✓ | Sometimes | Unsafe | Disallow returning values from constructors |
| `LU008` | `no-debugger` | ESLint | AST | ✓ | Sometimes | Safe | Disallow debugger statements |
| `LU010` | `no-duplicate-else-if` | ESLint | AST | ✓ | Sometimes | Unsafe | Disallow duplicate conditions in if-else-if chains |
| `LU009` | `no-duplicate-decorators` | Destack | AST | ✓ | Sometimes | Suggestion | Disallow identical decorators (same name and arguments) |
| `LU011` | `no-duplicate-match-arms` | Destack | AST | ✓ | Sometimes | Unsafe | Warn on match arms with identical bodies |
| `LU012` | `no-empty` | ESLint | AST | ✓ | Sometimes | Safe | Disallow empty block statements |
| `LU013` | `no-empty-function` | ESLint | AST | ✓ | Always | Safe | Disallow empty functions |
| `LU014` | `no-empty-pattern` | ESLint | AST | ✓ | Sometimes | Safe | Disallow empty destructuring patterns |
| `LU015` | `no-empty-static-block` | ESLint | AST | ✓ | Always | Suggestion | Disallow empty static initialization blocks in classes |
| `LU016` | `no-ex-assign` | ESLint | AST | ✓ | Sometimes | Unsafe | Disallow reassigning exceptions in catch clauses |
| `LU017` | `no-extra-non-null-assertion` | TS-ESLint | AST | ✓ | Always | Safe | Disallow extra non-null assertions |
| `LU018` | `no-identical-branches` | SonarQube | AST | ✓ | No | None | Warn when all branches of if/switch have identical bodies |
| `LU020` | `no-inner-declarations` | ESLint | AST | ✓ | No | None | Disallow variable or function declarations in nested blocks |
| `LU021` | `no-large-try-block` | DeepSource | AST | ✓ | No | None | Warn when try block contains much more than throwing code |
| `LU046` | `no-loop-func` | ESLint | DIR | ✓ | No | None | Disallow functions that capture loop variables |
| `LU023` | `no-mixed-key-types` | Destack | DIR | ✓ | No | None | Warn on objects that mix string, symbol, and numeric keys |
| `LU022` | `no-misleading-character-class` | ESLint | AST | ✓ | No | None | Disallow characters that behave unexpectedly in regex |
| `LU024` | `no-negation-in-equality-check` | Unicorn | AST | ✓ | Sometimes | Suggestion | Disallow negation in the left operand of equality tests |
| `LU025` | `no-redundant-match-guard` | Destack | AST | ✓ | Sometimes | Safe | Disallow match guards that are always true or false |
| `LU026` | `no-redundant-pattern` | Destack | AST | ✓ | No | None | Disallow patterns that bind nothing useful |
| `LU027` | `no-return-assign` | ESLint | AST | ✓ | Sometimes | Unsafe | Disallow assignment operators in return statements |
| `LU028` | `no-self-assign` | ESLint | AST | ✓ | Sometimes | Suggestion | Disallow assignments where both sides are exactly the same |
| `LU029` | `no-shadow-restricted-names` | ESLint | AST | ✓ | Sometimes | Suggestion | Disallow shadowing of restricted or builtin names |
| `LU030` | `no-single-element-tuple` | Destack | AST | ✓ | Sometimes | Suggestion | Warn on single-element tuples that may be accidental |
| `LU031` | `no-template-curly-in-string` | ESLint | AST | ✓ | Sometimes | Unsafe | Disallow template literal placeholder syntax in regular strings |
| `LU032` | `no-throw-literal` | ESLint | DIR | ✓ | Always | Unsafe | Disallow throwing literals instead of Error objects |
| `LU033` | `no-unused-except-recursion` | Destack | DIR | ✓ | Sometimes | Unsafe | Warn on function arguments only used for recursion |
| `LU034` | `no-useless-backreference` | ESLint | AST | ✓ | No | None | Disallow useless backreferences in regular expressions |
| `LU035` | `no-useless-catch` | ESLint | AST | ✓ | No | None | Disallow catch clauses that only rethrow |
| `LU036` | `no-useless-computed-key` | ESLint | AST | ✓ | Sometimes | Safe | Disallow unnecessary computed property keys |
| `LU037` | `no-useless-concat` | ESLint | AST | ✓ | Sometimes | Safe | Disallow unnecessary concatenation of literals |
| `LU038` | `no-useless-constructor` | ESLint | DIR | ✓ | Sometimes | Suggestion | Disallow unnecessary constructors |
| `LU039` | `no-useless-escape` | ESLint | AST | ✓ | Sometimes | Suggestion | Disallow unnecessary escape characters |
| `LU040` | `no-useless-rename` | ESLint | AST | ✓ | Sometimes | Safe | Disallow renaming imports, exports, and destructuring to the same name |
| `LU041` | `no-useless-return` | ESLint | AST | ✓ | Sometimes | Safe | Disallow redundant return statements |
| `LU042` | `require-await` | TS-ESLint | DIR | ✓ | Sometimes | Suggestion | Disallow async functions with no await expressions |
| `LU043` | `require-else-in-if-chain` | Destack | AST | ✓ | Sometimes | Unsafe | Require final else in if-else-if chains |
| `LU044` | `require-yield` | ESLint | AST | ✓ | No | None | Require generator functions to contain yield |
| `LU045` | `return-await` | TS-ESLint | DIR | ✓ | Always | Suggestion | Enforce consistent `return await` usage |

## Security (S)

Patterns that may expose the application to attacks.

[`src/rules/security/`](src/rules/security/)

| Code | Rule | Source | Level | Status | Autofix Support | Fixability | Description |
|------|------|--------|-------|--------|------------------|------------|-------------|
| `LS001` | `no-blank-target` | Biome | AST | ✓ | Sometimes | Safe | Disallow `target="_blank"` without `rel="noopener"` |
| `LS016` | `no-ffi-abi-mismatch` | Rust | DIR |  |  | None | Disallow FFI calls with ABI-unsafe layouts |
| `LS002` | `no-hardcoded-ip` | SonarQube | AST | ✓ | No | None | Disallow hardcoded IP addresses |
| `LS004` | `no-insecure-random` | Semgrep | DIR | ✓ | No | None | Disallow insecure random number generators |
| `LS005` | `no-open-redirect` | Semgrep | DIR | ✓ | No | None | Disallow tainted values in browser redirect APIs |
| `LS007` | `no-regex-injection` | Destack | DIR | ✓ | No | None | Disallow tainted values in dynamic regular expression patterns |
| `LS008` | `no-script-url` | ESLint | AST | ✓ | No | None | Disallow `javascript:` URLs |
| `LS009` | `no-secrets` | Biome | AST | ✓ | No | None | Disallow hardcoded secrets and credentials |
| `LS010` | `no-tainted-sink` | Destack | DIR | ✓ | No | None | Disallow passing tainted values into security sinks |

## Performance (P)

Correct code that could be faster or use less memory.

[`src/rules/performance/`](src/rules/performance/)

| Code | Rule | Source | Level | Status | Autofix Support | Fixability | Description |
|------|------|--------|-------|--------|------------------|------------|-------------|
| `LP027` | `large-stack-arrays` | Clippy | MIR |  |  | Suggestion | Warn on large stack allocations that should be heap allocated |
| `LP028` | `large-types-passed-by-value` | Clippy | DIR |  |  | Suggestion | Warn on passing large structs or arrays by value |
| `LP033` | `no-ambiguous-type` | Destack | DIR |  |  | Suggestion | Warn on types that force dynamic dispatch unnecessarily |
| `LP019` | `no-alloc-in-loop` | Clippy | MIR |  |  | None | Disallow heap allocations inside loops |
| `LP002` | `no-array-for-each` | Unicorn | DIR | ✓ | Sometimes | Unsafe | Prefer for-of over `Array.forEach()` |
| `LP003` | `no-array-unshift-loop` | Destack | DIR | ✓ | No | None | Disallow `unshift` in loops (causes O(n²) reallocations) |
| `LP004` | `no-await-in-loop` | ESLint | AST | ✓ | No | None | Disallow await inside of loops |
| `LP005` | `no-barrel-file` | Biome | AST | ✓ | No | None | Disallow barrel files that re-export everything |
| `LP006` | `no-json-clone` | Destack | DIR | ✓ | Sometimes | Unsafe | Disallow `JSON.parse(JSON.stringify())` for cloning |
| `LP007` | `no-nested-array-includes` | Destack | DIR | ✓ | No | None | Disallow `includes`/`indexOf` inside loops over another array |
| `LP009` | `no-regex-in-loop` | Destack | DIR | ✓ | No | None | Disallow `RegExp()` construction inside loops |
| `LP020` | `no-sequential-independent-await` | Destack | DIR | ✓ | No | None | Suggest `Promise.all` for independent sequential awaits |
| `LP010` | `no-string-concat-in-loop` | Destack | DIR | ✓ | No | None | Disallow `+=` and `x = x + y` string concatenation in loops |
| `LP011` | `no-super-linear-regex` | Destack | AST | ✓ | No | None | Disallow regular expressions with catastrophic backtracking |
| `LP012` | `prefer-array-every` | Unicorn | DIR | ✓ | Sometimes | Safe | Prefer `.every()` over `.filter().length === .length` |
| `LP013` | `prefer-array-literal` | Destack | DIR | ✓ | Sometimes | Unsafe | Suggest using array literal instead of empty array followed by extend |
| `LP014` | `prefer-for-of` | TS-ESLint | DIR | ✓ | Sometimes | Unsafe | Prefer for-of loops over index-based for loops |
| `LP015` | `prefer-includes` | TS-ESLint | DIR | ✓ | Sometimes | Safe | Prefer `.includes()` over `.indexOf() !== -1` |
| `LP021` | `prefer-reserve` | Clippy | DIR |  |  | Suggestion | Prefer reserving capacity when the size is known |
| `LP016` | `prefer-string-endswith` | Unicorn | DIR | ✓ | Sometimes | Safe | Prefer `.endsWith()` over `.slice(-n) === suffix` |
| `LP017` | `prefer-string-startswith` | Unicorn | DIR | ✓ | Sometimes | Safe | Prefer `.startsWith()` over `.indexOf() === 0` |
| `LP018` | `require-unicode-regexp` | ESLint | AST | ✓ | Sometimes | Safe | Require configured Unicode regex flag on regular expressions |

## Style (Y)

Subjective preferences for consistent coding style.

[`src/rules/style/`](src/rules/style/)

| Code    | Rule                                 | Source     | Level | Status | Autofix Support | Fixability | Description                                                         |
| ------- | ------------------------------------ | ---------- | ----- | ------ | --------------- | ---------- | ------------------------------------------------------------------- |
| `LY070` | `array-type`                         | TS-ESLint  | DIR   | ✓      | Always          | Safe       | Require consistently using either `T[]` or `Array<T>`               |
| `LY001` | `catch-error-name`                   | Unicorn    | AST   | ✓      | Sometimes       | Safe       | Enforce a specific name for catch clause error parameters           |
| `LY002` | `comment-casing`                     | Destack    | AST   | ✓      | Sometimes       | Safe       | Enforce comment / doc casing                                        |
| `LY003` | `comment-layout`                     | Destack    | AST   | ✓      | Sometimes       | Safe       | Enforce comment / doc layout                                        |
| `LY004` | `comment-punctuation`                | Destack    | AST   | ✓      | Sometimes       | Safe       | Enforce comment / doc punctuation style                             |
| `LY005` | `consistent-extension-style`         | Destack    | AST   | ✓      | No              | None       | Enforce consistent use of named or anonymous extensions             |
| `LY006` | `consistent-type-definitions`        | TS-ESLint  | AST   | ✓      | Sometimes       | Unsafe     | Enforce type definitions to use either `interface` or `type`        |
| `LY007` | `consistent-type-imports`            | TS-ESLint  | DIR   | ✓      | Sometimes       | Safe       | Enforce consistent usage of type imports                            |
| `LY008` | `default-param-last`                 | ESLint     | AST   | ✓      | Sometimes       | Unsafe     | Enforce default parameters to be last                               |
| `LY009` | `dot-notation`                       | ESLint     | AST   | ✓      | Sometimes       | Safe       | Enforce dot notation whenever possible                              |
| `LY011` | `explicit-function-return-type`      | TS-ESLint  | AST   | ✓      | Sometimes       | Safe       | Require explicit return types on functions                          |
| `LY012` | `filename-case`                      | Unicorn    | AST   | ✓      | No              | None       | Enforce a case style for filenames                                  |
| `LY013` | `grouped-accessor-pairs`             | ESLint     | AST   | ✓      | Sometimes       | Unsafe     | Require grouped accessor pairs in object literals and classes       |
| `LY014` | `no-boolean-literal-compare`         | Unicorn    | AST   | ✓      | Always          | Safe       | Disallow comparing boolean expressions to boolean literals          |
| `LY015` | `no-collapsible-if`                  | Unicorn    | AST   | ✓      | Always          | Safe       | Suggest merging nested if statements without else                   |
| `LY016` | `no-duplicate-string`                | SonarQube  | AST   | ✓      | No              | None       | Disallow the same string literal appearing many times               |
| `LY072` | `no-duplicate-type-constituents`     | TS-ESLint  | DIR   | ✓      | Sometimes       | Safe       | Disallow duplicate constituents in union/intersection types         |
| `LY017` | `no-else-return`                     | ESLint     | AST   | ✓      | Sometimes       | Safe       | Disallow else blocks after return statements                        |
| `LY018` | `no-empty-interface`                 | TS-ESLint  | DIR   | ✓      | Always          | Safe       | Disallow empty interfaces                                           |
| `LY067` | `no-extra-boolean-cast`              | ESLint     | DIR   | ✓      | Sometimes       | Safe       | Disallow unnecessary boolean casts                                  |
| `LY019` | `no-lonely-if`                       | ESLint     | AST   | ✓      | Always          | Safe       | Disallow if statements as the only statement in else blocks         |
| `LY020` | `no-negated-condition`               | ESLint     | AST   | ✓      | Always          | Safe       | Disallow negated conditions with else branches                      |
| `LY074` | `no-redundant-type-constituents`     | TS-ESLint  | DIR   | ✓      | Sometimes       | Safe       | Disallow type constituents made redundant by others                 |
| `LY021` | `no-nested-template-literal`         | SonarQube  | AST   | ✓      | No              | None       | Disallow template literals nested inside template literals          |
| `LY022` | `no-nested-ternary`                  | ESLint     | AST   | ✓      | Sometimes       | Unsafe     | Disallow nested ternary expressions                                 |
| `LY025` | `no-unneeded-ternary`                | ESLint     | AST   | ✓      | Sometimes       | Safe       | Disallow ternary operators when simpler alternatives exist          |
| `LY068` | `no-unnecessary-template-expression` | TS-ESLint  | DIR   | ✓      | Always          | Safe       | Disallow unnecessary template literal expressions                   |
| `LY024` | `no-unnecessary-lambda`              | ErrorProne | DIR   | ✓      | Always          | Safe       | Disallow lambdas that only wrap a direct function call              |
| `LY027` | `object-shorthand`                   | ESLint     | AST   | ✓      | Sometimes       | Safe       | Require or disallow method and property shorthand syntax            |
| `LY028` | `operator-assignment`                | ESLint     | AST   | ✓      | Sometimes       | Safe       | Require or disallow assignment operator shorthand                   |
| `LY033` | `prefer-arrow-callback`              | ESLint     | DIR   | ✓      | Always          | Safe       | Require arrow functions as callbacks                                |
| `LY034` | `prefer-as-const`                    | TS-ESLint  | AST   | ✓      | Sometimes       | Safe       | Prefer `as const` over literal type assertions                      |
| `LY035` | `prefer-const`                       | ESLint     | DIR   | ✓      | Sometimes       | Safe       | Require `const` declarations for never-reassigned variables         |
| `LY036` | `prefer-exponentiation-operator`     | ESLint     | DIR   | ✓      | Sometimes       | Safe       | Prefer `**` over `Math.pow()`                                       |
| `LY037` | `prefer-expression`                  | Destack    | AST   | ✓      | Sometimes       | Safe       | Prefer expression syntax for assignments                            |
| `LY039` | `prefer-fragment-shorthand`          | Destack    | AST   | ✓      | Always          | Safe       | Prefer `<>` shorthand over `<Fragment>`                             |
| `LY040` | `prefer-if-else-over-match-bool`     | Destack    | AST   | ✓      | Sometimes       | Safe       | Suggest using if/else instead of match on booleans                  |
| `LY041` | `prefer-implicit-return`             | Destack    | AST   | ✓      | Sometimes       | Safe       | Prefer implicit returns in expression-bodied functions              |
| `LY043` | `prefer-loop`                        | Destack    | AST   | ✓      | Always          | Safe       | Prefer `loop` keyword over `while(true)` or `for(;;)`               |
| `LY045` | `prefer-named-extension`             | Destack    | AST   | ✓      | No              | None       | Prefer named extensions for foreign types                           |
| `LY046` | `prefer-nullish-coalescing`          | TS-ESLint  | DIR   | ✓      | Sometimes       | Safe       | Prefer `??` over `\                                                 |
| `LY047` | `prefer-numeric-literals`            | ESLint     | DIR   | ✓      | Sometimes       | Safe       | Prefer numeric literals over `parseInt()`                           |
| `LY048` | `prefer-object-spread`               | ESLint     | DIR   | ✓      | Sometimes       | Safe       | Prefer spread operator over `Object.assign()`                       |
| `LY049` | `prefer-pattern-over-guard`          | Destack    | DIR   | ✓      | Sometimes       | Safe       | Suggest moving match guards into the pattern                        |
| `LY050` | `prefer-precise-numeric`             | Destack    | AST   | ✓      | Always          | Suggestion | Prefer precise numeric types over `number`                          |
| `LY076` | `prefer-promise-reject-errors`       | TS-ESLint  | DIR   | ✓      | No              | None       | Require Error objects in Promise rejections                         |
| `LY077` | `prefer-propagate-operator`          | Destack    | DIR   |        |                 | Safe       | Prefer `?` propagation over manual Result matching                  |
| `LY078` | `prefer-readonly`                    | TS-ESLint  | DIR   | ✓      | No              | None       | Prefer `readonly` for non-mutated fields                            |
| `LY053` | `prefer-self-closing-tree`           | Destack    | AST   | ✓      | Always          | Safe       | Prefer self-closing tree elements when possible                     |
| `LY079` | `prefer-set-over-empty-map`          | Destack    | DIR   | ✓      | No              | None       | Suggest `Set<K>` over `Map<K, void>`                                |
| `LY055` | `prefer-struct`                      | Destack    | DIR   | ✓      | Sometimes       | Unsafe     | Prefer struct for data-only classes                                 |
| `LY054` | `prefer-string-replaceall`           | Unicorn    | DIR   | ✓      | Sometimes       | Safe       | Prefer `.replaceAll()` over `.replace()` with global regex          |
| `LY056` | `prefer-struct-literal`              | Destack    | DIR   | ✓      | Sometimes       | Safe       | Prefer struct literal syntax over constructor calls                 |
| `LY057` | `prefer-template`                    | ESLint     | AST   | ✓      | Always          | Safe       | Prefer template literals over string concatenation                  |
| `LY058` | `prefer-tuple`                       | Destack    | AST   | ✓      | Always          | Safe       | Suggest tuple type for fixed-length heterogeneous arrays            |
| `LY059` | `prefer-tuple-destructure`           | Destack    | DIR   | ✓      | Sometimes       | Safe       | Prefer tuple destructuring over indexed access                      |
| `LY060` | `prefer-tuple-swap`                  | Destack    | AST   | ✓      | Always          | Safe       | Prefer tuple swap syntax over temporary variable                    |
| `LY061` | `prefer-unary-negation`              | Destack    | AST   | ✓      | Always          | Safe       | Prefer unary negation over multiplying by -1                        |
| `LY080` | `promise-function-async`             | TS-ESLint  | DIR   | ✓      | No              | None       | Require `async` keyword for Promise-returning functions             |
| `LY062` | `require-jsdoc`                      | ESLint     | AST   | ✓      | No              | None       | Require documentation on public items                               |
| `LY063` | `require-returns-doc`                | ESLint     | AST   | ✓      | No              | None       | Require return type documentation                                   |
| `LY081` | `restrict-template-expressions`      | TS-ESLint  | DIR   | ✓      | No              | None       | Require template expressions to be strings or numbers               |
| `LY064` | `sort-imports`                       | ESLint     | AST   | ✓      | Sometimes       | Safe       | Enforce sorted import declarations                                  |
| `LY065` | `symbol-description`                 | ESLint     | DIR   | ✓      | Always          | Suggestion | Require symbol descriptions                                         |
| `LY066` | `yoda`                               | ESLint     | AST   | ✓      | Sometimes       | Safe       | Disallow Yoda conditions                                            |
| `LY032` | `prefer-array-some`                  | Unicorn    | DIR   | ✓      | Sometimes       | Safe       | Prefer `some()` over `filter().length` or `findIndex()` comparisons |
| `LY029` | `prefer-array-filter`                | Destack    | DIR   | ✓      | Sometimes       | Unsafe     | Suggest `.filter()` over `forEach` with conditional push            |
| `LY030` | `prefer-array-find`                  | Unicorn    | DIR   | ✓      | Sometimes       | Safe       | Suggest `.find()` over `.filter()[0]`                               |
| `LY031` | `prefer-array-map`                   | Destack    | DIR   | ✓      | Sometimes       | Unsafe     | Suggest `.map()` over `forEach` with push                           |
| `LY038` | `prefer-flat-map`                    | Unicorn    | DIR   | ✓      | Sometimes       | Safe       | Suggest `.flatMap()` over `.map().flat()`                           |
| `LY044` | `prefer-match`                       | Destack    | AST   | ✓      | Sometimes       | Safe       | Suggest match expressions over complex if-else chains               |

## Complexity (X)

Overly complex code that is harder to understand and maintain.

[`src/rules/complexity/`](src/rules/complexity/)

| Code | Rule | Source | Level | Status | Autofix Support | Fixability | Description |
|------|------|--------|-------|--------|------------------|------------|-------------|
| `LX001` | `cognitive-complexity` | Biome | AST | ✓ | No | None | Enforce a maximum cognitive complexity |
| `LX002` | `cyclomatic-complexity` | ESLint | AST | ✓ | No | None | Enforce a maximum cyclomatic complexity |
| `LX003` | `max-branching-factor` | Destack | AST | ✓ | No | None | Enforce a maximum branching factor in conditionals |
| `LX004` | `max-depth` | ESLint | AST | ✓ | No | None | Enforce a maximum depth of nested blocks |
| `LX011` | `max-static-params` | Destack | AST | ✓ | No | None | Enforce a maximum number of static parameters |
| `LX005` | `max-lines` | ESLint | AST | ✓ | No | None | Enforce a maximum number of lines per file |
| `LX006` | `max-lines-per-function` | ESLint | AST | ✓ | No | None | Enforce a maximum number of lines per function |
| `LX007` | `max-nested-callbacks` | ESLint | AST | ✓ | No | None | Enforce a maximum depth of nested callbacks |
| `LX008` | `max-params` | ESLint | AST | ✓ | No | None | Enforce a maximum number of function parameters |
| `LX009` | `max-return-statements` | Code Climate | AST | ✓ | No | None | Enforce a maximum number of return statements per function |
| `LX010` | `max-statements` | ESLint | AST | ✓ | No | None | Enforce a maximum number of statements per function |
| `LX012` | `max-switch-cases` | SonarQube | AST | ✓ | No | None | Enforce a maximum number of cases in a switch statement |
| `LX013` | `max-type-fields` | Destack | AST | ✓ | No | None | Enforce a maximum number of fields in a struct, class, interface, or object type |
| `LX014` | `max-type-variants` | Destack | AST | ✓ | No | None | Enforce a maximum number of variants in a union type or enum |
| `LX015` | `no-complex-boolean-expression` | Destack | AST | ✓ | No | None | Suggest simplifying complex boolean expressions |
| `LX016` | `no-complex-type` | Destack | AST | ✓ | No | None | Warn on overly complex types that should be aliased |
| `LX017` | `no-duplicate-code` | Code Climate | AST | ✓ | No | None | Warn on duplicate or near-duplicate code blocks |
| `LX018` | `no-excessive-booleans` | Destack | AST | ✓ | No | None | Disallow too many boolean parameters or struct fields |
| `LX019` | `no-multi-assign` | ESLint | AST | ✓ | Sometimes | Safe | Disallow chained assignment expressions |
| `LX020` | `no-multi-declarators` | ESLint | AST | ✓ | Sometimes | Safe | Disallow multiple variable declarations per statement |
| `LX021` | `no-nested-switch` | SonarQube | AST | ✓ | No | None | Disallow switch statements nested inside switch statements (or match) |
| `LX022` | `no-unused-expressions` | ESLint | AST | ✓ | No | None | Disallow expressions that have no effect |
| `LX023` | `no-useless-underscore-binding` | Destack | AST | ✓ | No | None | Warn on underscore bindings with no side effects |
| `LX024` | `prefer-expression-over-let-if` | Destack | AST | ✓ | Sometimes | Suggestion | Suggest expression syntax over let-if sequences |
| `LX025` | `prefer-if-let` | Destack | AST | ✓ | Sometimes | Safe | Suggest if-let over single-arm match |
| `LX026` | `prefer-simplified-comparison` | Destack | AST | ✓ | Always | Safe | Suggest simplifying comparisons like `x >= y + 1` |

## Restriction (R)

Opt-in rules that ban certain patterns by project choice. These rules may conflict with each other.

[`src/rules/restriction/`](src/rules/restriction/)

| Code | Rule | Source | Level | Status | Autofix Support | Fixability | Description |
|------|------|--------|-------|--------|------------------|------------|-------------|
| `LR001` | `no-alert` | ESLint | DIR | ✓ | Sometimes | Unsafe | Disallow the use of `alert`, `confirm`, and `prompt` |
| `LR002` | `no-anonymous-default-export` | Unicorn | AST | ✓ | Sometimes | Suggestion | Disallow anonymous default exports |
| `LR003` | `no-banned-import` | ESLint | DIR | ✓ | No | None | Disallow imports from specified modules |
| `LR004` | `no-bitwise` | ESLint | AST | ✓ | No | None | Disallow bitwise operators |
| `LR005` | `no-circular-dependency` | Import | DIR | ✓ | No | None | Disallow circular module dependencies |
| `LR006` | `no-class` | Destack | AST | ✓ | No | None | Disallow class declarations (prefer structs) |
| `LR007` | `no-console` | ESLint | DIR | ✓ | Sometimes | Safe | Disallow the use of `console` |
| `LR008` | `no-continue` | ESLint | AST | ✓ | No | None | Disallow `continue` statements |
| `LR009` | `no-default-export` | Import | DIR | ✓ | Sometimes | Unsafe | Disallow default exports |
| `LR024` | `no-re-export-all` | Biome | AST | ✓ | No | None | Disallow `export * from` (hurts tree-shaking) |
| `LR011` | `no-enum` | Biome | AST | ✓ | No | None | Disallow TypeScript enums (prefer union types) |
| `LR014` | `no-implicit-return` | Destack | AST | ✓ | Always | Safe | Require explicit `return` statements |
| `LR015` | `no-labels` | ESLint | AST | ✓ | No | None | Disallow labeled statements |
| `LR036` | `no-layer-violation` | SonarQube | DIR | ✓ | No | None | Disallow imports that cross configured module component boundaries |
| `LR016` | `no-magic-numbers` | ESLint | AST | ✓ | No | None | Disallow magic numbers |
| `LR020` | `no-parameter-reassignment` | SonarQube | DIR | ✓ | Sometimes | Unsafe | Disallow reassigning function parameters |
| `LR021` | `no-placeholder-implementation` | ESLint | AST | ✓ | No | None | Disallow placeholder implementations (throw "not implemented", etc.) |
| `LR022` | `no-plusplus` | ESLint | AST | ✓ | Sometimes | Safe | Disallow `++` and `--` operators |
| `LR023` | `no-process-exit` | Unicorn | DIR | ✓ | Sometimes | Unsafe | Disallow `process.exit()` |
| `LR034` | `no-relative-parent-imports` | Import | DIR | ✓ | No | None | Disallow relative parent path imports |
| `LR025` | `no-sequences` | ESLint | AST | ✓ | Sometimes | Safe | Disallow comma operators |
| `LR026` | `no-shadow` | ESLint | DIR | ✓ | Sometimes | Unsafe | Disallow shadowing by rebinding a value |
| `LR027` | `no-struct` | Destack | AST | ✓ | No | None | Disallow struct declarations (prefer classes) |
| `LR028` | `no-ternary` | ESLint | AST | ✓ | No | None | Disallow ternary operators |
| `LR029` | `no-unused-modules` | Import | DIR | ✓ | No | None | Disallow exports that are never imported by any module |
| `LR030` | `no-warning-comments` | ESLint | AST | ✓ | Always | Suggestion | Disallow specified warning terms in comments (TODO, FIXME, etc.) |
| `LR031` | `no-wildcard-imports` | Destack | AST | ✓ | Sometimes | Unsafe | Disallow wildcard imports |
| `LR012` | `no-exceptions` | Destack | DIR | ✓ | No | None | Disallow `throw` and `try/catch` (use Result types) |

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_linter
just language/test-linter

# clean check
just language/check-quick

# exhaustive check
just language/check-full
```
