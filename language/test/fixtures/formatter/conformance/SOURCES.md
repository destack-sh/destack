# Formatter Conformance Sources

This file defines the source priority for external formatter corpus imports.

## Priority

1. Biome fixtures for high signal JS, TS, and TSX parser and formatter edge cases.
2. Prettier fixtures for broad ecosystem syntax and formatting behavior coverage.
3. Oxfmt fixtures for targeted style parity checks where we intentionally align.

## Why this order

Biome provides strong modern TypeScript and TSX case quality with predictable fixture structure.

Prettier provides the broadest long tail of real world formatting edge cases.

Oxfmt is close to our style direction, so we use it for focused style parity checks instead of full lockstep parity.

## Policy

Destack formatter remains authoritative for TS++ and annotation specific behavior.

External suites are compatibility signals, not strict style mandates.

Each imported case should map to one of these modes.

1. `expected-match`: compare output to imported expected output.
2. `idempotence-only`: no expected output, only require stable formatter output.

Start imports with `idempotence-only`, then promote stable subsets to `expected-match`.

## Script refs

Each fetch script supports an override ref environment variable:

1. `BIOME_REF=... ./biome-fetch.sh`
2. `PRETTIER_REF=... ./prettier-fetch.sh`
3. `OXFMT_REF=... ./oxfmt-fetch.sh`
