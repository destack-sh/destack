# Formatter Conformance Sources

This file defines the source priority for external formatter corpus imports.

## Priority

1. Prettier fixtures for broad ecosystem syntax and formatting behavior coverage.
2. Oxfmt fixtures for focused style and output parity coverage.

## Why this order

Prettier provides the broadest long tail of real world formatting edge cases.

Oxfmt is close to our style direction, so it gives us a second external oracle for output parity on shared syntax.

## Policy

Destack formatter remains authoritative for TS++ and annotation specific behavior.

External suites are strict conformance targets for supported syntax.

Each imported case should map to one of these modes.

1. `expected-match`: compare output to imported expected output.
2. `idempotence-only`: no expected output, only require stable formatter output.

Start imports with `idempotence-only`, then promote stable subsets to `expected-match`.

## Script refs

Each fetch script supports an override ref environment variable:

1. `PRETTIER_REF=... ./prettier-fetch.sh`
2. `OXFMT_REF=... ./oxfmt-fetch.sh`
