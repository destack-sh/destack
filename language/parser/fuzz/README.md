# Parser Fuzzing

Parser fuzzing uses libFuzzer against arbitrary source text and generated stress cases.
Crash repros should become permanent parser or formatter tests before artifacts are discarded.

## Targets

The parser fuzz package has these targets:

| Target | Purpose |
|--------|---------|
| `lexer` | Lex arbitrary UTF-8 as `.ds` |
| `parser` | Parse arbitrary UTF-8 and generated stress cases across `.ds`, `.d.ds`, `.ts`, `.tsx`, and `.d.ts` |

## Usage

Run these from `language/`.

```bash
just fuzz-lexer
just fuzz-parser
just fuzz-list
```
