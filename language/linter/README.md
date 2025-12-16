# Destack Linter

Static analysis rules for Destack (`.ds`, `.ts`, `.tsx`, `.js`, `.jsx`) files.

## Architecture

The linter operates at three IR levels, each enabling different kinds of analysis:

| Level | IR | What's Available | Example Rules |
|-------|-----|------------------|---------------|
| **AST** | Syntax tree | Syntax patterns, no type info | `no-debugger`, `no-eval` |
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

| Rule | Source | Level | Status | File | Notes |
|------|--------|-------|--------|------|-------|
| `await-thenable` | TS-ESLint | DIR | 🔶 | [`await_thenable.rs`](src/rules/correctness/await_thenable.rs) | `await` non-Promise |
| `constructor-super` | ESLint | DIR | 🟡 | [`constructor_super.rs`](src/rules/correctness/constructor_super.rs) | Verify `super()` in constructor |
| `for-direction` | ESLint | AST | 🟡 | [`for_direction.rs`](src/rules/correctness/for_direction.rs) | For loop going wrong direction |
| `getter-return` | ESLint | DIR | 🟡 | [`getter_return.rs`](src/rules/correctness/getter_return.rs) | Getter must return a value |
| `no-array-delete` | TS-ESLint | DIR | 🔶 | [`no_array_delete.rs`](src/rules/correctness/no_array_delete.rs) | `delete arr[i]` |
| `no-async-promise-executor` | ESLint | AST | 🟡 | [`no_async_promise_executor.rs`](src/rules/correctness/no_async_promise_executor.rs) | `new Promise(async ...)` |
| `no-class-assign` | ESLint | DIR | 🟡 | [`no_class_assign.rs`](src/rules/correctness/no_class_assign.rs) | Reassigning class declaration |
| `no-compare-neg-zero` | ESLint | AST | 🟡 | [`no_compare_neg_zero.rs`](src/rules/correctness/no_compare_neg_zero.rs) | Compare to `-0` |
| `no-const-assign` | ESLint | AST | 🟡 | [`no_const_assign.rs`](src/rules/correctness/no_const_assign.rs) | Reassigning `const` variable |
| `no-constant-binary-expression` | ESLint | AST | 🟡 | [`no_constant_binary_expression.rs`](src/rules/correctness/no_constant_binary_expression.rs) | Comparison always same result |
| `no-constant-condition` | ESLint | AST | ✅ | [`no_constant_condition.rs`](src/rules/correctness/no_constant_condition.rs) | Constant in condition (`if (true)`) |
| `no-control-regex` | ESLint | AST | 🟡 | [`no_control_regex.rs`](src/rules/correctness/no_control_regex.rs) | Control chars in regex |
| `no-dupe-args` | ESLint | AST | 🟡 | [`no_dupe_args.rs`](src/rules/correctness/no_dupe_args.rs) | Duplicate function parameters |
| `no-dupe-keys` | ESLint | AST | 🟡 | [`no_dupe_keys.rs`](src/rules/correctness/no_dupe_keys.rs) | Duplicate object/struct keys |
| `no-duplicate-case` | ESLint | AST | 🟡 | [`no_duplicate_case.rs`](src/rules/correctness/no_duplicate_case.rs) | Duplicate switch cases |
| `no-fallthrough` | ESLint | AST | 🟡 | [`no_fallthrough.rs`](src/rules/correctness/no_fallthrough.rs) | Switch case fallthrough |
| `no-floating-promises` | TS-ESLint | DIR | 🔶 | [`no_floating_promises.rs`](src/rules/correctness/no_floating_promises.rs) | Unhandled Promise |
| `no-for-in-array` | TS-ESLint | DIR | 🔶 | [`no_for_in_array.rs`](src/rules/correctness/no_for_in_array.rs) | `for-in` on array |
| `no-func-assign` | ESLint | DIR | 🟡 | [`no_func_assign.rs`](src/rules/correctness/no_func_assign.rs) | Reassigning function declaration |
| `no-import-assign` | ESLint | DIR | 🟡 | [`no_import_assign.rs`](src/rules/correctness/no_import_assign.rs) | Reassigning import binding |
| `no-invalid-regexp` | ESLint | AST | 🟡 | [`no_invalid_regexp.rs`](src/rules/correctness/no_invalid_regexp.rs) | Invalid regex syntax |
| `no-loss-of-precision` | ESLint | AST | 🟡 | [`no_loss_of_precision.rs`](src/rules/correctness/no_loss_of_precision.rs) | Numeric precision loss |
| `no-misused-promises` | TS-ESLint | DIR | 🔶 | [`no_misused_promises.rs`](src/rules/correctness/no_misused_promises.rs) | Promise in wrong context |
| `no-misused-spread` | TS-ESLint | DIR | 🔶 | [`no_misused_spread.rs`](src/rules/correctness/no_misused_spread.rs) | Spread in wrong context |
| `no-mixed-numeric-ops` | Destack | DIR | 🔶 | [`no_mixed_numeric_ops.rs`](src/rules/correctness/no_mixed_numeric_ops.rs) | `int32 + uint64` without cast |
| `no-newtype-structural-match` | Destack | DIR | 🟡 | [`no_newtype_structural_match.rs`](src/rules/correctness/no_newtype_structural_match.rs) | Match newtype without constructor |
| `no-obj-calls` | ESLint | DIR | 🟡 | [`no_obj_calls.rs`](src/rules/correctness/no_obj_calls.rs) | Calling `Math()`, `JSON()` |
| `no-promise-executor-return` | ESLint | AST | 🟡 | [`no_promise_executor_return.rs`](src/rules/correctness/no_promise_executor_return.rs) | Return in Promise executor |
| `no-self-compare` | ESLint | AST | ✅ | [`no_self_compare.rs`](src/rules/correctness/no_self_compare.rs) | Comparing value to itself |
| `no-setter-return` | ESLint | AST | 🟡 | [`no_setter_return.rs`](src/rules/correctness/no_setter_return.rs) | Setter must not return value |
| `no-sparse-arrays` | ESLint | AST | 🟡 | [`no_sparse_arrays.rs`](src/rules/correctness/no_sparse_arrays.rs) | Holes in arrays `[1,,3]` |
| `no-struct-identity-compare` | Destack | DIR | 🟡 | [`no_struct_identity_compare.rs`](src/rules/correctness/no_struct_identity_compare.rs) | `===` on structs (no identity) |
| `no-this-before-super` | ESLint | DIR | 🟡 | [`no_this_before_super.rs`](src/rules/correctness/no_this_before_super.rs) | `this` before `super()` call |
| `no-unsafe-finally` | ESLint | AST | 🟡 | [`no_unsafe_finally.rs`](src/rules/correctness/no_unsafe_finally.rs) | Control flow in `finally` |
| `no-unsafe-negation` | ESLint | AST | 🟡 | [`no_unsafe_negation.rs`](src/rules/correctness/no_unsafe_negation.rs) | `!a in b` vs `!(a in b)` |
| `no-unsafe-optional-chaining` | ESLint | DIR | 🟡 | [`no_unsafe_optional_chaining.rs`](src/rules/correctness/no_unsafe_optional_chaining.rs) | `?.` in unsafe contexts |
| `switch-exhaustiveness-check` | TS-ESLint | DIR | 🟡 | [`switch_exhaustiveness_check.rs`](src/rules/correctness/switch_exhaustiveness_check.rs) | Non-exhaustive switch |
| `unbound-method` | TS-ESLint | DIR | 🔶 | [`unbound_method.rs`](src/rules/correctness/unbound_method.rs) | Method used without binding |
| `use-isnan` | ESLint | AST | 🟡 | [`use_isnan.rs`](src/rules/correctness/use_isnan.rs) | Use `isNaN()` not `=== NaN` |
| `valid-typeof` | ESLint | AST | 🟡 | [`valid_typeof.rs`](src/rules/correctness/valid_typeof.rs) | Valid `typeof` comparison |

## Suspicious (U)

Code that is likely unintentional but may occasionally be intentional.

[`src/rules/suspicious/`](src/rules/suspicious/)

| Rule | Source | Level | Status | File | Notes |
|------|--------|-------|--------|------|-------|
| `no-cond-assign` | ESLint | AST | 🟡 | [`no_cond_assign.rs`](src/rules/suspicious/no_cond_assign.rs) | Assignment in condition |
| `no-confusing-non-null-assertion` | TS-ESLint | AST | 🟡 | [`no_confusing_non_null_assertion.rs`](src/rules/suspicious/no_confusing_non_null_assertion.rs) | `!` near `?` |
| `no-confusing-void-expression` | TS-ESLint | DIR | 🔶 | [`no_confusing_void_expression.rs`](src/rules/suspicious/no_confusing_void_expression.rs) | Void in expression position |
| `no-constructor-return` | ESLint | AST | 🟡 | [`no_constructor_return.rs`](src/rules/suspicious/no_constructor_return.rs) | Return in constructor |
| `no-debugger` | ESLint | AST | ✅ | [`no_debugger.rs`](src/rules/suspicious/no_debugger.rs) | Debugger statements |
| `no-dupe-else-if` | ESLint | AST | 🟡 | [`no_dupe_else_if.rs`](src/rules/suspicious/no_dupe_else_if.rs) | Duplicate else-if |
| `no-empty` | ESLint | AST | ✅ | [`no_empty.rs`](src/rules/suspicious/no_empty.rs) | Empty `{}` blocks |
| `no-empty-function` | ESLint | AST | 🟡 | [`no_empty_function.rs`](src/rules/suspicious/no_empty_function.rs) | Empty function body |
| `no-empty-match-arm` | Destack | AST | 🟡 | [`no_empty_match_arm.rs`](src/rules/suspicious/no_empty_match_arm.rs) | Empty match arm body |
| `no-empty-pattern` | ESLint | AST | 🟡 | [`no_empty_pattern.rs`](src/rules/suspicious/no_empty_pattern.rs) | Empty destructuring `{}` |
| `no-ex-assign` | ESLint | AST | 🟡 | [`no_ex_assign.rs`](src/rules/suspicious/no_ex_assign.rs) | Reassigning exception |
| `no-misleading-character-class` | ESLint | AST | 🟡 | [`no_misleading_character_class.rs`](src/rules/suspicious/no_misleading_character_class.rs) | Misleading regex chars |
| `no-negation-in-equality-check` | Unicorn | AST | 🟡 | [`no_negation_in_equality_check.rs`](src/rules/suspicious/no_negation_in_equality_check.rs) | `!a == b` confusion |
| `no-prototype-builtins` | ESLint | AST | 🟡 | [`no_prototype_builtins.rs`](src/rules/suspicious/no_prototype_builtins.rs) | Direct prototype methods |
| `no-redundant-pattern` | Destack | AST | 🟡 | [`no_redundant_pattern.rs`](src/rules/suspicious/no_redundant_pattern.rs) | Pattern binds nothing useful |
| `no-self-assign` | ESLint | AST | 🟡 | [`no_self_assign.rs`](src/rules/suspicious/no_self_assign.rs) | `x = x` |
| `no-template-curly-in-string` | ESLint | AST | 🟡 | [`no_template_curly_in_string.rs`](src/rules/suspicious/no_template_curly_in_string.rs) | `"${x}"` in regular string |
| `no-thenable` | Unicorn | DIR | 🔶 | [`no_thenable.rs`](src/rules/suspicious/no_thenable.rs) | Object with `.then()` |
| `no-unexpected-multiline` | ESLint | AST | 🟡 | [`no_unexpected_multiline.rs`](src/rules/suspicious/no_unexpected_multiline.rs) | Confusing line breaks |
| `no-unnecessary-type-assertion` | TS-ESLint | DIR | 🔶 | [`no_unnecessary_type_assertion.rs`](src/rules/suspicious/no_unnecessary_type_assertion.rs) | Redundant `as T` |
| `no-useless-backreference` | ESLint | AST | 🟡 | [`no_useless_backreference.rs`](src/rules/suspicious/no_useless_backreference.rs) | Invalid regex backrefs |
| `no-useless-catch` | ESLint | AST | 🟡 | [`no_useless_catch.rs`](src/rules/suspicious/no_useless_catch.rs) | Catch that just rethrows |
| `no-useless-computed-key` | ESLint | AST | 🟡 | [`no_useless_computed_key.rs`](src/rules/suspicious/no_useless_computed_key.rs) | `{["x"]: 1}` |
| `no-useless-concat` | ESLint | AST | 🟡 | [`no_useless_concat.rs`](src/rules/suspicious/no_useless_concat.rs) | `"a" + "b"` |
| `no-useless-constructor` | ESLint | AST | 🟡 | [`no_useless_constructor.rs`](src/rules/suspicious/no_useless_constructor.rs) | Empty constructor |
| `no-useless-escape` | ESLint | AST | 🟡 | [`no_useless_escape.rs`](src/rules/suspicious/no_useless_escape.rs) | Unnecessary escape chars |
| `no-useless-rename` | ESLint | AST | 🟡 | [`no_useless_rename.rs`](src/rules/suspicious/no_useless_rename.rs) | `{x: x}` in destructuring |
| `no-useless-return` | ESLint | AST | 🟡 | [`no_useless_return.rs`](src/rules/suspicious/no_useless_return.rs) | Return with no value at end |
| `prefer-match` | Destack | AST | 🟡 | [`prefer_match.rs`](src/rules/suspicious/prefer_match.rs) | Complex if-else → match |
| `require-yield` | ESLint | AST | 🟡 | [`require_yield.rs`](src/rules/suspicious/require_yield.rs) | Generator without yield |

## Security (S)

Patterns that may expose the application to attacks.

[`src/rules/security/`](src/rules/security/)

| Rule | Source | Level | Status | File | Notes |
|------|--------|-------|--------|------|-------|
| `no-eval` | ESLint | AST | 🟡 | [`no_eval.rs`](src/rules/security/no_eval.rs) | `eval()` usage |
| `no-implied-eval` | ESLint | DIR | 🟡 | [`no_implied_eval.rs`](src/rules/security/no_implied_eval.rs) | `setTimeout("code")` |
| `no-new-func` | ESLint | AST | 🟡 | [`no_new_func.rs`](src/rules/security/no_new_func.rs) | `new Function()` |
| `no-unsafe-argument` | TS-ESLint | DIR | 🔶 | [`no_unsafe_argument.rs`](src/rules/security/no_unsafe_argument.rs) | Passing `any` as argument |
| `no-unsafe-assignment` | TS-ESLint | DIR | 🔶 | [`no_unsafe_assignment.rs`](src/rules/security/no_unsafe_assignment.rs) | Assigning `any` |
| `no-unsafe-call` | TS-ESLint | DIR | 🔶 | [`no_unsafe_call.rs`](src/rules/security/no_unsafe_call.rs) | Calling `any` typed value |
| `no-unsafe-member-access` | TS-ESLint | DIR | 🔶 | [`no_unsafe_member_access.rs`](src/rules/security/no_unsafe_member_access.rs) | Accessing `any` member |
| `no-unsafe-return` | TS-ESLint | DIR | 🔶 | [`no_unsafe_return.rs`](src/rules/security/no_unsafe_return.rs) | Returning `any` |

## Performance (P)

Correct code that could be faster or use less memory.

[`src/rules/performance/`](src/rules/performance/)

| Rule | Source | Level | Status | File | Notes |
|------|--------|-------|--------|------|-------|
| `no-array-for-each` | Unicorn | AST | 🟡 | [`no_array_for_each.rs`](src/rules/performance/no_array_for_each.rs) | `forEach` → `for-of` |
| `no-await-in-loop` | ESLint | AST | 🟡 | [`no_await_in_loop.rs`](src/rules/performance/no_await_in_loop.rs) | Sequential awaits in loop |
| `prefer-for-of` | TS-ESLint | DIR | 🔶 | [`prefer_for_of.rs`](src/rules/performance/prefer_for_of.rs) | Index loop → for-of |
| `prefer-includes` | TS-ESLint | DIR | 🔶 | [`prefer_includes.rs`](src/rules/performance/prefer_includes.rs) | `.indexOf() !== -1` |
| `prefer-spread` | ESLint | AST | 🟡 | [`prefer_spread.rs`](src/rules/performance/prefer_spread.rs) | `.apply()` → spread |
| `require-array-sort-compare` | TS-ESLint | DIR | 🔶 | [`require_array_sort_compare.rs`](src/rules/performance/require_array_sort_compare.rs) | `.sort()` needs comparator |

## Style (Y)

Subjective preferences for consistent coding style.

[`src/rules/style/`](src/rules/style/)

| Rule | Source | Level | Status | File | Notes |
|------|--------|-------|--------|------|-------|
| `array-type` | TS-ESLint | AST | 🟡 | [`array_type.rs`](src/rules/style/array_type.rs) | `T[]` vs `Array<T>` |
| `catch-error-name` | Unicorn | AST | 🟡 | [`catch_error_name.rs`](src/rules/style/catch_error_name.rs) | Consistent error name |
| `consistent-extension-style` | Destack | AST | 🟡 | [`consistent_extension_style.rs`](src/rules/style/consistent_extension_style.rs) | Named vs anonymous extensions |
| `consistent-type-definitions` | TS-ESLint | AST | 🟡 | [`consistent_type_definitions.rs`](src/rules/style/consistent_type_definitions.rs) | `type` vs `interface` |
| `consistent-type-imports` | TS-ESLint | AST | 🟡 | [`consistent_type_imports.rs`](src/rules/style/consistent_type_imports.rs) | `import type` |
| `curly` | ESLint | AST | 🟡 | [`curly.rs`](src/rules/style/curly.rs) | Require braces |
| `dot-notation` | ESLint | AST | 🟡 | [`dot_notation.rs`](src/rules/style/dot_notation.rs) | `obj["x"]` → `obj.x` |
| `eqeqeq` | ESLint | AST | 🟡 | [`eqeqeq.rs`](src/rules/style/eqeqeq.rs) | `==` → `===` (Destack `==` is typed!) |
| `filename-case` | Unicorn | AST | 🟡 | [`filename_case.rs`](src/rules/style/filename_case.rs) | File naming convention |
| `no-else-return` | ESLint | AST | 🟡 | [`no_else_return.rs`](src/rules/style/no_else_return.rs) | Early return style |
| `no-lonely-if` | ESLint | AST | 🟡 | [`no_lonely_if.rs`](src/rules/style/no_lonely_if.rs) | Lonely `if` in `else` |
| `no-nested-ternary` | ESLint | AST | 🟡 | [`no_nested_ternary.rs`](src/rules/style/no_nested_ternary.rs) | Nested `?:` |
| `no-null` | Unicorn | AST | 🟡 | [`no_null.rs`](src/rules/style/no_null.rs) | `null` → `undefined` |
| `no-unneeded-ternary` | ESLint | AST | 🟡 | [`no_unneeded_ternary.rs`](src/rules/style/no_unneeded_ternary.rs) | `x ? true : false` |
| `no-var` | ESLint | AST | 🟡 | [`no_var.rs`](src/rules/style/no_var.rs) | `var` → `let`/`const` |
| `object-shorthand` | ESLint | AST | 🟡 | [`object_shorthand.rs`](src/rules/style/object_shorthand.rs) | `{x: x}` → `{x}` |
| `operator-assignment` | ESLint | AST | 🟡 | [`operator_assignment.rs`](src/rules/style/operator_assignment.rs) | `x = x + 1` → `x += 1` |
| `prefer-arrow-callback` | ESLint | AST | 🟡 | [`prefer_arrow_callback.rs`](src/rules/style/prefer_arrow_callback.rs) | Function → arrow |
| `prefer-as-const` | TS-ESLint | AST | 🟡 | [`prefer_as_const.rs`](src/rules/style/prefer_as_const.rs) | `as const` |
| `prefer-const` | ESLint | DIR | 🟡 | [`prefer_const.rs`](src/rules/style/prefer_const.rs) | `let` → `const` when never reassigned |
| `prefer-destructuring` | ESLint | DIR | 🟡 | [`prefer_destructuring.rs`](src/rules/style/prefer_destructuring.rs) | `x.y` → `const {y} = x` |
| `prefer-exponentiation-operator` | ESLint | AST | 🟡 | [`prefer_exponentiation_operator.rs`](src/rules/style/prefer_exponentiation_operator.rs) | `Math.pow` → `**` |
| `prefer-implicit-return` | Destack | AST | 🟡 | [`prefer_implicit_return.rs`](src/rules/style/prefer_implicit_return.rs) | `return x` → `x` |
| `prefer-newtype-over-alias` | Destack | DIR | 🟡 | [`prefer_newtype_over_alias.rs`](src/rules/style/prefer_newtype_over_alias.rs) | Type alias → newtype |
| `prefer-object-spread` | ESLint | AST | 🟡 | [`prefer_object_spread.rs`](src/rules/style/prefer_object_spread.rs) | `Object.assign` → `{...}` |
| `prefer-range-literal` | Destack | AST | 🟡 | [`prefer_range_literal.rs`](src/rules/style/prefer_range_literal.rs) | `for (let i=0; i<n; i++)` → `for i of 0..n` |
| `prefer-struct-over-class` | Destack | DIR | 🟡 | [`prefer_struct_over_class.rs`](src/rules/style/prefer_struct_over_class.rs) | Class with no identity → struct |
| `prefer-template` | ESLint | AST | 🟡 | [`prefer_template.rs`](src/rules/style/prefer_template.rs) | Concat → template literal |
| `prefer-tuple-destructuring` | Destack | AST | 🟡 | [`prefer_tuple_destructuring.rs`](src/rules/style/prefer_tuple_destructuring.rs) | `[a, b]` → `(a, b)` |

## Complexity (X)

Overly complex code that is harder to understand and maintain.

[`src/rules/complexity/`](src/rules/complexity/)

| Rule | Source | Level | Status | File | Notes |
|------|--------|-------|--------|------|-------|
| `max-depth` | ESLint | AST | 🟡 | [`max_depth.rs`](src/rules/complexity/max_depth.rs) | Maximum nesting depth |
| `max-lines` | ESLint | AST | 🟡 | [`max_lines.rs`](src/rules/complexity/max_lines.rs) | Maximum lines per file |
| `max-lines-per-function` | ESLint | AST | 🟡 | [`max_lines_per_function.rs`](src/rules/complexity/max_lines_per_function.rs) | Maximum lines per function |
| `max-nested-callbacks` | ESLint | AST | 🟡 | [`max_nested_callbacks.rs`](src/rules/complexity/max_nested_callbacks.rs) | Maximum callback nesting |
| `max-params` | ESLint | AST | 🟡 | [`max_params.rs`](src/rules/complexity/max_params.rs) | Maximum function parameters |
| `max-statements` | ESLint | AST | 🟡 | [`max_statements.rs`](src/rules/complexity/max_statements.rs) | Maximum statements per function |
| `no-multi-assign` | ESLint | AST | 🟡 | [`no_multi_assign.rs`](src/rules/complexity/no_multi_assign.rs) | `a = b = c` chains |

## Restriction (R)

Opt-in rules that ban certain patterns by project choice.

[`src/rules/restriction/`](src/rules/restriction/)

| Rule | Source | Level | Status | File | Notes |
|------|--------|-------|--------|------|-------|
| `no-alert` | ESLint | AST | 🟡 | [`no_alert.rs`](src/rules/restriction/no_alert.rs) | `alert()`, `confirm()`, `prompt()` |
| `no-anonymous-default-export` | Unicorn | AST | 🟡 | [`no_anonymous_default_export.rs`](src/rules/restriction/no_anonymous_default_export.rs) | Named exports only |
| `no-any` | Destack | AST | 🟡 | [`no_any.rs`](src/rules/restriction/no_any.rs) | Ban `any` entirely |
| `no-bitwise` | ESLint | AST | 🟡 | [`no_bitwise.rs`](src/rules/restriction/no_bitwise.rs) | Bitwise operators |
| `no-class` | Destack | AST | 🟡 | [`no_class.rs`](src/rules/restriction/no_class.rs) | Prefer struct over class |
| `no-console` | ESLint | AST | 🟡 | [`no_console.rs`](src/rules/restriction/no_console.rs) | `console.*` statements |
| `no-continue` | ESLint | AST | 🟡 | [`no_continue.rs`](src/rules/restriction/no_continue.rs) | `continue` statement |
| `no-explicit-any` | TS-ESLint | AST | 🟡 | [`no_explicit_any.rs`](src/rules/restriction/no_explicit_any.rs) | `any` type annotations |
| `no-implicit-return` | Destack | AST | 🟡 | [`no_implicit_return.rs`](src/rules/restriction/no_implicit_return.rs) | Require explicit `return` |
| `no-labels` | ESLint | AST | 🟡 | [`no_labels.rs`](src/rules/restriction/no_labels.rs) | Labeled statements |
| `no-loop-keyword` | Destack | AST | 🟡 | [`no_loop_keyword.rs`](src/rules/restriction/no_loop_keyword.rs) | Prefer `while(true)` over `loop` |
| `no-magic-numbers` | ESLint | AST | 🟡 | [`no_magic_numbers.rs`](src/rules/restriction/no_magic_numbers.rs) | Unnamed numeric literals |
| `no-namespace` | TS-ESLint | AST | 🟡 | [`no_namespace.rs`](src/rules/restriction/no_namespace.rs) | `namespace` keyword |
| `no-non-null-assertion` | TS-ESLint | AST | 🟡 | [`no_non_null_assertion.rs`](src/rules/restriction/no_non_null_assertion.rs) | `!` assertion operator |
| `no-plusplus` | ESLint | AST | 🟡 | [`no_plusplus.rs`](src/rules/restriction/no_plusplus.rs) | `++` and `--` |
| `no-process-exit` | Unicorn | AST | 🟡 | [`no_process_exit.rs`](src/rules/restriction/no_process_exit.rs) | `process.exit()` |
| `no-require-imports` | TS-ESLint | AST | 🟡 | [`no_require_imports.rs`](src/rules/restriction/no_require_imports.rs) | CommonJS `require()` |
| `no-restricted-globals` | ESLint | AST | 🟡 | [`no_restricted_globals.rs`](src/rules/restriction/no_restricted_globals.rs) | Banned globals |
| `no-restricted-imports` | ESLint | AST | 🟡 | [`no_restricted_imports.rs`](src/rules/restriction/no_restricted_imports.rs) | Banned imports |
| `no-restricted-syntax` | ESLint | AST | 🟡 | [`no_restricted_syntax.rs`](src/rules/restriction/no_restricted_syntax.rs) | Banned AST patterns |
| `no-ternary` | ESLint | AST | 🟡 | [`no_ternary.rs`](src/rules/restriction/no_ternary.rs) | Ternary operator |
| `no-void` | ESLint | AST | 🟡 | [`no_void.rs`](src/rules/restriction/no_void.rs) | `void` operator |
| `no-warning-comments` | ESLint | AST | 🟡 | [`no_warning_comments.rs`](src/rules/restriction/no_warning_comments.rs) | `TODO`, `FIXME` etc |
| `no-with` | ESLint | AST | 🟡 | [`no_with.rs`](src/rules/restriction/no_with.rs) | `with` statement |
| `strict-boolean-expressions` | TS-ESLint | DIR | 🔶 | [`strict_boolean_expressions.rs`](src/rules/restriction/strict_boolean_expressions.rs) | No truthy/falsy |

## Pedantic (D)

Very strict or opinionated checks that may be too noisy.

[`src/rules/pedantic/`](src/rules/pedantic/)

| Rule | Source | Level | Status | File | Notes |
|------|--------|-------|--------|------|-------|
| `consistent-return` | ESLint | DIR | 🟡 | [`consistent_return.rs`](src/rules/pedantic/consistent_return.rs) | Consistent return values |
| `explicit-function-return-type` | TS-ESLint | AST | 🟡 | [`explicit_function_return_type.rs`](src/rules/pedantic/explicit_function_return_type.rs) | Explicit return types |
| `explicit-length-check` | Unicorn | AST | 🟡 | [`explicit_length_check.rs`](src/rules/pedantic/explicit_length_check.rs) | `.length > 0` not `.length` |
| `guard-for-in` | ESLint | AST | 🟡 | [`guard_for_in.rs`](src/rules/pedantic/guard_for_in.rs) | `hasOwnProperty` in for-in |
| `no-caller` | ESLint | AST | 🟡 | [`no_caller.rs`](src/rules/pedantic/no_caller.rs) | `arguments.caller` |
| `no-extend-native` | ESLint | AST | 🟡 | [`no_extend_native.rs`](src/rules/pedantic/no_extend_native.rs) | Don't extend native prototypes |
| `no-inferrable-types` | TS-ESLint | AST | 🟡 | [`no_inferrable_types.rs`](src/rules/pedantic/no_inferrable_types.rs) | Remove obvious types |
| `no-iterator` | ESLint | AST | 🟡 | [`no_iterator.rs`](src/rules/pedantic/no_iterator.rs) | `__iterator__` property |
| `no-param-reassign` | ESLint | DIR | 🟡 | [`no_param_reassign.rs`](src/rules/pedantic/no_param_reassign.rs) | Reassigning parameters |
| `no-shadow` | ESLint | DIR | 🟡 | [`no_shadow.rs`](src/rules/pedantic/no_shadow.rs) | Variable shadowing |
| `no-unreadable-array-destructuring` | Unicorn | AST | 🟡 | [`no_unreadable_array_destructuring.rs`](src/rules/pedantic/no_unreadable_array_destructuring.rs) | Too many holes `[,,,x]` |
| `no-use-before-define` | ESLint | DIR | 🟡 | [`no_use_before_define.rs`](src/rules/pedantic/no_use_before_define.rs) | Use before declaration |
| `prefer-nullish-coalescing` | TS-ESLint | DIR | 🔶 | [`prefer_nullish_coalescing.rs`](src/rules/pedantic/prefer_nullish_coalescing.rs) | `??` vs `\|\|` |
| `radix` | ESLint | AST | 🟡 | [`radix.rs`](src/rules/pedantic/radix.rs) | `parseInt` radix parameter |

---

## Implementation Notes

### Naming Convention

Rule identifiers use the **original names from their source** where possible:
- ESLint: `no-debugger`, `no-eval`, `prefer-const`
- TS-ESLint: `no-floating-promises`, `await-thenable`
- Unicorn: `no-thenable`, `catch-error-name`
- Destack: `no-struct-identity-compare`, `prefer-range-literal`

File names use snake_case: `no-debugger` → `no_debugger.rs`

### Rules NOT to Implement

These are handled by the compiler or formatter:

| Rule | Reason |
|------|--------|
| `no-undef` | Compiler error (resolve phase) |
| `no-unused-vars` | Compiler warning (analyze phase) |
| `no-redeclare` | Compiler error (Destack allows shadowing) |
| Match exhaustiveness | Compiler error for `match` |
| All layout/formatting | Handled by formatter |

### IR Requirements by Rule Type

| Type | IR Level | Example |
|------|----------|---------|
| Syntax patterns | AST | `no-debugger`, `no-eval` |
| Symbol tracking | DIR | `prefer-const`, `no-shadow` |
| Type information | DIR | `no-floating-promises`, `await-thenable` |
| Control flow | MIR | `no-unreachable`, `cyclomatic-complexity` |
| Ownership/borrowing | MIR | `no-copy-in-loop`, `prefer-reference` |
