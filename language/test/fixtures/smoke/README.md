# Smoke Test Fixtures

Smoke tests are tests that just need to run without errors (for some definition of "run" and "without errors").
For example, parser tests need to parse without errors, and compiler tests need to compile without errors.

## Layout

```
smoke/
├── parser/        # parser inputs that must parse cleanly
└── compiler/      # compiler inputs that must compile cleanly
```

Files are named `parser-####.<ext>` or `compiler-####.<ext>` and use `.ds`, `.ts`, `.tsx`, or `.js` as needed.

## Running

```bash
just test-smoke
```
