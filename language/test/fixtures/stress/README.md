# Stress Tests

Stress tests exercise large, weird, and damaged parser/formatter inputs.
They are replayable robustness tests, not semantic specification fixtures.
The suite follows the same broad shape as compiler and parser torture suites: generated pathological corpora, isolated per-file execution, and fuzz seeds from the same grammar-shaped generators.

Stress fixtures are generated under `parser/generated/` and `formatter/generated/`.
The generated files are ignored by git so they can be inspected locally without becoming source fixtures.
Each stress case writes a topical folder with several size and shape variants.
Large cases include multi-megabyte files, extreme nesting, very wide lists, trivia floods, and repeated damaged syntax with recovery sentinels.
Each stress fixture runs in a child process so stack overflows and aborts are reported as case failures instead of killing the harness.
Stress output includes byte, line, wall-time, MB/s, and lines/s throughput for each worker.

## Matrix

The corpus is organized around grammar pressure points rather than source examples.

| Axis | Coverage |
| --- | --- |
| Source kind | `.ds`, `.d.ds`, `.ts`, `.tsx`, `.d.ts` where the syntax family applies |
| Scale | large, huge, massive, wide, dense, pathological, deep, and damaged variants |
| Shape | long files, wide lists, deep nesting, dense trivia, ambiguous prefixes, and damaged delimiters |
| Syntax | declarations, signatures, classes, interfaces, types, expressions, patterns, TSX, modules, decorators, comptime, memory, ranges, sequences, errors, and resource management |
| Recovery | damaged declarations, expressions, types, TSX, trivia, and delimiter storms with later recovery sentinels |
| Checks | clean parse, recovery diagnostics, recovery roots, formatter parseability, formatter idempotence, and throughput |

## Invariants

Parser stress checks these invariants:

- valid generated inputs parse without parser errors
- damaged generated inputs produce parser errors
- damaged generated inputs recover far enough to keep later roots

Formatter stress checks these invariants:

- valid generated inputs format without parser errors
- formatted output parses cleanly
- formatting is idempotent
- default and narrow line widths both stay stable

## Usage

Run these from `language/`.

```bash
just generate-stress
just test-stress
just test-stress-parser
just test-stress-formatter
```
