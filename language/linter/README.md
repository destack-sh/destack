# Destack Linter

Static analysis rules for Destack (`.ds`, `.ts`, `.tsx`, `.js`, `.jsx`) files.
Lints run at any of the IR levels (AST, DIR, MIR), usually per module.

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

---

## Correctness (C)

High-confidence issues that are almost always wrong.

[`src/rules/correctness/`](src/rules/correctness/)

| Rule | Source | Level | Ready | Status | Fixability | Description |
|------|--------|-------|-------|--------|------------|-------------|
| `await-holding-lock` | Clippy | DIR | ✗ | 🔶 | None | Disallow holding a mutex lock across an await point |
| `await-thenable` | TS-ESLint | DIR | ✗ | 🔶 | Safe | Disallow awaiting a value that is not a Promise |
| `for-direction` | ESLint | AST | ✓ | ✅ | None | Enforce for loop update clause moving in the correct direction |
| `no-approx-constant` | Destack | AST | ✓ | ✅ | Safe | Disallow approximate representations of mathematical constants |
| `no-array-delete` | TS-ESLint | DIR | ✓ | 🔶 | None | Disallow `delete` on arrays (creates holes) |
| `no-async-promise-executor` | ESLint | DIR | ✗ | 🔶 | Unsafe | Disallow async functions as Promise executor |
| `no-base-to-string` | TS-ESLint | DIR | ✓ | 🔶 | Suggestion | Disallow `.toString()` on objects without useful representation |
| `no-class-assign` | ESLint | DIR | ✓ | 🔶 | None | Disallow reassigning class/struct declarations |
| `no-compare-neg-zero` | ESLint | AST | ✓ | ✅ | Safe | Disallow comparing against negative zero |
| `no-const-assign` | ESLint | DIR | ✓ | 🔶 | None | Disallow reassigning const variables |
| `no-constant-binary-expression` | ESLint | AST | ✓ | ✅ | Unsafe | Disallow expressions where the operation doesn't affect the value |
| `no-constant-condition` | ESLint | AST | ✓ | ✅ | None | Disallow constant expressions in conditions |
| `no-control-regex` | ESLint | AST | ✓ | ✅ | None | Disallow control characters in regular expressions |
| `no-deprecated` | TS-ESLint | DIR | ✗ | 🔶 | Suggestion | Disallow use of `@deprecated` APIs |
| `no-duplicate-case` | ESLint | AST | ✓ | ✅ | None | Disallow duplicate case labels |
| `no-empty-range` | Destack | AST | ✓ | ✅ | None | Disallow empty ranges where start > end |
| `no-fallthrough` | ESLint | AST | ✓ | ✅ | Suggestion | Disallow fallthrough of case statements |
| `no-floating-promises` | TS-ESLint | DIR | ✗ | 🔶 | Suggestion | Require Promises to be awaited or returned |
| `no-for-in-array` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Disallow iterating over arrays with for-in |
| `no-func-assign` | ESLint | DIR | ✓ | 🔶 | None | Disallow reassigning function declarations |
| `no-infinite-iterator` | Destack | DIR | ✗ | 🔶 | None | Disallow using methods that produce infinite iterators |
| `no-invalid-regexp` | ESLint | AST | ✓ | ✅ | None | Disallow invalid regular expression strings |
| `no-misused-promises` | TS-ESLint | DIR | ✗ | 🔶 | None | Disallow Promises in places not designed to handle them |
| `no-misused-spread` | TS-ESLint | DIR | ✓ | 🔶 | None | Disallow spread syntax in contexts where it's incorrect |
| `no-new-native-nonconstructor` | ESLint | DIR | ✗ | 🔶 | Safe | Disallow `new` on Symbol and BigInt |
| `no-obj-calls` | ESLint | DIR | ✗ | 🔶 | None | Disallow calling global objects as functions |
| `no-overlapping-match-arms` | Destack | DIR | ✓ | 🔶 | Safe | Disallow match patterns that subsume later arms |
| `no-promise-executor-return` | ESLint | DIR | ✗ | 🔶 | Safe | Disallow returning values from Promise executor |
| `no-self-compare` | ESLint | AST | ✓ | ✅ | None | Disallow comparisons where both sides are exactly the same |
| `no-sparse-arrays` | ESLint | AST | ✓ | ✅ | None | Disallow sparse arrays with holes |
| `no-struct-identity-compare` | Destack | DIR | ✓ | 🔶 | Safe | Disallow identity comparison on value types |
| `no-this-before-super` | ESLint | DIR | ✓ | 🔶 | None | Disallow `this` before calling `super()` in constructors |
| `no-unhandled-result` | Destack | DIR | ✓ | 🔶 | Suggestion | Require Result values to be handled |
| `no-unsafe-finally` | ESLint | AST | ✓ | ✅ | Safe | Disallow control flow statements in finally blocks |
| `no-unsafe-negation` | ESLint | AST | ✓ | ✅ | Safe | Disallow negating the left operand of relational operators |
| `no-unsafe-optional-chaining` | ESLint | DIR | ✓ | 🔶 | None | Disallow optional chaining in contexts where undefined is not allowed |
| `no-useless-assignment` | ESLint | DIR | ✓ | 🔶 | Safe | Disallow assignments that are immediately overwritten |
| `require-array-sort-compare` | TS-ESLint | DIR | ✓ | 🔶 | Suggestion | Require comparison function for `.sort()` |
| `switch-exhaustiveness-check` | TS-ESLint | DIR | ✓ | 🔶 | Suggestion | Require switch statements to be exhaustive |
| `unbound-method` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Disallow unbound methods as callbacks |
| `unused-must-use` | Destack | DIR | ✓ | 🔶 | Suggestion | Disallow ignoring return values of `@mustUse` functions |
| `use-isnan` | ESLint | AST | ✓ | ✅ | Safe | Require `Number.isNaN()` instead of comparisons with `NaN` |

## Suspicious (U)

Code that is likely unintentional but may occasionally be intentional.

[`src/rules/suspicious/`](src/rules/suspicious/)

| Rule | Source | Level | Ready | Status | Fixability | Description |
|------|--------|-------|-------|--------|------------|-------------|
| `no-cond-assign` | ESLint | AST | ✓ | ✅ | Suggestion | Disallow assignment operators in conditional expressions |
| `no-confusing-assignment` | Destack | AST | ✓ | ✅ | Suggestion | Warn on assignments that look like comparisons |
| `no-confusing-non-null-assertion` | TS-ESLint | AST | ✓ | ✅ | Safe | Disallow non-null assertions after optional chain expressions |
| `no-constant-assertion` | Destack | AST | ✓ | ✅ | Safe | Disallow assertions on constant values |
| `no-constructor-return` | ESLint | AST | ✓ | ✅ | Safe | Disallow returning values from constructors |
| `no-debugger` | ESLint | AST | ✓ | ✅ | Safe | Disallow debugger statements |
| `no-dupe-else-if` | ESLint | AST | ✓ | ✅ | None | Disallow duplicate conditions in if-else-if chains |
| `no-duplicate-match-arms` | Destack | AST | ✓ | ✅ | None | Warn on match arms with identical bodies |
| `no-empty` | ESLint | AST | ✓ | ✅ | Suggestion | Disallow empty block statements |
| `no-empty-function` | ESLint | AST | ✓ | ✅ | Suggestion | Disallow empty functions |
| `no-empty-pattern` | ESLint | AST | ✓ | ✅ | None | Disallow empty destructuring patterns |
| `no-empty-static-block` | ESLint | AST | ✓ | ✅ | Safe | Disallow empty static initialization blocks in classes |
| `no-ex-assign` | ESLint | DIR | ✓ | 🔶 | None | Disallow reassigning exceptions in catch clauses |
| `no-extra-non-null-assertion` | TS-ESLint | AST | ✓ | ✅ | Safe | Disallow extra non-null assertions |
| `no-global-assign` | ESLint | DIR | ✗ | 🔶 | None | Disallow assignments to native objects or read-only globals |
| `no-implicit-void-expression` | Destack | AST | ✗ | 🔶 | Safe | Disallow implicit void returns from expression blocks |
| `no-incomplete-range` | Destack | AST | ✓ | ✅ | Safe | Warn on exclusive ranges that are likely meant to be inclusive |
| `no-inner-declarations` | ESLint | AST | ✓ | ✅ | None | Disallow variable or function declarations in nested blocks |
| `no-loop-func` | ESLint | DIR | ✓ | 🔶 | None | Disallow functions that capture loop variables |
| `no-method-shadowing` | Destack | DIR | ✓ | 🔶 | None | Warn when a method shadows an inherited method |
| `no-misleading-character-class` | ESLint | AST | ✓ | ✅ | Suggestion | Disallow characters that behave unexpectedly in regex |
| `no-negation-in-equality-check` | Unicorn | AST | ✓ | ✅ | Safe | Disallow negation in the left operand of equality tests |
| `no-prototype-builtins` | ESLint | DIR | ✗ | 🔶 | Safe | Disallow calling Object.prototype methods directly on objects |
| `no-redundant-match-guard` | Destack | DIR | ✓ | 🔶 | Safe | Disallow match guards that are always true or false |
| `no-redundant-pattern` | Destack | AST | ✓ | ✅ | Safe | Disallow patterns that bind nothing useful |
| `no-return-assign` | ESLint | AST | ✓ | ✅ | None | Disallow assignment operators in return statements |
| `no-self-assign` | ESLint | AST | ✓ | ✅ | Safe | Disallow assignments where both sides are exactly the same |
| `no-shadow-restricted-names` | ESLint | DIR | ✓ | 🔶 | None | Disallow shadowing of restricted names |
| `no-single-element-tuple` | Destack | AST | ✓ | ✅ | Safe | Warn on single-element tuples that may be accidental |
| `no-template-curly-in-string` | ESLint | AST | ✓ | ✅ | Safe | Disallow template literal placeholder syntax in regular strings |
| `no-throw-literal` | ESLint | DIR | ✓ | 🔶 | Suggestion | Disallow throwing literals instead of Error objects |
| `no-unexpected-multiline` | ESLint | AST | ✗ | 🔶 | Safe | Disallow confusing multiline expressions |
| `no-unnecessary-clone` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Warn on cloning values that are not used afterward |
| `no-unnecessary-type-assertion` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Disallow type assertions that do not change the type |
| `no-unsafe-declaration-merging` | TS-ESLint | DIR | ✓ | 🔶 | None | Disallow unsafe declaration merging |
| `no-unused-except-recursion` | Destack | DIR | ✓ | 🔶 | Suggestion | Warn on function arguments only used for recursion |
| `no-useless-backreference` | ESLint | AST | ✓ | ✅ | Safe | Disallow useless backreferences in regular expressions |
| `no-useless-catch` | ESLint | AST | ✓ | ✅ | Safe | Disallow catch clauses that only rethrow |
| `no-useless-computed-key` | ESLint | AST | ✓ | ✅ | Safe | Disallow unnecessary computed property keys |
| `no-useless-concat` | ESLint | AST | ✓ | ✅ | Safe | Disallow unnecessary concatenation of literals |
| `no-useless-constructor` | ESLint | AST | ✓ | ✅ | Safe | Disallow unnecessary constructors |
| `no-useless-escape` | ESLint | AST | ✓ | ✅ | Safe | Disallow unnecessary escape characters |
| `no-useless-rename` | ESLint | AST | ✓ | ✅ | Safe | Disallow renaming imports/exports to the same name |
| `no-useless-return` | ESLint | AST | ✓ | ✅ | Safe | Disallow redundant return statements |
| `prefer-array-filter` | Unicorn | DIR | ✓ | 🔶 | Unsafe | Suggest `.filter()` over manual filtering loops |
| `prefer-array-find` | Unicorn | DIR | ✓ | 🔶 | Unsafe | Suggest `.find()` over manual search loops |
| `prefer-array-map` | Destack | DIR | ✓ | 🔶 | Unsafe | Suggest `.map()` over manual mapping loops |
| `prefer-flat-map` | Unicorn | DIR | ✓ | 🔶 | Safe | Suggest `.flatMap()` over `.map().flatten()` |
| `prefer-match` | Destack | AST | ✓ | ✅ | Unsafe | Suggest match expressions over complex if-else chains |
| `require-await` | TS-ESLint | AST | ✓ | ✅ | Safe | Disallow async functions with no await expressions |
| `require-else-in-if-chain` | Destack | AST | ✓ | ✅ | Suggestion | Require final else in if-else-if chains |
| `require-yield` | ESLint | AST | ✓ | ✅ | None | Require generator functions to contain yield |
| `return-await` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Enforce consistent `return await` usage |
| `guard-for-in` | ESLint | AST | ✓ | ✅ | Suggestion | Require `hasOwnProperty` guard in for-in loops |

## Security (S)

Patterns that may expose the application to attacks.

[`src/rules/security/`](src/rules/security/)

| Rule | Source | Level | Ready | Status | Fixability | Description |
|------|--------|-------|-------|--------|------------|-------------|
| `no-blank-target` | Biome | AST | ✓ | ✅ | Safe | Disallow `target="_blank"` without `rel="noopener"` |
| `no-eval` | ESLint | DIR | ✗ | 🔶 | None | Disallow the use of `eval()` |
| `no-implied-eval` | ESLint | DIR | ✗ | 🔶 | Safe | Disallow `setTimeout` and `setInterval` with string arguments |
| `no-new-func` | ESLint | DIR | ✗ | 🔶 | None | Disallow `new Function()` |
| `no-script-url` | ESLint | AST | ✓ | ✅ | None | Disallow `javascript:` URLs |
| `no-secrets` | Biome | AST | ✓ | ✅ | None | Disallow hardcoded secrets and credentials |

## Performance (P)

Correct code that could be faster or use less memory.

[`src/rules/performance/`](src/rules/performance/)

| Rule | Source | Level | Ready | Status | Fixability | Description |
|------|--------|-------|-------|--------|------------|-------------|
| `no-accumulating-spread` | Biome | DIR | ✗ | 🔶 | Unsafe | Disallow spreading in accumulators (causes O(n²) allocations) |
| `no-array-for-each` | Unicorn | DIR | ✗ | 🔶 | Safe | Prefer for-of over `Array.forEach()` |
| `no-await-in-loop` | ESLint | AST | ✓ | ✅ | Suggestion | Disallow await inside of loops |
| `no-barrel-file` | Biome | AST | ✓ | ✅ | None | Disallow barrel files that re-export everything |
| `no-re-export-all` | Biome | AST | ✓ | ✅ | None | Disallow `export * from` (hurts tree-shaking) |
| `prefer-array-literal` | Destack | DIR | ✗ | 🔶 | Unsafe | Suggest using array literal instead of empty array followed by extend |
| `prefer-for-of` | TS-ESLint | DIR | ✗ | 🔶 | Safe | Prefer for-of loops over index-based for loops |
| `prefer-includes` | TS-ESLint | DIR | ✗ | 🔶 | Safe | Prefer `.includes()` over `.indexOf() !== -1` |
| `prefer-some-over-find` | Destack | DIR | ✗ | 🔶 | Safe | Prefer `.some()` over `.find() !== undefined` |
| `require-unicode-regexp` | ESLint | AST | ✓ | ✅ | Safe | Require `u` or `v` flag on regular expressions |

## Style (Y)

Subjective preferences for consistent coding style.

[`src/rules/style/`](src/rules/style/)

| Rule | Source | Level | Ready | Status | Fixability | Description |
|------|--------|-------|-------|--------|------------|-------------|
| `array-type` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Require consistently using either `T[]` or `Array<T>` |
| `catch-error-name` | Unicorn | AST | ✓ | ✅ | Safe | Enforce a specific name for catch clause error parameters |
| `comment-casing` | Destack | AST | ✓ | ✅ | Safe | Enforce comment / doc casing |
| `comment-layout` | Destack | AST | ✓ | ✅ | Safe | Enforce comment / doc layout |
| `comment-punctuation` | Destack | AST | ✓ | ✅ | Safe | Enforce comment / doc punctuation style |
| `consistent-extension-style` | Destack | AST | ✓ | ✅ | Unsafe | Enforce consistent use of named or anonymous extensions |
| `consistent-type-definitions` | TS-ESLint | AST | ✓ | ✅ | Safe | Enforce type definitions to use either `interface` or `type` |
| `consistent-type-imports` | TS-ESLint | AST | ✓ | ✅ | Safe | Enforce consistent usage of type imports |
| `default-param-last` | ESLint | AST | ✓ | ✅ | Unsafe | Enforce default parameters to be last |
| `dot-notation` | ESLint | AST | ✓ | ✅ | Safe | Enforce dot notation whenever possible |
| `eqeqeq` | ESLint | AST | ✓ | ✅ | Safe | Require `===` and `!==` |
| `explicit-function-return-type` | TS-ESLint | AST | ✓ | ✅ | Suggestion | Require explicit return types on functions |
| `filename-case` | Unicorn | AST | ✓ | ✅ | Unsafe | Enforce a case style for filenames |
| `grouped-accessor-pairs` | ESLint | AST | ✓ | ✅ | None | Require grouped accessor pairs in object literals and classes |
| `no-boolean-literal-compare` | Unicorn | AST | ✓ | ✅ | Safe | Disallow comparing boolean expressions to boolean literals |
| `no-collapsible-if` | Unicorn | AST | ✓ | ✅ | Safe | Suggest merging nested if statements without else |
| `no-duplicate-type-constituents` | TS-ESLint | AST | ✓ | ✅ | Safe | Disallow duplicate constituents in union/intersection types |
| `no-else-return` | ESLint | AST | ✓ | ✅ | Safe | Disallow else blocks after return statements |
| `no-empty-interface` | TS-ESLint | AST | ✓ | ✅ | Safe | Disallow empty interfaces |
| `no-extra-boolean-cast` | ESLint | DIR | ✗ | 🟡 | Safe | Disallow unnecessary boolean casts |
| `no-implicit-coercion` | ESLint | DIR | ✗ | 🟡 | Safe | Disallow shorthand type conversions |
| `no-lonely-if` | ESLint | AST | ✓ | ✅ | Safe | Disallow if statements as the only statement in else blocks |
| `no-negated-condition` | ESLint | AST | ✓ | ✅ | Safe | Disallow negated conditions with else branches |
| `no-redundant-type-constituents` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Disallow type constituents made redundant by others |
| `no-nested-ternary` | ESLint | AST | ✓ | ✅ | Unsafe | Disallow nested ternary expressions |
| `no-unneeded-ternary` | ESLint | AST | ✓ | ✅ | Safe | Disallow ternary operators when simpler alternatives exist |
| `no-unnecessary-template-expression` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Disallow unnecessary template literal expressions |
| `no-unnecessary-type-arguments` | TS-ESLint | DIR | ✓ | 🔶 | Safe | Disallow type arguments that equal the default |
| `no-var` | ESLint | AST | ✓ | ✅ | Safe | Require `let` or `const` instead of `var` |
| `object-shorthand` | ESLint | AST | ✓ | ✅ | Safe | Require or disallow method and property shorthand syntax |
| `operator-assignment` | ESLint | AST | ✓ | ✅ | Safe | Require or disallow assignment operator shorthand |
| `prefer-arrow-callback` | ESLint | AST | ✓ | ✅ | Safe | Require arrow functions as callbacks |
| `prefer-as-const` | TS-ESLint | AST | ✓ | ✅ | Safe | Prefer `as const` over literal type assertions |
| `prefer-const` | ESLint | DIR | ✓ | 🔶 | Safe | Require `const` declarations for never-reassigned variables |
| `prefer-destructuring` | ESLint | DIR | ✓ | 🔶 | Safe | Prefer destructuring from arrays and objects |
| `prefer-exponentiation-operator` | ESLint | DIR | ✗ | 🔶 | Safe | Prefer `**` over `Math.pow()` |
| `prefer-expression` | Destack | AST | ✓ | ✅ | Safe | Prefer expression syntax for assignments |
| `prefer-extension-method` | Destack | DIR | ✗ | 🔶 | Suggestion | Suggest converting functions to extension methods |
| `prefer-fragment-shorthand` | Destack | AST | ✓ | ✅ | Safe | Prefer `<>` shorthand over `<Fragment>` |
| `prefer-if-else-over-match-bool` | Destack | AST | ✓ | ✅ | Safe | Suggest using if/else instead of match on booleans |
| `prefer-implicit-return` | Destack | AST | ✓ | ✅ | Safe | Prefer implicit returns in expression-bodied functions |
| `prefer-inclusive-range` | Destack | AST | ✓ | ✅ | Safe | Prefer inclusive range syntax where applicable |
| `prefer-loop` | Destack | AST | ✓ | ✅ | Safe | Prefer `loop` keyword over `while(true)` or `for(;;)` |
| `prefer-map-or-else` | Destack | DIR | ✓ | 🔶 | Safe | Prefer `mapOrElse()` over `map().unwrap()` |
| `prefer-named-extension` | Destack | AST | ✓ | ✅ | Unsafe | Prefer named extensions for foreign types |
| `prefer-numeric-literals` | ESLint | DIR | ✗ | 🔶 | Safe | Prefer numeric literals over `parseInt()` |
| `prefer-object-has-own` | ESLint | DIR | ✗ | 🔶 | Safe | Prefer `Object.hasOwn()` over `Object.prototype.hasOwnProperty` |
| `prefer-object-spread` | ESLint | DIR | ✗ | 🔶 | Safe | Prefer spread operator over `Object.assign()` |
| `prefer-pattern-over-guard` | Destack | AST | ✓ | ✅ | Safe | Suggest moving match guards into the pattern |
| `prefer-precise-numeric` | Destack | AST | ✓ | ✅ | Suggestion | Prefer precise numeric types over `number` |
| `prefer-promise-reject-errors` | TS-ESLint | DIR | ✗ | 🔶 | Suggestion | Require Error objects in Promise rejections |
| `prefer-propagate-operator` | Destack | DIR | ✗ | 🟡 | Safe | Prefer `?` operator over manual Result matching |
| `prefer-range-contains` | Destack | AST | ✓ | ✅ | Safe | Prefer range contains method over comparison chains |
| `prefer-range-literal` | Destack | AST | ✓ | ✅ | Safe | Prefer range literals over C-style for loops |
| `prefer-result-type` | Destack | DIR | ✓ | 🔶 | Suggestion | Prefer `Result<T, E>` return type over throwing |
| `prefer-self-closing-tree` | Destack | AST | ✓ | ✅ | Safe | Prefer self-closing tree elements when possible |
| `prefer-set-over-empty-map` | Destack | DIR | ✗ | 🔶 | Safe | Suggest `Set<K>` over `Map<K, void>` |
| `prefer-struct` | Destack | AST | ✓ | ✅ | Unsafe | Prefer struct for data-only classes |
| `prefer-struct-literal` | Destack | AST | ✓ | ✅ | Safe | Prefer struct literal syntax over constructor calls |
| `prefer-template` | ESLint | AST | ✓ | ✅ | Safe | Prefer template literals over string concatenation |
| `prefer-tuple` | Destack | AST | ✓ | 🔶 | Safe | Suggest tuple type for fixed-length heterogeneous arrays |
| `prefer-tuple-destructure` | Destack | AST | ✓ | ✅ | Safe | Prefer tuple destructuring over indexed access |
| `prefer-tuple-swap` | Destack | AST | ✓ | ✅ | Safe | Prefer tuple swap syntax over temporary variable |
| `prefer-unary-negation` | Destack | AST | ✓ | ✅ | Safe | Prefer unary negation over multiplying by -1 |
| `promise-function-async` | TS-ESLint | DIR | ✗ | 🔶 | Safe | Require `async` keyword for Promise-returning functions |
| `require-jsdoc` | ESLint | AST | ✓ | ✅ | Suggestion | Require documentation on public items |
| `require-returns-doc` | ESLint | AST | ✓ | ✅ | Suggestion | Require return type documentation |
| `restrict-template-expressions` | TS-ESLint | DIR | ✓ | 🔶 | Suggestion | Require template expressions to be strings or numbers |
| `sort-imports` | ESLint | AST | ✓ | ✅ | Safe | Enforce sorted import declarations |
| `symbol-description` | ESLint | DIR | ✗ | 🔶 | Suggestion | Require symbol descriptions |
| `yoda` | ESLint | AST | ✓ | ✅ | Safe | Disallow Yoda conditions |

## Complexity (X)

Overly complex code that is harder to understand and maintain.

[`src/rules/complexity/`](src/rules/complexity/)

| Rule | Source | Level | Ready | Status | Fixability | Description |
|------|--------|-------|-------|--------|------------|-------------|
| `cognitive-complexity` | Biome | AST | ✓ | ✅ | None | Enforce a maximum cognitive complexity |
| `cyclomatic-complexity` | ESLint | AST | ✓ | ✅ | None | Enforce a maximum cyclomatic complexity |
| `max-depth` | ESLint | AST | ✓ | ✅ | None | Enforce a maximum depth of nested blocks |
| `max-lines` | ESLint | AST | ✓ | ✅ | None | Enforce a maximum number of lines per file |
| `max-lines-per-function` | ESLint | AST | ✓ | ✅ | None | Enforce a maximum number of lines per function |
| `max-nested-callbacks` | ESLint | AST | ✓ | ✅ | None | Enforce a maximum depth of nested callbacks |
| `max-params` | ESLint | AST | ✓ | ✅ | None | Enforce a maximum number of function parameters |
| `max-statements` | ESLint | AST | ✓ | ✅ | None | Enforce a maximum number of statements per function |
| `no-complex-boolean-expression` | Destack | AST | ✓ | ✅ | Safe | Suggest simplifying complex boolean expressions |
| `no-complex-type` | Destack | AST | ✓ | 🔶 | Suggestion | Warn on overly complex types that should be aliased |
| `no-excessive-booleans` | Destack | AST | ✓ | ✅ | Suggestion | Disallow too many boolean parameters or struct fields |
| `no-multi-assign` | ESLint | AST | ✓ | ✅ | Safe | Disallow chained assignment expressions |
| `no-multi-declarators` | ESLint | AST | ✓ | ✅ | Safe | Disallow multiple variable declarations per statement |
| `no-unused-expressions` | ESLint | AST | ✓ | ✅ | Safe | Disallow expressions that have no effect |
| `no-useless-underscore-binding` | Destack | AST | ✓ | ✅ | Safe | Warn on underscore bindings with no side effects |
| `prefer-expression-over-let-if` | Destack | AST | ✓ | ✅ | Safe | Suggest expression syntax over let-if sequences |
| `prefer-if-let` | Destack | AST | ✓ | ✅ | Safe | Suggest if-let over single-arm match |
| `prefer-simplified-comparison` | Destack | AST | ✓ | ✅ | Safe | Suggest simplifying comparisons like `x >= y + 1` |

## Restriction (R)

Opt-in rules that ban certain patterns by project choice. These rules may conflict with each other.

[`src/rules/restriction/`](src/rules/restriction/)

| Rule | Source | Level | Ready | Status | Fixability | Description |
|------|--------|-------|-------|--------|------------|-------------|
| `no-alert` | ESLint | DIR | ✗ | 🔶 | None | Disallow the use of `alert`, `confirm`, and `prompt` |
| `no-anonymous-default-export` | Unicorn | AST | ✓ | ✅ | Suggestion | Disallow anonymous default exports |
| `no-arguments` | ESLint | DIR | ✗ | 🔶 | Safe | Disallow use of the `arguments` object |
| `no-bitwise` | ESLint | AST | ✓ | ✅ | None | Disallow bitwise operators |
| `no-class` | Destack | AST | ✓ | ✅ | Unsafe | Disallow class declarations (prefer structs) |
| `no-console` | ESLint | DIR | ✗ | 🔶 | Safe | Disallow the use of `console` |
| `no-continue` | ESLint | AST | ✓ | ✅ | None | Disallow `continue` statements |
| `no-default-export` | Import | AST | ✓ | ✅ | Unsafe | Disallow default exports |
| `no-enum` | Biome | AST | ✓ | ✅ | Unsafe | Disallow TypeScript enums (prefer union types) |
| `no-explicit-any` | TS-ESLint | AST | ✓ | ✅ | Suggestion | Disallow the `any` type |
| `no-implicit-return` | Destack | AST | ✓ | ✅ | Safe | Require explicit `return` statements |
| `no-labels` | ESLint | AST | ✓ | ✅ | None | Disallow labeled statements |
| `no-magic-numbers` | ESLint | AST | ✓ | ✅ | Suggestion | Disallow magic numbers |
| `no-namespace` | TS-ESLint | AST | ✓ | ✅ | None | Disallow TypeScript namespaces |
| `no-non-null-assertion` | TS-ESLint | AST | ✓ | ✅ | Suggestion | Disallow non-null assertions using the `!` postfix |
| `no-null` | Unicorn | AST | ✓ | ✅ | Safe | Disallow `null` (prefer `undefined`) |
| `no-placeholder-implementation` | ESLint | AST | ✓ | ✅ | None | Disallow placeholder implementations (throw "not implemented", etc.) |
| `no-plusplus` | ESLint | AST | ✓ | ✅ | Safe | Disallow `++` and `--` operators |
| `no-process-exit` | Unicorn | DIR | ✗ | 🔶 | None | Disallow `process.exit()` |
| `no-require-imports` | TS-ESLint | DIR | ✗ | 🟡 | Safe | Disallow `require()` imports |
| `no-sequences` | ESLint | AST | ✓ | ✅ | None | Disallow comma operators |
| `no-shadow` | Destack | DIR | ✓ | 🔶 | Suggestion | Disallow shadowing by rebinding a value |
| `no-struct` | Destack | AST | ✓ | ✅ | Unsafe | Disallow struct declarations (prefer classes) |
| `no-ternary` | ESLint | AST | ✓ | ✅ | Unsafe | Disallow ternary operators |
| `no-void` | ESLint | AST | ✗ | 🔶 | Safe | Disallow the `void` operator |
| `no-warning-comments` | ESLint | AST | ✓ | ✅ | None | Disallow specified warning terms in comments (TODO, FIXME, etc.) |
| `no-wildcard-imports` | Destack | AST | ✓ | ✅ | Unsafe | Disallow wildcard imports |
| `strict-boolean-expressions` | TS-ESLint | DIR | ✓ | 🔶 | Unsafe | Disallow truthy/falsy coercion in conditions |

---

## Implementation Notes

### Compiler vs Linter Boundary

Some checks are handled by the **compiler** rather than the linter:

| Check | Owner | Reasoning |
|-------|-------|-----------|
| Type mismatches | Compiler | Fundamental type system |
| Unbound symbols | Compiler | Required for compilation |
| Conflicting symbols | Compiler | Required for compilation |
| Unreachable code | Compiler | CFG analysis for codegen |
| Precision loss | Compiler | Numeric type semantics |
| Pattern exhaustiveness | Compiler | Required for correctness |
| Ownership violations | Compiler | Memory safety |

The linter focuses on:
- **Style** - Preferences among valid alternatives
- **Suspicious patterns** - Valid but probably wrong
- **Performance hints** - Non-blocking suggestions
- **Complexity metrics** - Configurable thresholds
- **Project restrictions** - Team/project conventions
