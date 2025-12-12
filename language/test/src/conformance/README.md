# Conformance Tests

Conformance tests check that the Destack parser conforms both to the ECMAScript specification and various other established "real-world" test suites.

## Why Not 100%?

We do not expect to reach 100% conformance because:

1. **Destack is TSX**: `.ds` files are more like `.tsx` than `.ts` or `.js`. Just like `.tsx` is not 100% compatible with `.ts`, `.ds` is not 100% compatible with `.ts` or `.js`.

2. **Destack is TypeScript++**: `.ds` files override some obscure TypeScript syntax patterns with more useful features (like tuples with `()` instead of sequence operators).

## Current Results

| Suite    | Passed | Failed | Total |  Rate   |
|:---------|-------:|-------:|------:|--------:|
| test262  |  4519  |   844  |  5363 |  84.3%  |
| babel    |   511  |   209  |   720 |  71.0%  |
| swc      |   522  |   163  |   685 |  76.2%  |
| biome    |   409  |   228  |   637 |  64.2%  |

## Notes

- **Annex B**: The test262-parser-tests suite we use does not include Annex B tests. We intentionally don't support Annex B (legacy web compatibility features like HTML comments in scripts). This is the same approach SWC takes.
- **Flow**: Intentionally excluded. We support TypeScript only.
- **Real-world priority**: The babel/swc/biome suites test patterns users actually write. Improving these rates matters more than edge-case test262 compliance.