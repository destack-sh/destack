# Parser Fuzzing

Parser fuzzing uses libFuzzer against arbitrary source text.
Crash repros should become permanent parser or formatter tests before artifacts are discarded.

## Targets

The parser fuzz package has these targets:

| Target | Purpose |
|--------|---------|
| `lexer` | Lex arbitrary UTF-8 as `.ds` |
| `parser` | Decode arbitrary bytes and parse them as `.ds` and `.d.ds` |

## Usage

Run these from `language/`.

```bash
just fuzz-lexer
just fuzz-parser
just fuzz-list
```
