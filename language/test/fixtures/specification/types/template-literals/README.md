# Template Literal Types

Tests for template literal matching, assignability, inference, and cross-feature interactions.

## Coverage

- Literal matching against string, boolean, number, bigint, and int spans.
- Assignability between template literal types.
- Conditional inference and generic inference from template literals.
- Numeric span grammar validation.
- Interaction with mapped keys, indexed access, and `satisfies`.
- Interaction with associated type projections and owner substitutions.
- Cross-module resolution through re-exports and namespace imports.
- Flow behavior across null checks, equality checks, and match join commit rules.
- Inference scenarios are split into basic, argument, and numeric cases.
