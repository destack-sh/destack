# Destack Linter

Static analysis rules for Destack (`.ds`, `.ts`, `.tsx`, `.js`, `.jsx`) files.

## Architecture

The linter operates at three IR levels, each enabling different kinds of analysis:

| Level | IR | What's Available | Example Rules |
|-------|-----|------------------|---------------|
| **AST** | Syntax tree | Syntax patterns, no type info | `no-debugger`, `for-direction` |
| **DIR** | Typed IR | Symbols, types, cross-references | `no-floating-promises`, `prefer-const` |
| **MIR** | Machine IR | CFG, ownership, data flow | `no-unreachable`, `cyclomatic-complexity` |

## Categories

Rules are organized into categories, each with a letter code for diagnostic IDs:

| Category | Code | Default | Description |
|----------|------|---------|-------------|
| [Correctness](#correctness-c) | `C` | Error | Likely bugs and logic errors |
| [Suspicious](#suspicious-u) | `U` | Warning | Code that is likely unintentional |
| [Security](#security-s) | `S` | Error | Potential vulnerabilities |
| [Performance](#performance-p) | `P` | Warning | Inefficient patterns |
| [Style](#style-y) | `Y` | Warning | Consistent coding style |
| [Complexity](#complexity-x) | `X` | Warning | Overly complex code |
| [Restriction](#restriction-r) | `R` | Off | Project-specific restrictions (opt-in) |
| [Pedantic](#pedantic-d) | `D` | Off | Very strict checks (opt-in) |

**Recommended preset**: Correctness + Suspicious + Security

---

## Correctness (C)

High-confidence issues that are almost always wrong.

[`src/rules/correctness/`](src/rules/correctness/)

| Rule | Source | Level | Ready | Status | Notes |
|------|--------|-------|-------|--------|-------|
| `await-thenable` | TS-ESLint | DIR | ✗ | 🔶 | `await` non-Promise |
| `constructor-super` | ESLint | DIR | ✗ | 🟡 | Verify `super()` in constructor |
| `for-direction` | ESLint | AST | ✓ | ✅ | For loop going wrong direction |
| `getter-return` | ESLint | DIR | ✗ | 🟡 | Getter must return a value |
| `no-array-delete` | TS-ESLint | DIR | ✗ | 🔶 | `delete arr[i]` |
| `no-async-promise-executor` | ESLint | AST | ✗ | 🟡 | Needs `Promise` builtin check |
| `no-class-assign` | ESLint | DIR | ✗ | 🟡 | Reassigning class declaration |
| `no-compare-neg-zero` | ESLint | AST | ✓ | ✅ | Compare to `-0` |
| `no-const-assign` | ESLint | DIR | ✗ | 🟡 | Needs binding kind tracking |
| `no-constant-binary-expression` | ESLint | AST | ✓ | ✅ | Comparison always same result |
| `no-constant-condition` | ESLint | AST | ✓ | ✅ | Constant in condition (`if (true)`) |
| `no-duplicate-case` | ESLint | AST | ✓ | ✅ | Duplicate switch cases |
| `no-fallthrough` | ESLint | AST | ✓ | ✅ | Switch case fallthrough |
| `no-floating-promises` | TS-ESLint | DIR | ✗ | 🔶 | Unhandled Promise |
| `no-for-in-array` | TS-ESLint | DIR | ✗ | 🔶 | `for-in` on array |
| `no-func-assign` | ESLint | DIR | ✗ | 🟡 | Reassigning function declaration |
| `no-import-assign` | ESLint | DIR | ✗ | 🟡 | Reassigning import binding |
| `no-misused-promises` | TS-ESLint | DIR | ✗ | 🔶 | Promise in wrong context |
| `no-misused-spread` | TS-ESLint | DIR | ✗ | 🔶 | Spread in wrong context |
| `no-newtype-structural-match` | Destack | DIR | ✗ | 🟡 | Match newtype without constructor |
| `no-obj-calls` | ESLint | DIR | ✗ | 🟡 | Needs `Math`/`JSON` builtin check |
| `no-promise-executor-return` | ESLint | AST | ✗ | 🟡 | Needs `Promise` builtin check |
| `no-self-compare` | ESLint | AST | ✓ | ✅ | Comparing value to itself |
| `no-struct-identity-compare` | Destack | DIR | ✗ | 🟡 | `===` on structs (no identity) |
| `no-this-before-super` | ESLint | DIR | ✗ | 🟡 | `this` before `super()` call |
| `no-unsafe-finally` | ESLint | AST | ✓ | ✅ | Control flow in `finally` |
| `no-unsafe-negation` | ESLint | AST | ✓ | ✅ | `!a in b` vs `!(a in b)` |
| `no-unsafe-optional-chaining` | ESLint | DIR | ✗ | 🟡 | `?.` in unsafe contexts |
| `switch-exhaustiveness-check` | TS-ESLint | DIR | ✗ | 🟡 | Non-exhaustive switch |
| `unbound-method` | TS-ESLint | DIR | ✗ | 🔶 | Method used without binding |

## Suspicious (U)

Code that is likely unintentional but may occasionally be intentional.

[`src/rules/suspicious/`](src/rules/suspicious/)

| Rule | Source | Level | Ready | Status | Notes |
|------|--------|-------|-------|--------|-------|
| `no-cond-assign` | ESLint | AST | ✓ | ✅ | Assignment in condition (allows `let`) |
| `no-confusing-non-null-assertion` | TS-ESLint | AST | ✓ | ✅ | `!` near `?` |
| `no-confusing-void-expression` | TS-ESLint | DIR | ✗ | 🔶 | Void in expression position |
| `no-constructor-return` | ESLint | AST | ✓ | ✅ | Return in constructor |
| `no-debugger` | ESLint | AST | ✓ | ✅ | Debugger statements |
| `no-dupe-else-if` | ESLint | AST | ✓ | ✅ | Duplicate else-if |
| `no-empty` | ESLint | AST | ✓ | ✅ | Empty `{}` blocks |
| `no-empty-pattern` | ESLint | AST | ✓ | ✅ | Empty destructuring `{}` |
| `no-ex-assign` | ESLint | DIR | ✗ | 🟡 | Needs exception binding tracking |
| `no-misleading-character-class` | ESLint | AST | ✓ | ✅ | Misleading regex chars |
| `no-negation-in-equality-check` | Unicorn | AST | ✓ | ✅ | `!a == b` confusion |
| `no-prototype-builtins` | ESLint | DIR | ✗ | 🟡 | Needs builtin method check |
| `no-redundant-pattern` | Destack | AST | ✓ | ✅ | Pattern binds nothing useful |
| `no-self-assign` | ESLint | AST | ✓ | ✅ | `x = x` |
| `no-template-curly-in-string` | ESLint | AST | ✓ | ✅ | `"${x}"` in regular string |
| `no-thenable` | Unicorn | DIR | ✗ | 🔶 | Object with `.then()` |
| `no-unnecessary-type-assertion` | TS-ESLint | DIR | ✗ | 🔶 | Redundant `as T` |
| `no-useless-backreference` | ESLint | AST | ✓ | ✅ | Invalid regex backrefs |
| `no-useless-catch` | ESLint | AST | ✓ | ✅ | Catch that just rethrows |
| `no-useless-computed-key` | ESLint | AST | ✓ | ✅ | `{["x"]: 1}` |
| `no-useless-concat` | ESLint | AST | ✓ | ✅ | `"a" + "b"` |
| `no-useless-constructor` | ESLint | AST | ✓ | ✅ | Empty constructor |
| `no-useless-escape` | ESLint | AST | ✓ | ✅ | Unnecessary escape chars |
| `no-useless-rename` | ESLint | AST | ✓ | ✅ | `{x: x}` in destructuring |
| `no-useless-return` | ESLint | AST | ✓ | ✅ | Return with no value at end |
| `prefer-match` | Destack | AST | ✓ | ✅ | Complex if-else → match |
| `require-yield` | ESLint | AST | ✓ | ✅ | Generator without yield |

## Security (S)

Patterns that may expose the application to attacks.

[`src/rules/security/`](src/rules/security/)

| Rule | Source | Level | Ready | Status | Notes |
|------|--------|-------|-------|--------|-------|
| `no-eval` | ESLint | DIR | ✗ | 🟡 | Needs `eval` builtin check |
| `no-implied-eval` | ESLint | DIR | ✗ | 🟡 | Needs `setTimeout` builtin check |
| `no-new-func` | ESLint | DIR | ✗ | 🟡 | Needs `Function` builtin check |

## Performance (P)

Correct code that could be faster or use less memory.

[`src/rules/performance/`](src/rules/performance/)

| Rule | Source | Level | Ready | Status | Notes |
|------|--------|-------|-------|--------|-------|
| `no-array-for-each` | Unicorn | DIR | ✗ | 🟡 | Needs array type check |
| `no-await-in-loop` | ESLint | AST | ✓ | ✅ | Sequential awaits in loop |
| `prefer-for-of` | TS-ESLint | DIR | ✗ | 🔶 | Index loop → for-of |
| `prefer-includes` | TS-ESLint | DIR | ✗ | 🔶 | `.indexOf() !== -1` |
| `prefer-spread` | ESLint | DIR | ✗ | 🟡 | Needs `Function.prototype` check |
| `require-array-sort-compare` | TS-ESLint | DIR | ✗ | 🔶 | `.sort()` needs comparator |

## Style (Y)

Subjective preferences for consistent coding style.

[`src/rules/style/`](src/rules/style/)

| Rule | Source | Level | Ready | Status | Notes |
|------|--------|-------|-------|--------|-------|
| `array-type` | TS-ESLint | AST | ✗ | 🟡 | `T[]` vs `Array<T>` |
| `catch-error-name` | Unicorn | AST | ✓ | ✅ | Consistent error name |
| `consistent-extension-style` | Destack | AST | ✓ | ✅ | Named vs anonymous extensions |
| `consistent-type-definitions` | TS-ESLint | AST | ✓ | ✅ | `type` vs `interface` |
| `consistent-type-imports` | TS-ESLint | AST | ✓ | ✅ | `import type` |
| `dot-notation` | ESLint | AST | ✓ | ✅ | `obj["x"]` → `obj.x` |
| `eqeqeq` | ESLint | AST | ✓ | ✅ | `==` → `===` (Destack `==` is typed / overloadable!) |
| `filename-case` | Unicorn | AST | ✓ | ✅ | File naming convention |
| `no-else-return` | ESLint | AST | ✓ | ✅ | Early return style |
| `no-lonely-if` | ESLint | AST | ✓ | ✅ | Lonely `if` in `else` |
| `no-nested-ternary` | ESLint | AST | ✓ | ✅ | Nested `?:` |
| `no-unneeded-ternary` | ESLint | AST | ✓ | ✅ | `x ? true : false` |
| `no-var` | ESLint | AST | ✓ | ✅ | `var` → `let`/`const` |
| `object-shorthand` | ESLint | AST | ✓ | ✅ | `{x: x}` → `{x}` |
| `operator-assignment` | ESLint | AST | ✓ | ✅ | `x = x + 1` → `x += 1` |
| `prefer-arrow-callback` | ESLint | AST | ✓ | ✅ | Function → arrow |
| `prefer-as-const` | TS-ESLint | AST | ✓ | ✅ | `as const` |
| `prefer-const` | ESLint | DIR | ✗ | 🟡 | Needs reassignment tracking |
| `prefer-destructuring` | ESLint | DIR | ✗ | 🟡 | Needs symbol tracking |
| `prefer-exponentiation-operator` | ESLint | DIR | ✗ | 🟡 | Needs `Math.pow` builtin check |
| `prefer-expression` | Destack | AST | ✓ | ✅ | `let x; if (...) x=a` → `const x = if (...) { a }` |
| `prefer-implicit-return` | Destack | AST | ✓ | ✅ | `return x` → `x` |
| `prefer-loop` | Destack | AST | ✓ | 🟡 | Prefer explicit `loop` over `while(true)` or `for (;;)` |
| `prefer-newtype-over-alias` | Destack | DIR | ✗ | 🟡 | Type alias → newtype |
| `prefer-object-spread` | ESLint | DIR | ✗ | 🟡 | Needs `Object.assign` builtin check |
| `prefer-range-literal` | Destack | AST | ✓ | ✅ | `for (let i=0; i<n; i++)` → `for i of 0..n` |
| `prefer-struct-over-class` | Destack | DIR | ✗ | 🟡 | Needs usage analysis |
| `prefer-template` | ESLint | AST | ✓ | ✅ | Concat → template literal |
| `prefer-tuple` | Destack | DIR | ✗ | 🟡 | Arrays used as tuples → `(a, b)` |

## Complexity (X)

Overly complex code that is harder to understand and maintain.

[`src/rules/complexity/`](src/rules/complexity/)

| Rule | Source | Level | Ready | Status | Notes |
|------|--------|-------|-------|--------|-------|
| `max-depth` | ESLint | AST | ✓ | ✅ | Maximum nesting depth |
| `max-lines` | ESLint | AST | ✓ | ✅ | Maximum lines per file |
| `max-lines-per-function` | ESLint | AST | ✓ | ✅ | Maximum lines per function |
| `max-nested-callbacks` | ESLint | AST | ✓ | ✅ | Maximum callback nesting |
| `max-params` | ESLint | AST | ✓ | ✅ | Maximum function parameters |
| `max-statements` | ESLint | AST | ✓ | ✅ | Maximum statements per function |
| `no-multi-assign` | ESLint | AST | ✓ | ✅ | `a = b = c` chains |
| `no-multi-declarators` | ESLint | AST | ✓ | ✅ | `let a = 1, b = 2` multi-declarators |

## Restriction (R)

Opt-in rules that ban certain patterns by project choice.

[`src/rules/restriction/`](src/rules/restriction/)

| Rule | Source | Level | Ready | Status | Notes |
|------|--------|-------|-------|--------|-------|
| `no-alert` | ESLint | DIR | ✗ | 🟡 | Needs `alert` builtin check |
| `no-anonymous-default-export` | Unicorn | AST | ✓ | ✅ | Named exports only |
| `no-arguments` | ESLint | DIR | ✗ | 🟡 | Needs `arguments` binding in resolve |
| `no-bitwise` | ESLint | AST | ✓ | ✅ | Bitwise operators |
| `no-class` | Destack | AST | ✓ | ✅ | Prefer struct over class |
| `no-console` | ESLint | DIR | ✗ | 🟡 | Needs `console` builtin check |
| `no-continue` | ESLint | AST | ✓ | ✅ | `continue` statement |
| `no-explicit-any` | TS-ESLint | AST | ✓ | ✅ | Ban explicit `any` type |
| `no-implicit-return` | Destack | AST | ✓ | 🟡 | Require explicit `return` |
| `no-labels` | ESLint | AST | ✓ | ✅ | Labeled statements |
| `no-magic-numbers` | ESLint | AST | ✓ | ✅ | Unnamed numeric literals |
| `no-namespace` | TS-ESLint | AST | ✓ | ✅ | `namespace` keyword |
| `no-non-null-assertion` | TS-ESLint | AST | ✓ | ✅ | `!` assertion operator |
| `no-plusplus` | ESLint | AST | ✓ | ✅ | `++` and `--` |
| `no-process-exit` | Unicorn | DIR | ✗ | 🟡 | Needs `process` builtin check |
| `no-require-imports` | TS-ESLint | AST | ✓ | ✅ | CommonJS `require()` |
| `no-restricted-imports` | ESLint | AST | ✓ | 🟡 | Banned imports |
| `no-sequences` | ESLint | AST | ✓ | ✅ | Comma operator (JS/TS only) |
| `no-ternary` | ESLint | AST | ✓ | ✅ | Ternary operator |
| `no-warning-comments` | ESLint | AST | ✗ | 🟡 | Needs comment API |
| `strict-boolean-expressions` | TS-ESLint | DIR | ✗ | 🔶 | No truthy/falsy |

## Pedantic (D)

Very strict or opinionated checks that may be too noisy.

[`src/rules/pedantic/`](src/rules/pedantic/)

| Rule | Source | Level | Ready | Status | Notes |
|------|--------|-------|-------|--------|-------|
| `consistent-return` | ESLint | DIR | ✗ | 🟡 | Needs flow analysis |
| `explicit-function-return-type` | TS-ESLint | AST | ✓ | ✅ | Explicit return types |
| `explicit-length-check` | Unicorn | DIR | ✗ | 🟡 | Needs type info |
| `guard-for-in` | ESLint | AST | ✗ | 🟡 | `hasOwnProperty` in for-in |
| `no-caller` | ESLint | AST | ✗ | 🟡 | `arguments.caller` |
| `no-extend-native` | ESLint | DIR | ✗ | 🟡 | Needs prototype tracking |
| `no-inferrable-types` | TS-ESLint | DIR | ✗ | 🟡 | Needs type inference |
| `no-iterator` | ESLint | AST | ✓ | 🟡 | `__iterator__` property |
| `no-param-reassign` | ESLint | DIR | ✗ | 🟡 | Needs symbol tracking |
| `no-shadow` | ESLint | DIR | ✗ | 🟡 | Needs scope analysis |
| `no-unreadable-array-destructuring` | Unicorn | AST | ✓ | 🟡 | Too many holes `[,,,x]` |
| `no-use-before-define` | ESLint | DIR | ✗ | 🟡 | Needs declaration order |
| `prefer-nullish-coalescing` | TS-ESLint | DIR | ✗ | 🔶 | `??` vs `\|\|` |
| `radix` | ESLint | DIR | ✗ | 🟡 | Needs `parseInt` builtin check |
