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

| Rule | Source | Level | Ready | Status | Description |
|------|--------|-------|-------|--------|-------------|
| `await-holding-lock` | Clippy | DIR | ✗ | 🔶 | Disallow holding a mutex lock across an await point |
| `await-thenable` | TS-ESLint | DIR | ✗ | 🔶 | Disallow awaiting a value that is not a Promise |
| `for-direction` | ESLint | AST | ✓ | ✅ | Enforce for loop update clause moving in the correct direction |
| `no-approx-constant` | Destack | AST | ✓ | ✅ | Disallow approximate representations of mathematical constants |
| `no-array-delete` | TS-ESLint | DIR | ✓ | 🔶 | Disallow `delete` on arrays (creates holes) |
| `no-async-promise-executor` | ESLint | DIR | ✗ | 🔶 | Disallow async functions as Promise executor |
| `no-base-to-string` | TS-ESLint | DIR | ✓ | 🔶 | Disallow `.toString()` on objects without useful representation |
| `no-class-assign` | ESLint | DIR | ✓ | 🔶 | Disallow reassigning class/struct declarations |
| `no-compare-neg-zero` | ESLint | AST | ✓ | ✅ | Disallow comparing against negative zero |
| `no-const-assign` | ESLint | DIR | ✓ | 🔶 | Disallow reassigning const variables |
| `no-constant-binary-expression` | ESLint | AST | ✓ | ✅ | Disallow expressions where the operation doesn't affect the value |
| `no-constant-condition` | ESLint | AST | ✓ | ✅ | Disallow constant expressions in conditions |
| `no-control-regex` | ESLint | AST | ✓ | ✅ | Disallow control characters in regular expressions |
| `no-deprecated` | TS-ESLint | DIR | ✗ | 🔶 | Disallow use of `@deprecated` APIs |
| `no-duplicate-case` | ESLint | AST | ✓ | ✅ | Disallow duplicate case labels |
| `no-empty-range` | Destack | AST | ✓ | ✅ | Disallow empty ranges where start > end |
| `no-fallthrough` | ESLint | AST | ✓ | ✅ | Disallow fallthrough of case statements |
| `no-floating-promises` | TS-ESLint | DIR | ✗ | 🔶 | Require Promises to be awaited or returned |
| `no-for-in-array` | TS-ESLint | DIR | ✓ | 🔶 | Disallow iterating over arrays with for-in |
| `no-func-assign` | ESLint | DIR | ✓ | 🔶 | Disallow reassigning function declarations |
| `no-infinite-iterator` | Destack | DIR | ✗ | 🔶 | Disallow using methods that produce infinite iterators |
| `no-invalid-regexp` | ESLint | AST | ✓ | ✅ | Disallow invalid regular expression strings |
| `no-misused-promises` | TS-ESLint | DIR | ✗ | 🔶 | Disallow Promises in places not designed to handle them |
| `no-misused-spread` | TS-ESLint | DIR | ✓ | 🔶 | Disallow spread syntax in contexts where it's incorrect |
| `no-new-native-nonconstructor` | ESLint | DIR | ✗ | 🔶 | Disallow `new` on Symbol and BigInt |
| `no-obj-calls` | ESLint | DIR | ✗ | 🔶 | Disallow calling global objects as functions |
| `no-overlapping-match-arms` | Destack | DIR | ✓ | 🔶 | Disallow match patterns that subsume later arms |
| `no-promise-executor-return` | ESLint | DIR | ✗ | 🔶 | Disallow returning values from Promise executor |
| `no-self-compare` | ESLint | AST | ✓ | ✅ | Disallow comparisons where both sides are exactly the same |
| `no-sparse-arrays` | ESLint | AST | ✓ | ✅ | Disallow sparse arrays with holes |
| `no-struct-identity-compare` | Destack | DIR | ✓ | 🔶 | Disallow identity comparison on value types |
| `no-this-before-super` | ESLint | DIR | ✓ | 🔶 | Disallow `this` before calling `super()` in constructors |
| `no-unhandled-result` | Destack | DIR | ✓ | 🔶 | Require Result values to be handled |
| `no-unsafe-finally` | ESLint | AST | ✓ | ✅ | Disallow control flow statements in finally blocks |
| `no-unsafe-negation` | ESLint | AST | ✓ | ✅ | Disallow negating the left operand of relational operators |
| `no-unsafe-optional-chaining` | ESLint | DIR | ✓ | 🔶 | Disallow optional chaining in contexts where undefined is not allowed |
| `no-useless-assignment` | ESLint | DIR | ✓ | 🔶 | Disallow assignments that are immediately overwritten |
| `require-array-sort-compare` | TS-ESLint | DIR | ✓ | 🔶 | Require comparison function for `.sort()` |
| `switch-exhaustiveness-check` | TS-ESLint | DIR | ✓ | 🔶 | Require switch statements to be exhaustive |
| `unbound-method` | TS-ESLint | DIR | ✓ | 🔶 | Disallow unbound methods as callbacks |
| `unused-must-use` | Destack | DIR | ✓ | 🔶 | Disallow ignoring return values of `@mustUse` functions |
| `use-isnan` | ESLint | AST | ✓ | ✅ | Require `Number.isNaN()` instead of comparisons with `NaN` |

## Suspicious (U)

Code that is likely unintentional but may occasionally be intentional.

[`src/rules/suspicious/`](src/rules/suspicious/)

| Rule | Source | Level | Ready | Status | Description |
|------|--------|-------|-------|--------|-------------|
| `no-cond-assign` | ESLint | AST | ✓ | ✅ | Disallow assignment operators in conditional expressions |
| `no-confusing-assignment` | Destack | AST | ✓ | ✅ | Warn on assignments that look like comparisons |
| `no-confusing-non-null-assertion` | TS-ESLint | AST | ✓ | ✅ | Disallow non-null assertions after optional chain expressions |
| `no-constructor-return` | ESLint | AST | ✓ | ✅ | Disallow returning values from constructors |
| `no-debugger` | ESLint | AST | ✓ | ✅ | Disallow debugger statements |
| `no-dupe-else-if` | ESLint | AST | ✓ | ✅ | Disallow duplicate conditions in if-else-if chains |
| `no-duplicate-match-arms` | Destack | AST | ✓ | ✅ | Warn on match arms with identical bodies |
| `no-empty` | ESLint | AST | ✓ | ✅ | Disallow empty block statements |
| `no-empty-function` | ESLint | AST | ✓ | ✅ | Disallow empty functions |
| `no-empty-pattern` | ESLint | AST | ✓ | ✅ | Disallow empty destructuring patterns |
| `no-empty-static-block` | ESLint | AST | ✓ | ✅ | Disallow empty static initialization blocks in classes |
| `no-ex-assign` | ESLint | DIR | ✓ | 🔶 | Disallow reassigning exceptions in catch clauses |
| `no-extra-non-null-assertion` | TS-ESLint | AST | ✓ | ✅ | Disallow extra non-null assertions |
| `no-global-assign` | ESLint | DIR | ✗ | 🔶 | Disallow assignments to native objects or read-only globals |
| `no-implicit-void-expression` | Destack | AST | ✗ | 🔶 | Disallow implicit void returns from expression blocks |
| `no-incomplete-range` | Destack | AST | ✓ | 🟡 | Warn on exclusive ranges that are likely meant to be inclusive |
| `no-inner-declarations` | ESLint | AST | ✓ | ✅ | Disallow variable or function declarations in nested blocks |
| `no-loop-func` | ESLint | DIR | ✓ | 🔶 | Disallow functions that capture loop variables |
| `no-method-shadowing` | Destack | DIR | ✓ | 🔶 | Warn when a method shadows an inherited method |
| `no-misleading-character-class` | ESLint | AST | ✓ | ✅ | Disallow characters that behave unexpectedly in regex |
| `no-mixed-read-write` | Destack | DIR | ✓ | 🔶 | Disallow reading and mutating the same variable in one expression |
| `no-negation-in-equality-check` | Unicorn | AST | ✓ | ✅ | Disallow negation in the left operand of equality tests |
| `no-prototype-builtins` | ESLint | DIR | ✗ | 🔶 | Disallow calling Object.prototype methods directly on objects |
| `no-redundant-match-guard` | Destack | DIR | ✓ | 🔶 | Disallow match guards that are always true or false |
| `no-redundant-pattern` | Destack | AST | ✓ | ✅ | Disallow patterns that bind nothing useful |
| `no-return-assign` | ESLint | AST | ✓ | ✅ | Disallow assignment operators in return statements |
| `no-self-assign` | ESLint | AST | ✓ | ✅ | Disallow assignments where both sides are exactly the same |
| `no-shadow-restricted-names` | ESLint | DIR | ✓ | 🔶 | Disallow shadowing of restricted names |
| `no-single-element-tuple` | Destack | AST | ✓ | ✅ | Warn on single-element tuples that may be accidental |
| `no-template-curly-in-string` | ESLint | AST | ✓ | ✅ | Disallow template literal placeholder syntax in regular strings |
| `no-throw-literal` | ESLint | DIR | ✓ | 🔶 | Disallow throwing literals instead of Error objects |
| `no-unexpected-multiline` | ESLint | AST | ✗ | 🔶 | Disallow confusing multiline expressions |
| `no-unnecessary-clone` | TS-ESLint | DIR | ✓ | 🔶 | Warn on cloning values that are not used afterward |
| `no-unnecessary-type-assertion` | TS-ESLint | DIR | ✓ | 🔶 | Disallow type assertions that do not change the type |
| `no-unsafe-declaration-merging` | TS-ESLint | DIR | ✓ | 🔶 | Disallow unsafe declaration merging |
| `no-unused-except-recursion` | Destack | DIR | ✓ | 🔶 | Warn on function arguments only used for recursion |
| `no-useless-backreference` | ESLint | AST | ✓ | ✅ | Disallow useless backreferences in regular expressions |
| `no-useless-catch` | ESLint | AST | ✓ | ✅ | Disallow catch clauses that only rethrow |
| `no-useless-computed-key` | ESLint | AST | ✓ | ✅ | Disallow unnecessary computed property keys |
| `no-useless-concat` | ESLint | AST | ✓ | ✅ | Disallow unnecessary concatenation of literals |
| `no-useless-constructor` | ESLint | AST | ✓ | ✅ | Disallow unnecessary constructors |
| `no-useless-escape` | ESLint | AST | ✓ | ✅ | Disallow unnecessary escape characters |
| `no-useless-rename` | ESLint | AST | ✓ | ✅ | Disallow renaming imports/exports to the same name |
| `no-useless-return` | ESLint | AST | ✓ | ✅ | Disallow redundant return statements |
| `prefer-array-filter` | Unicorn | DIR | ✓ | 🔶 | Suggest `.filter()` over manual filtering loops |
| `prefer-array-find` | Unicorn | DIR | ✓ | 🔶 | Suggest `.find()` over manual search loops |
| `prefer-array-map` | Destack | DIR | ✓ | 🔶 | Suggest `.map()` over manual mapping loops |
| `prefer-flat-map` | Unicorn | DIR | ✓ | 🔶 | Suggest `.flatMap()` over `.map().flatten()` |
| `prefer-match` | Destack | AST | ✓ | ✅ | Suggest match expressions over complex if-else chains |
| `require-await` | TS-ESLint | AST | ✓ | ✅ | Disallow async functions with no await expressions |
| `require-else-in-if-chain` | Destack | AST | ✓ | ✅ | Require final else in if-else-if chains |
| `require-yield` | ESLint | AST | ✓ | ✅ | Require generator functions to contain yield |
| `return-await` | TS-ESLint | DIR | ✓ | 🔶 | Enforce consistent `return await` usage |
| `guard-for-in` | ESLint | AST | ✓ | ✅ | Require `hasOwnProperty` guard in for-in loops |

## Security (S)

Patterns that may expose the application to attacks.

[`src/rules/security/`](src/rules/security/)

| Rule | Source | Level | Ready | Status | Description |
|------|--------|-------|-------|--------|-------------|
| `no-blank-target` | Biome | AST | ✓ | ✅ | Disallow `target="_blank"` without `rel="noopener"` |
| `no-eval` | ESLint | DIR | ✗ | 🔶 | Disallow the use of `eval()` |
| `no-implied-eval` | ESLint | DIR | ✗ | 🔶 | Disallow `setTimeout` and `setInterval` with string arguments |
| `no-new-func` | ESLint | DIR | ✗ | 🔶 | Disallow `new Function()` |
| `no-script-url` | ESLint | AST | ✓ | ✅ | Disallow `javascript:` URLs |
| `no-secrets` | Biome | AST | ✓ | ✅ | Disallow hardcoded secrets and credentials |

## Performance (P)

Correct code that could be faster or use less memory.

[`src/rules/performance/`](src/rules/performance/)

| Rule | Source | Level | Ready | Status | Description |
|------|--------|-------|-------|--------|-------------|
| `no-accumulating-spread` | Biome | DIR | ✗ | 🔶 | Disallow spreading in accumulators (causes O(n²) allocations) |
| `no-array-for-each` | Unicorn | DIR | ✗ | 🔶 | Prefer for-of over `Array.forEach()` |
| `no-await-in-loop` | ESLint | AST | ✓ | ✅ | Disallow await inside of loops |
| `no-barrel-file` | Biome | AST | ✓ | ✅ | Disallow barrel files that re-export everything |
| `no-re-export-all` | Biome | AST | ✓ | ✅ | Disallow `export * from` (hurts tree-shaking) |
| `prefer-array-literal` | Destack | DIR | ✗ | 🔶 | Suggest using array literal instead of empty array followed by extend |
| `prefer-for-of` | TS-ESLint | DIR | ✗ | 🔶 | Prefer for-of loops over index-based for loops |
| `prefer-includes` | TS-ESLint | DIR | ✗ | 🔶 | Prefer `.includes()` over `.indexOf() !== -1` |
| `prefer-some-over-find` | Destack | DIR | ✗ | 🔶 | Prefer `.some()` over `.find() !== undefined` |
| `require-unicode-regexp` | ESLint | AST | ✓ | ✅ | Require `u` or `v` flag on regular expressions |

## Style (Y)

Subjective preferences for consistent coding style.

[`src/rules/style/`](src/rules/style/)

| Rule | Source | Level | Ready | Status | Description |
|------|--------|-------|-------|--------|-------------|
| `array-type` | TS-ESLint | DIR | ✓ | 🔶 | Require consistently using either `T[]` or `Array<T>` |
| `catch-error-name` | Unicorn | AST | ✓ | ✅ | Enforce a specific name for catch clause error parameters |
| `comment-casing` | Destack | AST | ✓ | ✅ | Enforce comment / doc casing |
| `comment-layout` | Destack | AST | ✓ | ✅ | Enforce comment / doc layout |
| `comment-punctuation` | Destack | AST | ✓ | ✅ | Enforce comment / doc punctuation style |
| `consistent-extension-style` | Destack | AST | ✓ | ✅ | Enforce consistent use of named or anonymous extensions |
| `consistent-type-definitions` | TS-ESLint | AST | ✓ | ✅ | Enforce type definitions to use either `interface` or `type` |
| `consistent-type-imports` | TS-ESLint | AST | ✓ | ✅ | Enforce consistent usage of type imports |
| `default-param-last` | ESLint | AST | ✓ | ✅ | Enforce default parameters to be last |
| `dot-notation` | ESLint | AST | ✓ | ✅ | Enforce dot notation whenever possible |
| `eqeqeq` | ESLint | AST | ✓ | ✅ | Require `===` and `!==` |
| `explicit-function-return-type` | TS-ESLint | AST | ✓ | ✅ | Require explicit return types on functions |
| `filename-case` | Unicorn | AST | ✓ | ✅ | Enforce a case style for filenames |
| `grouped-accessor-pairs` | ESLint | AST | ✓ | 🟡 | Require grouped accessor pairs in object literals and classes |
| `no-boolean-literal-compare` | Unicorn | AST | ✓ | ✅ | Disallow comparing boolean expressions to boolean literals |
| `no-class-for-data` | Destack | AST | ✓ | ✅ | Suggest struct for classes with only data fields |
| `no-collapsible-if` | Unicorn | AST | ✓ | ✅ | Suggest merging nested if statements without else |
| `no-constant-assertion` | Destack | AST | ✓ | 🟡 | Disallow assertions on constant values |
| `no-duplicate-type-constituents` | TS-ESLint | AST | ✓ | ✅ | Disallow duplicate constituents in union/intersection types |
| `no-else-return` | ESLint | AST | ✓ | ✅ | Disallow else blocks after return statements |
| `no-empty-interface` | TS-ESLint | AST | ✓ | ✅ | Disallow empty interfaces |
| `no-extra-boolean-cast` | ESLint | DIR | ✗ | 🟡 | Disallow unnecessary boolean casts |
| `no-implicit-coercion` | ESLint | DIR | ✗ | 🟡 | Disallow shorthand type conversions |
| `no-lonely-if` | ESLint | AST | ✓ | ✅ | Disallow if statements as the only statement in else blocks |
| `no-negated-condition` | ESLint | AST | ✓ | ✅ | Disallow negated conditions with else branches |
| `no-redundant-type-constituents` | TS-ESLint | DIR | ✓ | 🔶 | Disallow type constituents made redundant by others |
| `no-nested-ternary` | ESLint | AST | ✓ | ✅ | Disallow nested ternary expressions |
| `no-unneeded-ternary` | ESLint | AST | ✓ | ✅ | Disallow ternary operators when simpler alternatives exist |
| `no-unnecessary-template-expression` | TS-ESLint | DIR | ✓ | 🔶 | Disallow unnecessary template literal expressions |
| `no-unnecessary-type-arguments` | TS-ESLint | DIR | ✓ | 🔶 | Disallow type arguments that equal the default |
| `no-var` | ESLint | AST | ✓ | ✅ | Require `let` or `const` instead of `var` |
| `object-shorthand` | ESLint | AST | ✓ | ✅ | Require or disallow method and property shorthand syntax |
| `operator-assignment` | ESLint | AST | ✓ | ✅ | Require or disallow assignment operator shorthand |
| `prefer-arrow-callback` | ESLint | AST | ✓ | ✅ | Require arrow functions as callbacks |
| `prefer-as-const` | TS-ESLint | AST | ✓ | ✅ | Prefer `as const` over literal type assertions |
| `prefer-const` | ESLint | DIR | ✓ | 🔶 | Require `const` declarations for never-reassigned variables |
| `prefer-destructuring` | ESLint | DIR | ✓ | 🔶 | Prefer destructuring from arrays and objects |
| `prefer-exponentiation-operator` | ESLint | DIR | ✗ | 🔶 | Prefer `**` over `Math.pow()` |
| `prefer-expression` | Destack | AST | ✓ | ✅ | Prefer expression syntax for assignments |
| `prefer-extension-method` | Destack | DIR | ✗ | 🔶 | Suggest converting functions to extension methods |
| `prefer-fragment-shorthand` | React | AST | ✓ | 🟡 | Prefer `<>` shorthand over `<Fragment>` |
| `prefer-if-else-over-match-bool` | Destack | AST | ✓ | 🟡 | Suggest using if/else instead of match on booleans |
| `prefer-implicit-return` | Destack | AST | ✓ | ✅ | Prefer implicit returns in expression-bodied functions |
| `prefer-inclusive-range` | Destack | AST | ✓ | 🟡 | Prefer inclusive range syntax where applicable |
| `prefer-loop` | Destack | AST | ✓ | ✅ | Prefer `loop` keyword over `while(true)` or `for(;;)` |
| `prefer-map-or-else` | Destack | DIR | ✓ | 🔶 | Prefer `mapOrElse()` over `map().unwrap()` |
| `prefer-named-extension` | Destack | AST | ✓ | ✅ | Prefer named extensions for foreign types |
| `prefer-numeric-literals` | ESLint | DIR | ✗ | 🔶 | Prefer numeric literals over `parseInt()` |
| `prefer-object-has-own` | ESLint | DIR | ✗ | 🔶 | Prefer `Object.hasOwn()` over `Object.prototype.hasOwnProperty` |
| `prefer-object-spread` | ESLint | DIR | ✗ | 🔶 | Prefer spread operator over `Object.assign()` |
| `prefer-pattern-over-guard` | Destack | AST | ✓ | 🟡 | Suggest moving match guards into the pattern |
| `prefer-precise-numeric` | Destack | AST | ✓ | ✅ | Prefer precise numeric types over `number` |
| `prefer-promise-reject-errors` | TS-ESLint | DIR | ✗ | 🔶 | Require Error objects in Promise rejections |
| `prefer-propagate-operator` | Destack | DIR | ✗ | 🟡 | Prefer `?` operator over manual Result matching |
| `prefer-range-contains` | Destack | AST | ✓ | 🟡 | Prefer range contains method over comparison chains |
| `prefer-range-literal` | Destack | AST | ✓ | ✅ | Prefer range literals over C-style for loops |
| `prefer-result-type` | Destack | DIR | ✓ | 🔶 | Prefer `Result<T, E>` return type over throwing |
| `prefer-self-closing-tree` | React | AST | ✓ | 🟡 | Prefer self-closing tree elements when possible |
| `prefer-set-over-empty-map` | Destack | DIR | ✗ | 🔶 | Suggest `Set<K>` over `Map<K, void>` |
| `prefer-struct-literal` | Destack | AST | ✓ | 🟡 | Prefer struct literal syntax over constructor calls |
| `prefer-struct-over-class` | Destack | DIR | ✓ | 🔶 | Suggest struct over class when possible |
| `prefer-template` | ESLint | AST | ✓ | ✅ | Prefer template literals over string concatenation |
| `prefer-tuple` | Destack | DIR | ✓ | 🔶 | Suggest tuple type for fixed-length heterogeneous arrays |
| `prefer-tuple-destructure` | Destack | AST | ✓ | 🟡 | Prefer tuple destructuring over indexed access |
| `prefer-tuple-swap` | Destack | AST | ✓ | 🟡 | Prefer tuple swap syntax over temporary variable |
| `prefer-unary-negation` | Destack | AST | ✓ | ✅ | Prefer unary negation over multiplying by -1 |
| `promise-function-async` | TS-ESLint | DIR | ✗ | 🔶 | Require `async` keyword for Promise-returning functions |
| `require-jsdoc` | ESLint | AST | ✓ | ✅ | Require documentation on public items |
| `require-returns-doc` | ESLint | AST | ✓ | ✅ | Require return type documentation |
| `restrict-template-expressions` | TS-ESLint | DIR | ✓ | 🔶 | Require template expressions to be strings or numbers |
| `sort-imports` | ESLint | AST | ✓ | 🟡 | Enforce sorted import declarations |
| `symbol-description` | ESLint | DIR | ✗ | 🔶 | Require symbol descriptions |
| `tree-prop-spread-candidate` | Destack | AST | ✓ | 🟡 | Suggest spreading repeated tree props |
| `yoda` | ESLint | AST | ✓ | ✅ | Disallow Yoda conditions |

## Complexity (X)

Overly complex code that is harder to understand and maintain.

[`src/rules/complexity/`](src/rules/complexity/)

| Rule | Source | Level | Ready | Status | Description |
|------|--------|-------|-------|--------|-------------|
| `cognitive-complexity` | Biome | DIR | ✓ | 🟡 | Enforce a maximum cognitive complexity |
| `cyclomatic-complexity` | ESLint | DIR | ✓ | 🟡 | Enforce a maximum cyclomatic complexity |
| `max-depth` | ESLint | AST | ✓ | ✅ | Enforce a maximum depth of nested blocks |
| `max-lines` | ESLint | AST | ✓ | ✅ | Enforce a maximum number of lines per file |
| `max-lines-per-function` | ESLint | AST | ✓ | ✅ | Enforce a maximum number of lines per function |
| `max-nested-callbacks` | ESLint | AST | ✓ | ✅ | Enforce a maximum depth of nested callbacks |
| `max-params` | ESLint | AST | ✓ | ✅ | Enforce a maximum number of function parameters |
| `max-statements` | ESLint | AST | ✓ | ✅ | Enforce a maximum number of statements per function |
| `no-complex-boolean-expression` | Destack | AST | ✓ | ✅ | Suggest simplifying complex boolean expressions |
| `no-complex-type` | Destack | AST/DIR | ✓ | 🔶 | Warn on overly complex types that should be aliased |
| `no-excessive-booleans` | Destack | AST | ✓ | ✅ | Disallow too many boolean parameters or struct fields |
| `no-multi-assign` | ESLint | AST | ✓ | ✅ | Disallow chained assignment expressions |
| `no-multi-declarators` | ESLint | AST | ✓ | ✅ | Disallow multiple variable declarations per statement |
| `no-unused-expressions` | ESLint | AST | ✓ | ✅ | Disallow expressions that have no effect |
| `no-useless-underscore-binding` | Destack | AST | ✓ | ✅ | Warn on underscore bindings with no side effects |
| `prefer-expression-over-let-if` | Destack | AST | ✓ | ✅ | Suggest expression syntax over let-if sequences |
| `prefer-if-let` | Destack | AST | ✓ | ✅ | Suggest if-let over single-arm match |
| `prefer-simplified-comparison` | Destack | AST | ✓ | ✅ | Suggest simplifying comparisons like `x >= y + 1` |

## Restriction (R)

Opt-in rules that ban certain patterns by project choice. These rules may conflict with each other.

[`src/rules/restriction/`](src/rules/restriction/)

| Rule | Source | Level | Ready | Status | Description |
|------|--------|-------|-------|--------|-------------|
| `no-alert` | ESLint | DIR | ✗ | 🔶 | Disallow the use of `alert`, `confirm`, and `prompt` |
| `no-anonymous-default-export` | Unicorn | AST | ✓ | ✅ | Disallow anonymous default exports |
| `no-arguments` | ESLint | DIR | ✗ | 🔶 | Disallow use of the `arguments` object |
| `no-bitwise` | ESLint | AST | ✓ | ✅ | Disallow bitwise operators |
| `no-class` | Destack | AST | ✓ | ✅ | Disallow class declarations (prefer structs) |
| `no-console` | ESLint | DIR | ✗ | 🔶 | Disallow the use of `console` |
| `no-continue` | ESLint | AST | ✓ | ✅ | Disallow `continue` statements |
| `no-default-export` | Import | AST | ✓ | ✅ | Disallow default exports |
| `no-enum` | Biome | AST | ✓ | ✅ | Disallow TypeScript enums (prefer union types) |
| `no-explicit-any` | TS-ESLint | AST | ✓ | ✅ | Disallow the `any` type |
| `no-implicit-return` | Destack | AST | ✓ | ✅ | Require explicit `return` statements |
| `no-labels` | ESLint | AST | ✓ | ✅ | Disallow labeled statements |
| `no-magic-numbers` | ESLint | AST | ✓ | ✅ | Disallow magic numbers |
| `no-namespace` | TS-ESLint | AST | ✓ | ✅ | Disallow TypeScript namespaces |
| `no-non-null-assertion` | TS-ESLint | AST | ✓ | ✅ | Disallow non-null assertions using the `!` postfix |
| `no-null` | Unicorn | AST | ✓ | ✅ | Disallow `null` (prefer `undefined`) |
| `no-placeholder-implementation` | ESLint | AST | ✓ | ✅ | Disallow placeholder implementations (throw "not implemented", etc.) |
| `no-plusplus` | ESLint | AST | ✓ | ✅ | Disallow `++` and `--` operators |
| `no-process-exit` | Unicorn | DIR | ✗ | 🔶 | Disallow `process.exit()` |
| `no-require-imports` | TS-ESLint | DIR | ✗ | 🟡 | Disallow `require()` imports |
| `no-sequences` | ESLint | AST | ✓ | ✅ | Disallow comma operators |
| `no-shadow` | Destack | DIR | ✓ | 🔶 | Disallow shadowing by rebinding a value |
| `no-struct` | Destack | AST | ✓ | ✅ | Disallow struct declarations (prefer classes) |
| `no-ternary` | ESLint | AST | ✓ | ✅ | Disallow ternary operators |
| `no-void` | ESLint | AST | ✗ | 🔶 | Disallow the `void` operator |
| `no-warning-comments` | ESLint | AST | ✓ | ✅ | Disallow specified warning terms in comments (TODO, FIXME, etc.) |
| `no-wildcard-imports` | Destack | AST | ✓ | ✅ | Disallow wildcard imports |
| `strict-boolean-expressions` | TS-ESLint | DIR | ✓ | 🔶 | Disallow truthy/falsy coercion in conditions |

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
